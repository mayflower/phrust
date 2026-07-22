use crate::{
    metrics::ServerMetrics,
    request_metadata::RequestLocalAddr,
    response::ResponseBody,
    serve::{admit_request, handle_parts_admitted},
    server::ServerError,
    state::AppState,
    tls::build_quic_server_config,
    transport::{ConnectionActivity, idle_request_body},
};
use bytes::{Buf, Bytes};
use h3::error::Code;
use h3::server::RequestStream;
use http_body_util::BodyExt;
use hyper::{
    Response,
    body::{Body, Frame, SizeHint},
};
use std::{
    net::SocketAddr,
    path::Path,
    pin::Pin,
    sync::{Arc, atomic::Ordering},
    task::{Context, Poll},
    time::Duration,
};
use tokio::task::JoinSet;
use tracing::{debug, warn};

pub(crate) fn build_http3_endpoint(
    cert_path: &Path,
    key_path: &Path,
    listen: SocketAddr,
    max_streams_per_connection: u32,
    connection_idle_timeout: Duration,
) -> Result<quinn::Endpoint, ServerError> {
    let server_config = build_quic_server_config(
        cert_path,
        key_path,
        max_streams_per_connection,
        connection_idle_timeout,
    )?;
    quinn::Endpoint::server(server_config, listen).map_err(ServerError::Io)
}

