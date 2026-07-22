use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use futures_util::task::AtomicWaker;
use http_body_util::BodyExt;
use hyper::Version;
use hyper::body::{Body, Frame, SizeHint};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::{Instant, Sleep, sleep_until},
};

use crate::{metrics::ServerMetrics, response::RequestBody};

#[derive(Debug, Default)]
pub(crate) struct ConnectionActivity {
    active_requests: AtomicUsize,
    idle_waker: AtomicWaker,
}

impl ConnectionActivity {
    pub(crate) fn enter(self: &Arc<Self>) -> ConnectionRequestGuard {
        self.active_requests.fetch_add(1, Ordering::AcqRel);
        ConnectionRequestGuard {
            activity: Arc::clone(self),
        }
    }

    fn has_active_requests(&self) -> bool {
        self.active_requests.load(Ordering::Acquire) != 0
    }
}

#[derive(Debug)]
pub(crate) struct ConnectionRequestGuard {
    activity: Arc<ConnectionActivity>,
}

#[derive(Debug)]
pub(crate) struct ConnectionProtocolTracker {
    protocol: AtomicUsize,
    metrics: Arc<ServerMetrics>,
}

impl ConnectionProtocolTracker {
    pub(crate) fn new(metrics: Arc<ServerMetrics>) -> Self {
        Self {
            protocol: AtomicUsize::new(0),
            metrics,
        }
    }

    pub(crate) fn observe(&self, version: Version) {
        let protocol = match version {
            Version::HTTP_10 | Version::HTTP_11 => 1,
            Version::HTTP_2 => 2,
            _ => return,
        };
        if self
            .protocol
            .compare_exchange(0, protocol, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            match protocol {
                1 => &self.metrics.h1_connections_active,
                _ => &self.metrics.h2_connections_active,
            }
            .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn observed_protocol(&self) -> usize {
        self.protocol.load(Ordering::Acquire)
    }
}

impl Drop for ConnectionProtocolTracker {
    fn drop(&mut self) {
        match self.protocol.load(Ordering::Acquire) {
            1 => &self.metrics.h1_connections_active,
            2 => &self.metrics.h2_connections_active,
            _ => return,
        }
        .fetch_sub(1, Ordering::Relaxed);
    }
}

impl Drop for ConnectionRequestGuard {
    fn drop(&mut self) {
        self.activity.active_requests.fetch_sub(1, Ordering::AcqRel);
        self.activity.idle_waker.wake();
    }
}

pub(crate) struct TransportIo<S> {
    inner: S,
    activity: Arc<ConnectionActivity>,
    metrics: Arc<ServerMetrics>,
    connection_idle_timeout: Duration,
    response_write_idle_timeout: Duration,
    connection_idle: Pin<Box<Sleep>>,
    write_idle: Pin<Box<Sleep>>,
    connection_idle_armed: bool,
    write_idle_armed: bool,
    connection_timeout_counted: bool,
    write_timeout_counted: bool,
}

impl<S> TransportIo<S> {
    pub(crate) fn new(
        inner: S,
        activity: Arc<ConnectionActivity>,
        metrics: Arc<ServerMetrics>,
        connection_idle_timeout: Duration,
        response_write_idle_timeout: Duration,
    ) -> Self {
        let far_future = Instant::now() + Duration::from_secs(365 * 24 * 60 * 60);
        Self {
            inner,
            activity,
            metrics,
            connection_idle_timeout,
            response_write_idle_timeout,
            connection_idle: Box::pin(sleep_until(far_future)),
            write_idle: Box::pin(sleep_until(far_future)),
            connection_idle_armed: false,
            write_idle_armed: false,
            connection_timeout_counted: false,
            write_timeout_counted: false,
        }
    }

    fn read_pending_timeout(&mut self, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.activity.idle_waker.register(context.waker());
        if self.activity.has_active_requests() {
            self.connection_idle_armed = false;
            return Poll::Pending;
        }
        if !self.connection_idle_armed {
            self.connection_idle
                .as_mut()
                .reset(Instant::now() + self.connection_idle_timeout);
            self.connection_idle_armed = true;
        }
        if self.connection_idle.as_mut().poll(context).is_pending() {
            return Poll::Pending;
        }
        if !self.connection_timeout_counted {
            self.metrics
                .connection_idle_timeouts_total
                .fetch_add(1, Ordering::Relaxed);
            self.connection_timeout_counted = true;
        }
        Poll::Ready(Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "connection idle timeout",
        )))
    }

