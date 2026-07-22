use bytes::Bytes;
use futures_util::{TryStreamExt, stream};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::{
    Method, Response, StatusCode,
    body::{Body, Frame, SizeHint},
    header,
};
use std::{
    convert::Infallible,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

use crate::transport::ConnectionRequestGuard;
use crate::{
    metrics::ServerMetrics,
    request_metadata::HttpProtocol,
    shutdown::ShutdownPhase,
    transfer::{TransferLifecycle, TransferOutcome},
};
use php_runtime::api::RuntimeCancellationState;

pub type ResponseBody = ServerBody;
pub type RequestBody = BoxBody<Bytes, std::io::Error>;

pub(crate) fn request_reader_body<R>(reader: R) -> RequestBody
where
    R: AsyncRead + Send + Sync + 'static,
{
    let stream = ReaderStream::new(reader).map_ok(Frame::data);
    StreamBody::new(stream).boxed()
}

pub struct ServerBody {
    inner: BoxBody<Bytes, std::io::Error>,
    lifecycle: Option<TransferLifecycle>,
    emitted_bytes: u64,
    finished: bool,
    defer_completion: bool,
    reached_end: bool,
    cancellation: Option<RuntimeCancellationState>,
    expected_bytes: Option<u64>,
    connection_request: Option<ConnectionRequestGuard>,
}

impl ServerBody {
    fn new(inner: BoxBody<Bytes, std::io::Error>) -> Self {
        Self {
            inner,
            lifecycle: None,
            emitted_bytes: 0,
            finished: false,
            defer_completion: false,
            reached_end: false,
            cancellation: None,
            expected_bytes: None,
            connection_request: None,
        }
    }

    pub(crate) fn attach_lifecycle(&mut self, lifecycle: TransferLifecycle) {
        assert!(
            self.lifecycle.is_none(),
            "response body lifecycle attached twice"
        );
        self.lifecycle = Some(lifecycle);
        if self.inner.is_end_stream() {
            self.finish(TransferOutcome::Completed);
        }
    }

    pub(crate) fn attach_connection_request(&mut self, guard: ConnectionRequestGuard) {
        assert!(
            self.connection_request.is_none(),
            "connection request guard attached twice"
        );
        if self.finished || self.reached_end || self.inner.is_end_stream() {
            drop(guard);
        } else {
            self.connection_request = Some(guard);
        }
    }

    pub(crate) fn abort(&mut self) {
        self.finish(TransferOutcome::Aborted);
    }

    pub(crate) fn error(&mut self) {
        self.finish(TransferOutcome::Error);
    }

    pub(crate) fn defer_completion(&mut self) {
        self.defer_completion = true;
    }

    pub(crate) fn attach_cancellation(&mut self, cancellation: RuntimeCancellationState) {
        self.cancellation = Some(cancellation);
    }

    pub(crate) fn set_expected_bytes(&mut self, expected_bytes: u64) {
        self.expected_bytes = Some(expected_bytes);
    }

    fn suppress(&mut self) {
        self.inner = Full::new(Bytes::new())
            .map_err(|never: Infallible| match never {})
            .boxed();
        self.reached_end = false;
        self.expected_bytes = Some(0);
    }

    pub(crate) fn complete(&mut self) {
        self.finish(TransferOutcome::Completed);
    }

    fn finish(&mut self, outcome: TransferOutcome) {
        if self.finished {
            return;
        }
        if outcome != TransferOutcome::Completed
            && let Some(cancellation) = &self.cancellation
        {
            cancellation.cancel();
        }
        self.finished = true;
        self.connection_request.take();
        if let Some(lifecycle) = self.lifecycle.take() {
            lifecycle.finish_transfer(outcome, self.emitted_bytes);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ResponseFinalized;

pub(crate) fn finalize_response(
    method: &Method,
    protocol: HttpProtocol,
    shutdown_phase: ShutdownPhase,
    force_connection_close: bool,
    response: &mut Response<ResponseBody>,
    metrics: &ServerMetrics,
) {
    assert!(
        response.extensions().get::<ResponseFinalized>().is_none(),
        "response finalized twice"
    );
    response.extensions_mut().insert(ResponseFinalized);

    let mut nominated = Vec::new();
    for value in response.headers().get_all(header::CONNECTION) {
        let Ok(value) = value.to_str() else {
            metrics
                .response_invalid_connection_tokens_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            continue;
        };
        for token in value.split(',').map(str::trim) {
            if token.is_empty() {
                metrics
                    .response_invalid_connection_tokens_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                continue;
            }
            if let Ok(name) = hyper::header::HeaderName::from_bytes(token.as_bytes()) {
                nominated.push(name);
            } else {
                metrics
                    .response_invalid_connection_tokens_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    for name in nominated {
        if response.headers_mut().remove(name).is_some() {
            metrics
                .response_connection_nominated_headers_removed_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    for name in [
        header::CONNECTION,
        hyper::header::HeaderName::from_static("keep-alive"),
        header::TRANSFER_ENCODING,
        header::UPGRADE,
        hyper::header::HeaderName::from_static("proxy-connection"),
        header::TRAILER,
    ] {
        if response.headers_mut().remove(name).is_some() {
            metrics
                .response_hop_by_hop_headers_removed_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    let status = response.status();
    if method != Method::HEAD && status != StatusCode::NOT_MODIFIED {
        let invalid_content_length =
            response
                .headers()
                .get(header::CONTENT_LENGTH)
                .is_some_and(|value| {
                    let Ok(value) = value.to_str() else {
                        return true;
                    };
                    let Ok(value) = value.parse::<u64>() else {
                        return true;
                    };
                    response
                        .body()
                        .size_hint()
                        .exact()
                        .is_some_and(|exact| exact != value)
                });
        if invalid_content_length {
            response.headers_mut().remove(header::CONTENT_LENGTH);
        }
    }
    let suppress = method == Method::HEAD
        || status.is_informational()
        || matches!(
            status,
            StatusCode::NO_CONTENT | StatusCode::RESET_CONTENT | StatusCode::NOT_MODIFIED
        );
    if suppress {
        response.body_mut().suppress();
        metrics
            .response_body_suppressed_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if status.is_informational() || status == StatusCode::NO_CONTENT {
        response.headers_mut().remove(header::CONTENT_LENGTH);
    } else if status == StatusCode::RESET_CONTENT {
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            hyper::header::HeaderValue::from_static("0"),
        );
    }
    if status == StatusCode::SWITCHING_PROTOCOLS {
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response.headers_mut().remove(header::CONTENT_LENGTH);
        response.body_mut().suppress();
        metrics
            .response_invalid_upgrade_status_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    if protocol.is_h1() && (force_connection_close || shutdown_phase != ShutdownPhase::Running) {
        response.headers_mut().insert(
            header::CONNECTION,
            hyper::header::HeaderValue::from_static("close"),
        );
    }
}

impl Body for ServerBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match Pin::new(&mut self.inner).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    self.emitted_bytes = self.emitted_bytes.saturating_add(data.len() as u64);
                }
                if !self.defer_completion
                    && self
                        .expected_bytes
                        .is_some_and(|expected| self.emitted_bytes >= expected)
                {
                    self.finish(TransferOutcome::Completed);
                }
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => {
                self.finish(TransferOutcome::Error);
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                self.reached_end = true;
                if !self.defer_completion {
                    self.finish(TransferOutcome::Completed);
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.reached_end || self.finished || self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Drop for ServerBody {
    fn drop(&mut self) {
        if !self.finished {
            self.finish(TransferOutcome::Aborted);
        }
    }
}

pub fn text(status: StatusCode, body: &'static str) -> Response<ResponseBody> {
    response(status, Bytes::from_static(body.as_bytes()))
}

pub fn text_dynamic(
    status: StatusCode,
    body: String,
    content_type: &'static str,
) -> Response<ResponseBody> {
    bytes(status, Bytes::from(body), content_type)
}

pub fn empty(status: StatusCode) -> Response<ResponseBody> {
    response(status, Bytes::new())
}

pub fn bytes(
    status: StatusCode,
    body: Bytes,
    content_type: &'static str,
) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_LENGTH, body.len().to_string())
        .header(header::CONTENT_TYPE, content_type)
        .body(full_body(body))
        .expect("static response builder is valid")
}

pub fn static_head(
    status: StatusCode,
    content_length: u64,
    content_type: &'static str,
) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_LENGTH, content_length.to_string())
        .header(header::CONTENT_TYPE, content_type)
        .body(full_body(Bytes::new()))
        .expect("static response builder is valid")
}

pub fn response(status: StatusCode, body: Bytes) -> Response<ResponseBody> {
    Response::builder()
        .status(status)
        .body(full_body(body))
        .expect("static response builder is valid")
}

pub fn full_body(body: Bytes) -> ResponseBody {
    let length = body.len() as u64;
    let mut body = ServerBody::new(
        Full::new(body)
            .map_err(|never: Infallible| match never {})
            .boxed(),
    );
    body.set_expected_bytes(length);
    body
}

pub fn reader_body<R>(reader: R) -> ResponseBody
where
    R: AsyncRead + Send + Sync + 'static,
{
    let stream = ReaderStream::new(reader).map_ok(Frame::data);
    ServerBody::new(StreamBody::new(stream).boxed())
}

pub fn reader_body_with_length<R>(reader: R, length: u64) -> ResponseBody
where
    R: AsyncRead + Send + Sync + 'static,
{
    let mut body = reader_body(reader);
    body.set_expected_bytes(length);
    body
}

pub(crate) fn channel_body(
    receiver: tokio::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
) -> ResponseBody {
    let stream = stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|chunk| {
            let frame = chunk.map(|chunk| Frame::data(Bytes::from(chunk)));
            (frame, receiver)
        })
    });
    ServerBody::new(StreamBody::new(stream).boxed())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transfer::{TransferLifecycle, TransferOutcome};
    use crate::{metrics::ServerMetrics, request_metadata::HttpProtocol, shutdown::ShutdownPhase};
    use futures_util::stream;
    use std::sync::Arc;

    #[tokio::test]
    async fn counts_all_emitted_data_frames() {
        let frames = stream::iter([
            Ok::<_, std::io::Error>(Frame::data(Bytes::from_static(b"one"))),
            Ok(Frame::data(Bytes::from_static(b"two"))),
            Ok(Frame::data(Bytes::from_static(b"three"))),
        ]);
        let mut body = ServerBody::new(StreamBody::new(frames).boxed());
        let lifecycle = TransferLifecycle::test();
        body.attach_lifecycle(lifecycle.clone());

        while body.frame().await.is_some() {}

        assert_eq!(lifecycle.observed(), Some((TransferOutcome::Completed, 11)));
    }

    #[tokio::test]
    async fn finalizer_removes_hop_by_hop_and_connection_nominated_headers() {
        let metrics = ServerMetrics::default();
        let mut response = text(StatusCode::OK, "body");
        response
            .headers_mut()
            .insert(header::CONNECTION, "x-remove, bad token,".parse().unwrap());
        response
            .headers_mut()
            .insert("x-remove", "secret".parse().unwrap());
        response
            .headers_mut()
            .insert("keep-alive", "timeout=5".parse().unwrap());
        response
            .headers_mut()
            .insert(header::TRANSFER_ENCODING, "chunked".parse().unwrap());
        response
            .headers_mut()
            .insert(header::UPGRADE, "websocket".parse().unwrap());
        response
            .headers_mut()
            .insert(header::TRAILER, "x-end".parse().unwrap());

        finalize_response(
            &Method::GET,
            HttpProtocol::Http2,
            ShutdownPhase::Running,
            false,
            &mut response,
            &metrics,
        );

        for name in [
            "connection",
            "x-remove",
            "keep-alive",
            "transfer-encoding",
            "upgrade",
            "trailer",
        ] {
            assert!(!response.headers().contains_key(name), "{name}");
        }
        assert_eq!(
            metrics
                .response_connection_nominated_headers_removed_total
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics
                .response_invalid_connection_tokens_total
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[tokio::test]
    async fn finalizer_enforces_bodyless_statuses_and_h1_drain_close() {
        for (status, expected_length) in [
            (StatusCode::NO_CONTENT, None),
            (StatusCode::RESET_CONTENT, Some("0")),
            (StatusCode::NOT_MODIFIED, Some("4")),
        ] {
            let metrics = ServerMetrics::default();
            let mut response = bytes(status, Bytes::from_static(b"body"), "text/plain");
            finalize_response(
                &Method::GET,
                HttpProtocol::Http11,
                ShutdownPhase::Draining,
                false,
                &mut response,
                &metrics,
            );
            assert_eq!(
                response
                    .headers()
                    .get(header::CONTENT_LENGTH)
                    .and_then(|value| value.to_str().ok()),
                expected_length
            );
            assert_eq!(response.headers().get(header::CONNECTION).unwrap(), "close");
            assert!(
                response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .is_empty()
            );
        }

        let metrics = Arc::new(ServerMetrics::default());
        let mut head = bytes(StatusCode::OK, Bytes::from_static(b"body"), "text/plain");
        finalize_response(
            &Method::HEAD,
            HttpProtocol::Http3,
            ShutdownPhase::Running,
            false,
            &mut head,
            &metrics,
        );
        assert_eq!(head.headers().get(header::CONTENT_LENGTH).unwrap(), "4");
        assert!(!head.headers().contains_key(header::CONNECTION));
        assert!(
            head.into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn drop_before_last_frame_aborts_exactly_once() {
        let frames = stream::iter([
            Ok::<_, std::io::Error>(Frame::data(Bytes::from_static(b"first"))),
            Ok(Frame::data(Bytes::from_static(b"second"))),
        ]);
        let mut body = ServerBody::new(StreamBody::new(frames).boxed());
        let lifecycle = TransferLifecycle::test();
        body.attach_lifecycle(lifecycle.clone());
        let _ = body.frame().await;
        drop(body);

        assert_eq!(lifecycle.observed(), Some((TransferOutcome::Aborted, 5)));
    }

    #[tokio::test]
    async fn body_error_finishes_as_error_exactly_once() {
        let frames = stream::iter([Err::<Frame<Bytes>, _>(std::io::Error::other("broken"))]);
        let mut body = ServerBody::new(StreamBody::new(frames).boxed());
        let lifecycle = TransferLifecycle::test();
        body.attach_lifecycle(lifecycle.clone());
        assert!(body.frame().await.expect("error frame").is_err());
        drop(body);

        assert_eq!(lifecycle.observed(), Some((TransferOutcome::Error, 0)));
    }
}