pub(crate) async fn serve_http3_endpoint(endpoint: quinn::Endpoint, state: Arc<AppState>) {
    let mut tasks = JoinSet::new();
    let mut shutdown = state.connections.shutdown.subscribe();
    let local_addr = match endpoint.local_addr() {
        Ok(addr) => addr,
        Err(error) => {
            warn!(%error, "HTTP/3 endpoint local address unavailable");
            return;
        }
    };
    loop {
        let incoming = tokio::select! {
            incoming = endpoint.accept() => incoming,
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow_and_update() != crate::shutdown::ShutdownPhase::Running {
                    break;
                }
                continue;
            }
        };
        let Some(incoming) = incoming else {
            break;
        };
        if !state.connections.shutdown.is_running() {
            incoming.refuse();
            break;
        }
        let peer = incoming.remote_address();
        let Ok(connection_permit) = Arc::clone(&state.connections.permits).try_acquire_owned()
        else {
            state
                .services
                .metrics
                .connection_limit_rejections_total
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .h3_connection_limit_rejections_total
                .fetch_add(1, Ordering::Relaxed);
            incoming.refuse();
            continue;
        };
        let Ok(handshake_permit) =
            Arc::clone(&state.connections.handshake_permits).try_acquire_owned()
        else {
            state
                .services
                .metrics
                .connection_limit_rejections_total
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .h3_connection_limit_rejections_total
                .fetch_add(1, Ordering::Relaxed);
            incoming.refuse();
            continue;
        };
        let state = Arc::clone(&state);
        tasks.spawn(async move {
            let _connection_permit = connection_permit;
            match tokio::time::timeout(state.transport.limits.tls_handshake_timeout, incoming).await
            {
                Ok(Ok(connection)) => {
                    drop(handshake_permit);
                    let _active = ActiveHttp3Connection::new(Arc::clone(&state.services.metrics));
                    state
                        .services
                        .metrics
                        .h3_connections_accepted_total
                        .fetch_add(1, Ordering::Relaxed);
                    serve_http3_connection(connection, state, peer, local_addr).await
                }
                Ok(Err(error)) => warn!(%peer, %error, "HTTP/3 QUIC handshake failed"),
                Err(_) => {
                    state
                        .services
                        .metrics
                        .h3_handshake_timeouts_total
                        .fetch_add(1, Ordering::Relaxed);
                    debug!(%peer, "HTTP/3 QUIC handshake timed out");
                }
            }
        });
        while let Some(result) = tasks.try_join_next() {
            if let Err(error) = result {
                warn!(%error, "HTTP/3 connection task failed");
            }
        }
    }
    while !tasks.is_empty() {
        tokio::select! {
            result = tasks.join_next() => {
                if let Some(Err(error)) = result {
                    warn!(%error, "HTTP/3 connection task failed during drain");
                }
            }
            changed = shutdown.changed() => {
                let forced = changed.is_err()
                    || *shutdown.borrow_and_update() == crate::shutdown::ShutdownPhase::Forced;
                if forced {
                    tasks.abort_all();
                    while tasks.join_next().await.is_some() {}
                    break;
                }
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
    let mut builder = h3::server::builder();
    builder
        .max_field_section_size(state.transport.limits.max_request_header_bytes as u64)
        .enable_extended_connect(false)
        .enable_datagram(false)
        .enable_webtransport(false)
        .max_webtransport_sessions(0);
    let mut connection = match builder.build(quic).await {
        Ok(connection) => connection,
        Err(error) => {
            observe_quic_idle_timeout(&error, &state.services.metrics);
            warn!(%peer, %error, "HTTP/3 connection setup failed");
            return;
        }
    };
    let activity = Arc::new(ConnectionActivity::default());
    let mut requests = JoinSet::new();
    let mut shutdown = state.connections.shutdown.subscribe();

    loop {
        let accepted = tokio::select! {
            accepted = connection.accept() => accepted,
            changed = shutdown.changed() => {
                if changed.is_err() {
                    break;
                }
                let phase = *shutdown.borrow_and_update();
                match phase {
                    crate::shutdown::ShutdownPhase::Running => continue,
                    crate::shutdown::ShutdownPhase::Draining => {
                        if let Err(error) = connection.shutdown(0).await {
                            debug!(%peer, %error, "HTTP/3 GOAWAY failed");
                        }
                        break;
                    }
                    crate::shutdown::ShutdownPhase::Forced => {
                        requests.abort_all();
                        break;
                    }
                }
            }
        };
        match accepted {
            Ok(Some(resolver)) => {
                while let Some(result) = requests.try_join_next() {
                    if let Err(error) = result {
                        state
                            .services
                            .metrics
                            .h3_request_task_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                        warn!(%peer, %error, "HTTP/3 request task failed");
                    }
                }
                if requests.len() >= state.transport.limits.max_streams_per_connection as usize {
                    state
                        .services
                        .metrics
                        .h3_request_stream_limit_rejections_total
                        .fetch_add(1, Ordering::Relaxed);
                    drop(resolver);
                    continue;
                }
                let request_state = Arc::clone(&state);
                let activity = Arc::clone(&activity);
                requests.spawn(async move {
                    let _active =
                        ActiveHttp3Request::new(Arc::clone(&request_state.services.metrics));
                    match tokio::time::timeout(
                        request_state.transport.limits.request_header_timeout,
                        resolver.resolve_request(),
                    )
                    .await
                    {
                        Ok(Ok((request, stream))) => {
                            handle_http3_request(
                                request,
                                stream,
                                request_state,
                                peer,
                                local_addr,
                                activity,
                            )
                            .await
                        }
                        Ok(Err(error)) => warn!(%peer, %error, "HTTP/3 request resolution failed"),
                        Err(_) => {
                            request_state
                                .services
                                .metrics
                                .h3_header_timeouts_total
                                .fetch_add(1, Ordering::Relaxed);
                            debug!(%peer, "HTTP/3 request header resolution timed out");
                        }
                    }
                });
                while let Some(result) = requests.try_join_next() {
                    if let Err(error) = result {
                        state
                            .services
                            .metrics
                            .h3_request_task_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                        warn!(%peer, %error, "HTTP/3 request task failed");
                    }
                }
            }
            Ok(None) => break,
            Err(error) => {
                observe_quic_idle_timeout(&error, &state.services.metrics);
                debug!(%peer, %error, "HTTP/3 connection accept ended");
                break;
            }
        }
    }
    while !requests.is_empty() {
        tokio::select! {
            result = requests.join_next() => {
                if let Some(Err(error)) = result {
                    state
                        .services
                        .metrics
                        .h3_request_task_failures_total
                        .fetch_add(1, Ordering::Relaxed);
                    warn!(%peer, %error, "HTTP/3 request task failed");
                }
            }
            changed = shutdown.changed() => {
                let forced = changed.is_err()
                    || *shutdown.borrow_and_update() == crate::shutdown::ShutdownPhase::Forced;
                if forced {
                    requests.abort_all();
                    while requests.join_next().await.is_some() {}
                    break;
                }
            }
        }
    }
}

fn observe_quic_idle_timeout(error: &h3::error::ConnectionError, metrics: &ServerMetrics) {
    // h3 0.0.8 exposes the timeout variant as non-exhaustive without an
    // is_timeout accessor. Its public Display value is the only stable
    // classifier available at this layer.
    if error.to_string() == "Timeout" {
        metrics
            .quic_idle_timeouts_total
            .fetch_add(1, Ordering::Relaxed);
    }
}

async fn handle_http3_request(
    request: hyper::Request<()>,
    stream: RequestStream<h3_quinn::BidiStream<Bytes>, Bytes>,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
    activity: Arc<ConnectionActivity>,
) {
    let (mut parts, ()) = request.into_parts();
    parts.extensions.insert(RequestLocalAddr(local_addr));
    let admission = match admit_request(&parts, &state, peer, activity.enter()).await {
        Ok(admission) => admission,
        Err(response) => {
            let (send, mut recv) = stream.split();
            if let Err(error) = send_http3_response(
                send,
                response,
                state.transport.limits.response_write_idle_timeout,
                &state.services.metrics,
            )
            .await
            {
                warn!(%peer, %error, "HTTP/3 overload response failed");
            }
            recv.stop_sending(Code::H3_REQUEST_CANCELLED);
            return;
        }
    };
    let (send, recv) = stream.split();
    let body = Http3RequestBody::new(recv).boxed();
    let body = idle_request_body(
        body,
        state.request.request_body_idle_timeout,
        Arc::clone(&state.services.metrics),
    );
    let write_timeout = state.transport.limits.response_write_idle_timeout;
    let metrics = Arc::clone(&state.services.metrics);
    let response = handle_parts_admitted(parts, body, state, peer, admission).await;
    if let Err(error) = send_http3_response(send, response, write_timeout, &metrics).await {
        warn!(%peer, %error, "HTTP/3 response send failed");
    }
}

struct Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + 'static,
{
    stream: Option<RequestStream<S, Bytes>>,
    complete: bool,
}

impl<S> Http3RequestBody<S>
where
    S: h3::quic::RecvStream + Send + 'static,
{
    fn new(stream: RequestStream<S, Bytes>) -> Self {
        Self {
            stream: Some(stream),
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

async fn send_http3_response<S>(
    mut stream: RequestStream<S, Bytes>,
    response: Response<ResponseBody>,
    write_timeout: Duration,
    metrics: &ServerMetrics,
) -> Result<(), String>
where
    S: h3::quic::SendStream<Bytes>,
{
    let (parts, mut body) = response.into_parts();
    body.defer_completion();
    let response_result = tokio::time::timeout(
        write_timeout,
        stream.send_response(Response::from_parts(parts, ())),
    )
    .await
    .map_err(|_| h3_write_timeout(metrics))?;
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
                    && let Err(error) = tokio::time::timeout(write_timeout, stream.send_data(data))
                        .await
                        .map_err(|_| h3_write_timeout(metrics))?
                {
                    body.abort();
                    return Err(error.to_string());
                }
                continue;
            }
            Err(frame) => frame,
        };
        if let Ok(trailers) = frame.into_trailers()
            && let Err(error) = tokio::time::timeout(write_timeout, stream.send_trailers(trailers))
                .await
                .map_err(|_| h3_write_timeout(metrics))?
        {
            body.abort();
            return Err(error.to_string());
        }
    }
    if let Err(error) = tokio::time::timeout(write_timeout, stream.finish())
        .await
        .map_err(|_| h3_write_timeout(metrics))?
    {
        body.abort();
        return Err(error.to_string());
    }
    body.complete();
    Ok(())
}

fn h3_write_timeout(metrics: &ServerMetrics) -> String {
    metrics
        .h3_write_timeouts_total
        .fetch_add(1, Ordering::Relaxed);
    metrics
        .response_write_idle_timeouts_total
        .fetch_add(1, Ordering::Relaxed);
    "HTTP/3 response write idle timeout".to_string()
}

struct ActiveHttp3Connection {
    metrics: Arc<ServerMetrics>,
}

impl ActiveHttp3Connection {
    fn new(metrics: Arc<ServerMetrics>) -> Self {
        metrics
            .h3_connections_active
            .fetch_add(1, Ordering::Relaxed);
        Self { metrics }
    }
}

impl Drop for ActiveHttp3Connection {
    fn drop(&mut self) {
        self.metrics
            .h3_connections_active
            .fetch_sub(1, Ordering::Relaxed);
    }
}

struct ActiveHttp3Request {
    metrics: Arc<ServerMetrics>,
}

impl ActiveHttp3Request {
    fn new(metrics: Arc<ServerMetrics>) -> Self {
        metrics
            .h3_request_streams_active
            .fetch_add(1, Ordering::Relaxed);
        Self { metrics }
    }
}

impl Drop for ActiveHttp3Request {
    fn drop(&mut self) {
        self.metrics
            .h3_request_streams_active
            .fetch_sub(1, Ordering::Relaxed);
    }
}