    fn write_pending_timeout(&mut self, context: &mut Context<'_>) -> Poll<io::Result<usize>> {
        if !self.write_idle_armed {
            self.write_idle
                .as_mut()
                .reset(Instant::now() + self.response_write_idle_timeout);
            self.write_idle_armed = true;
        }
        if self.write_idle.as_mut().poll(context).is_pending() {
            return Poll::Pending;
        }
        if !self.write_timeout_counted {
            self.metrics
                .response_write_idle_timeouts_total
                .fetch_add(1, Ordering::Relaxed);
            self.write_timeout_counted = true;
        }
        Poll::Ready(Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "response write idle timeout",
        )))
    }

    fn wrote_progress(&mut self) {
        self.write_idle_armed = false;
        self.connection_idle_armed = false;
    }

    fn flushed(&mut self) {
        // A transport may report an empty flush as ready on every connection
        // poll. Writes already record actual progress; treating a no-op flush
        // as connection activity would postpone keep-alive idle forever.
        self.write_idle_armed = false;
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for TransportIo<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buffer.filled().len();
        match Pin::new(&mut this.inner).poll_read(context, buffer) {
            Poll::Ready(Ok(())) => {
                if buffer.filled().len() > before {
                    this.connection_idle_armed = false;
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.read_pending_timeout(context),
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for TransportIo<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(context, buffer) {
            Poll::Ready(Ok(written)) => {
                if written != 0 {
                    this.wrote_progress();
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.write_pending_timeout(context),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_flush(context) {
            Poll::Ready(Ok(())) => {
                this.flushed();
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => match this.write_pending_timeout(context) {
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Ready(Ok(_)) | Poll::Pending => Poll::Pending,
            },
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(context)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffers: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write_vectored(context, buffers) {
            Poll::Ready(Ok(written)) => {
                if written != 0 {
                    this.wrote_progress();
                }
                Poll::Ready(Ok(written))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.write_pending_timeout(context),
        }
    }
}

struct IdleRequestBody {
    inner: RequestBody,
    timeout: Duration,
    timer: Pin<Box<Sleep>>,
    metrics: Arc<ServerMetrics>,
    armed: bool,
    complete: bool,
    counted: bool,
}

impl Body for IdleRequestBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if this.complete {
            return Poll::Ready(None);
        }
        match Pin::new(&mut this.inner).poll_frame(context) {
            Poll::Ready(frame) => {
                this.armed = false;
                if frame.is_none() {
                    this.complete = true;
                }
                Poll::Ready(frame)
            }
            Poll::Pending => {
                if !this.armed {
                    this.timer.as_mut().reset(Instant::now() + this.timeout);
                    this.armed = true;
                }
                if this.timer.as_mut().poll(context).is_pending() {
                    return Poll::Pending;
                }
                this.complete = true;
                if !this.counted {
                    this.metrics
                        .request_body_idle_timeouts_total
                        .fetch_add(1, Ordering::Relaxed);
                    this.counted = true;
                }
                Poll::Ready(Some(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "request body idle timeout",
                ))))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.complete || self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

pub(crate) fn idle_request_body(
    body: RequestBody,
    timeout: Duration,
    metrics: Arc<ServerMetrics>,
) -> RequestBody {
    let far_future = Instant::now() + Duration::from_secs(365 * 24 * 60 * 60);
    IdleRequestBody {
        inner: body,
        timeout,
        timer: Box::pin(sleep_until(far_future)),
        metrics,
        armed: false,
        complete: false,
        counted: false,
    }
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{StreamExt, stream};
    use http_body_util::StreamBody;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn connection_idle_waits_until_the_active_request_finishes() {
        let metrics = Arc::new(ServerMetrics::default());
        let activity = Arc::new(ConnectionActivity::default());
        let guard = activity.enter();
        let (server, _client) = tokio::io::duplex(64);
        let mut io = TransportIo::new(
            server,
            Arc::clone(&activity),
            Arc::clone(&metrics),
            Duration::from_millis(20),
            Duration::from_secs(1),
        );
        let mut byte = [0_u8; 1];

        assert!(
            tokio::time::timeout(Duration::from_millis(60), io.read(&mut byte))
                .await
                .is_err(),
            "connection idle fired while a request guard was active"
        );
        drop(guard);
        let error = io
            .read(&mut byte)
            .await
            .expect_err("idle read must time out");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            metrics
                .connection_idle_timeouts_total
                .load(Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn stalled_write_times_out_and_is_counted_once() {
        let metrics = Arc::new(ServerMetrics::default());
        let activity = Arc::new(ConnectionActivity::default());
        let (server, _client) = tokio::io::duplex(1);
        let mut io = TransportIo::new(
            server,
            activity,
            Arc::clone(&metrics),
            Duration::from_secs(1),
            Duration::from_millis(20),
        );

        let error = io
            .write_all(&[0_u8; 1024])
            .await
            .expect_err("blocked write must time out");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            metrics
                .response_write_idle_timeouts_total
                .load(Ordering::Relaxed),
            1
        );
    }

    #[tokio::test]
    async fn request_body_idle_is_distinct_and_terminates_after_one_error() {
        let frames =
            stream::once(async { Ok::<_, io::Error>(Frame::data(Bytes::from_static(b"a"))) })
                .chain(stream::pending());
        let body = http_body_util::BodyExt::boxed(StreamBody::new(frames));
        let metrics = Arc::new(ServerMetrics::default());
        let mut body = idle_request_body(body, Duration::from_millis(20), Arc::clone(&metrics));

        assert_eq!(
            body.frame()
                .await
                .expect("first frame")
                .expect("first frame succeeds")
                .into_data()
                .expect("data frame"),
            Bytes::from_static(b"a")
        );
        let error = body
            .frame()
            .await
            .expect("timeout frame")
            .expect_err("idle body must error");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(body.frame().await.is_none());
        assert_eq!(
            metrics
                .request_body_idle_timeouts_total
                .load(Ordering::Relaxed),
            1
        );
    }
}
