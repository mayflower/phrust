use super::php_request::RequestLocalAddr;
use crate::{
    response::{self, ResponseBody},
    serve::{admit_request, bytes_request_body, finish_admitted_response, handle_parts_admitted},
    server::ServerError,
    state::AppState,
    tls::build_quic_server_config,
};
use bytes::{Buf, Bytes, BytesMut};
use h3::server::RequestStream;
use http_body_util::BodyExt;
use hyper::{
    Response, StatusCode,
    header::{self, HeaderName},
};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{Arc, atomic::Ordering},
};
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
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
) {
    let (mut parts, ()) = request.into_parts();
    parts.extensions.insert(RequestLocalAddr(local_addr));
    let admission = match admit_request(&parts, &state, peer).await {
        Ok(admission) => admission,
        Err(response) => {
            if let Err(error) = send_http3_response(stream, response).await {
                warn!(%peer, %error, "HTTP/3 overload response failed");
            }
            return;
        }
    };
    let body = match read_http3_request_body(&mut stream, state.request.max_body_bytes).await {
        Ok(body) => body,
        Err(Http3BodyReadError::Invalid(error)) => {
            warn!(%peer, %error, "HTTP/3 request body read failed");
            let response = finish_admitted_response(
                admission,
                state,
                response::text(StatusCode::BAD_REQUEST, "bad request\n"),
                "bad-request",
                None,
            );
            if let Err(error) = send_http3_response(stream, response).await {
                warn!(%peer, %error, "HTTP/3 bad-request response failed");
            }
            return;
        }
        Err(Http3BodyReadError::TooLarge) => {
            state
                .services
                .metrics
                .body_too_large
                .fetch_add(1, Ordering::Relaxed);
            let response = finish_admitted_response(
                admission,
                state,
                response::text(StatusCode::PAYLOAD_TOO_LARGE, "request body too large\n"),
                "body-too-large",
                None,
            );
            if let Err(error) = send_http3_response(stream, response).await {
                warn!(%peer, %error, "HTTP/3 payload-too-large response failed");
            }
            return;
        }
    };
    let response =
        handle_parts_admitted(parts, bytes_request_body(body), state, peer, admission).await;
    if let Err(error) = send_http3_response(stream, response).await {
        warn!(%peer, %error, "HTTP/3 response send failed");
    }
}

#[derive(Debug)]
enum Http3BodyReadError {
    Invalid(String),
    TooLarge,
}

async fn read_http3_request_body(
    stream: &mut RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    max_body_bytes: usize,
) -> Result<Bytes, Http3BodyReadError> {
    let mut body = BytesMut::new();
    while let Some(mut chunk) = stream
        .recv_data()
        .await
        .map_err(|error| Http3BodyReadError::Invalid(error.to_string()))?
    {
        while chunk.has_remaining() {
            let remaining = chunk.remaining();
            if remaining > max_body_bytes.saturating_sub(body.len()) {
                return Err(Http3BodyReadError::TooLarge);
            }
            let bytes = chunk.copy_to_bytes(remaining);
            body.extend_from_slice(&bytes);
        }
    }
    Ok(body.freeze())
}

async fn send_http3_response(
    mut stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    response: Response<ResponseBody>,
) -> Result<(), String> {
    let (mut parts, mut body) = response.into_parts();
    body.defer_completion();
    strip_http3_forbidden_headers(&mut parts.headers);
    if let Err(error) = stream.send_response(Response::from_parts(parts, ())).await {
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
