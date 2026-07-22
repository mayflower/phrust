use super::php_request::RequestLocalAddr;
use crate::{
    response::ResponseBody,
    serve::{admit_request, handle_parts_admitted},
    server::ServerError,
    state::AppState,
    tls::build_quic_server_config,
};
use bytes::{Buf, Bytes};
use h3::error::Code;
use h3::server::RequestStream;
use http_body_util::BodyExt;
use hyper::{
    Response,
    body::{Body, Frame, SizeHint},
    header::{self, HeaderName},
};
use std::{
    net::SocketAddr,
    path::Path,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tracing::{debug, warn};

pub(crate) fn build_http3_endpoint(
    cert_path: &Path,
    key_path: &Path,
    listen: SocketAddr,
) -> Result<quinn::Endpoint, ServerError> {
    let server_config = build_quic_server_config(cert_path, key_path)?;
    quinn::Endpoint::server(server_config, listen).map_err(ServerError::Io)
}

pub(crate) async fn serve_http3_endpoint(endpoint: quinn::Endpoint, state: Arc<AppState>) {
    let mut tasks = JoinSet::new();
    let local_addr = match endpoint.local_addr() {
        Ok(addr) => addr,
        Err(error) => {
            warn!(%error, "HTTP/3 endpoint local address unavailable");
            return;
        }
    };
    while let Some(incoming) = endpoint.accept().await {
        let peer = incoming.remote_address();
        let state = Arc::clone(&state);
        tasks.spawn(async move {
            match incoming.await {
                Ok(connection) => serve_http3_connection(connection, state, peer, local_addr).await,
                Err(error) => warn!(%peer, %error, "HTTP/3 QUIC handshake failed"),
            }
        });
        while let Some(result) = tasks.try_join_next() {
            if let Err(error) = result {
                warn!(%error, "HTTP/3 connection task failed");
            }
        }
    }
}

async fn serve_http3_connection(
    connection: quinn::Connection,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
) {
    let quic = h3_quinn::Connection::new(connection);
    let mut connection = match h3::server::builder().build(quic).await {
        Ok(connection) => connection,
        Err(error) => {
            warn!(%peer, %error, "HTTP/3 connection setup failed");
            return;
        }
    };

    loop {
        match connection.accept().await {
            Ok(Some(resolver)) => {
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    match resolver.resolve_request().await {
                        Ok((request, stream)) => {
                            handle_http3_request(request, stream, state, peer, local_addr).await
                        }
                        Err(error) => warn!(%peer, %error, "HTTP/3 request resolution failed"),
                    }
                });
            }
            Ok(None) => break,
            Err(error) => {
                debug!(%peer, %error, "HTTP/3 connection accept ended");
                break;
            }
        }
    }
}

async fn handle_http3_request(
    request: hyper::Request<()>,
    stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
) {
    let (mut parts, ()) = request.into_parts();
    parts.extensions.insert(RequestLocalAddr(local_addr));
    let admission = match admit_request(&parts, &state, peer).await {
        Ok(admission) => admission,
        Err(response) => {
            let (send, mut recv) = stream.split();
            if let Err(error) = send_http3_response(send, response, None).await {
                warn!(%peer, %error, "HTTP/3 overload response failed");
            }
            recv.stop_sending(Code::H3_REQUEST_CANCELLED);
            return;
        }
    };
    let (send, recv) = stream.split();
    let control = Arc::new(Http3RequestControl::default());
    let body = Http3RequestBody::new(recv, Arc::clone(&control)).boxed();
    let response = handle_parts_admitted(parts, body, state, peer, admission).await;
    if let Err(error) = send_http3_response(send, response, Some(&control)).await {
        warn!(%peer, %error, "HTTP/3 response send failed");
    }
}

#[derive(Default)]
struct Http3RequestControl {
    response_started: AtomicBool,
    response_started_notify: Notify,
}

impl Http3RequestControl {
    fn mark_response_started(&self) {
        self.response_started.store(true, Ordering::Release);
        self.response_started_notify.notify_waiters();
    }

    async fn wait_for_response_start(&self) {
        loop {
            let notified = self.response_started_notify.notified();
            if self.response_started.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}

struct Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + 'static,
{
    stream: Option<RequestStream<S, Bytes>>,
    control: Arc<Http3RequestControl>,
    complete: bool,
}

impl<S> Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + 'static,
{
    fn new(stream: RequestStream<S, Bytes>, control: Arc<Http3RequestControl>) -> Self {
        Self {
            stream: Some(stream),
            control,
            complete: false,
        }
    }
}

impl<S> Body for Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + Sync + Unpin + 'static,
{
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let stream = self.stream.as_mut().expect("HTTP/3 receive stream");
        match stream.poll_recv_data(context) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(Some(mut data))) => {
                let bytes = data.copy_to_bytes(data.remaining());
                Poll::Ready(Some(Ok(Frame::data(bytes))))
            }
            Poll::Ready(Ok(None)) => {
                self.complete = true;
                Poll::Ready(None)
            }
            Poll::Ready(Err(error)) => {
                self.complete = true;
                Poll::Ready(Some(Err(std::io::Error::other(error.to_string()))))
            }
        }
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }
}

impl<S> Drop for Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + 'static,
{
    fn drop(&mut self) {
        if !self.complete
            && let Some(mut stream) = self.stream.take()
        {
            let control = Arc::clone(&self.control);
            tokio::spawn(async move {
                let _ =
                    tokio::time::timeout(Duration::from_secs(5), control.wait_for_response_start())
                        .await;
                stream.stop_sending(Code::H3_REQUEST_CANCELLED);
            });
        }
    }
}

async fn send_http3_response<S>(
    mut stream: RequestStream<S, Bytes>,
    response: Response<ResponseBody>,
    request_control: Option<&Http3RequestControl>,
) -> Result<(), String>
where
    S: h3::quic::SendStream<Bytes>,
{
    let (mut parts, mut body) = response.into_parts();
    body.defer_completion();
    strip_http3_forbidden_headers(&mut parts.headers);
    let response_result = stream.send_response(Response::from_parts(parts, ())).await;
    if let Some(control) = request_control {
        control.mark_response_started();
    }
    if let Err(error) = response_result {
        body.error();
        return Err(error.to_string());
    }
    while let Some(frame) = body.frame().await {
        let frame = match frame {
            Ok(frame) => frame,
            Err(error) => {
                body.error();
                return Err(error.to_string());
            }
        };
        let frame = match frame.into_data() {
            Ok(data) => {
                if !data.is_empty()
                    && let Err(error) = stream.send_data(data).await
                {
                    body.abort();
                    return Err(error.to_string());
                }
                continue;
            }
            Err(frame) => frame,
        };
        if let Ok(trailers) = frame.into_trailers()
            && let Err(error) = stream.send_trailers(trailers).await
        {
            body.abort();
            return Err(error.to_string());
        }
    }
    if let Err(error) = stream.finish().await {
        body.abort();
        return Err(error.to_string());
    }
    body.complete();
    Ok(())
}

fn strip_http3_forbidden_headers(headers: &mut header::HeaderMap) {
    for name in [
        header::CONNECTION,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
        HeaderName::from_static("keep-alive"),
        HeaderName::from_static("proxy-connection"),
    ] {
        headers.remove(name);
    }
}
