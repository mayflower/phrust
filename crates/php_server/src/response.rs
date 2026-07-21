use bytes::Bytes;
use futures_util::{TryStreamExt, stream};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::{
    Response, StatusCode,
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

use crate::transfer::{TransferLifecycle, TransferOutcome};
use php_runtime::api::RuntimeCancellationState;

pub type ResponseBody = ServerBody;
pub type RequestBody = BoxBody<Bytes, std::io::Error>;

pub struct ServerBody {
    inner: BoxBody<Bytes, std::io::Error>,
    lifecycle: Option<TransferLifecycle>,
    emitted_bytes: u64,
    finished: bool,
    defer_completion: bool,
    reached_end: bool,
    cancellation: Option<RuntimeCancellationState>,
    expected_bytes: Option<u64>,
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
        if let Some(lifecycle) = self.lifecycle.take() {
            lifecycle.finish_transfer(outcome, self.emitted_bytes);
        }
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

pub fn request_body_from_bytes(body: Bytes) -> RequestBody {
    Full::new(body)
        .map_err(|never: Infallible| match never {})
        .boxed()
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
    use futures_util::stream;

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
