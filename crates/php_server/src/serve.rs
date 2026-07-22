use super::{
    access_log::AccessLogEntry,
    diagnostics::{
        RequestDiagnostic, emit_request_diagnostic, emit_server_debug_lazy, header_debug_value,
        route_debug_name,
    },
    metrics::ServerMetrics,
    metrics::metrics_response,
    php_request::{
        BodyReadError, PartsAndBody, execute_builtin_router_if_configured, execute_php_request,
        prepare_request_data,
    },
    request_metadata::{HttpProtocol, RequestLocalAddr, RequestMetadata, validate_request},
    request_pipeline::PhpTransferCompletion,
    state::AppState,
    static_files::{static_file_response, static_not_acceptable_response},
    transfer::{PhpExecutionCoordinator, TransferContext, TransferLifecycle},
    transport::{
        ConnectionActivity, ConnectionProtocolTracker, ConnectionRequestGuard, TransportIo,
        idle_request_body,
    },
};
use crate::{
    acme::AcmeManager,
    response::{self, RequestBody, ResponseBody},
    routing::{ResolvedRoute, resolve_route},
    tls::TcpTls,
};
use http_body_util::BodyExt;
use hyper::{
    Method, Request, Response, StatusCode,
    body::Incoming,
    header::{self, HeaderValue},
    http::request::Parts,
    service::service_fn,
};
use hyper_util::{
    rt::{TokioExecutor, TokioIo, TokioTimer},
    server::conn::auto::Builder,
};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{LazyConfigAcceptor, server::TlsStream};
use tracing::{debug, warn};

pub(crate) async fn serve_until_shutdown(
    listener: TcpListener,
    state: Arc<AppState>,
    tls: Option<TcpTls>,
    http3_endpoint: Option<quinn::Endpoint>,
    acme_manager: Option<AcmeManager>,
) {
    let mut tasks = JoinSet::new();
    if let Some(manager) = acme_manager {
        let metrics = Arc::clone(&state.services.metrics);
        let shutdown = state.connections.shutdown.clone();
        tasks.spawn(async move {
            manager.run(metrics, shutdown).await;
        });
    }
    if let Some(endpoint) = http3_endpoint.clone() {
        let http3_state = Arc::clone(&state);
        tasks.spawn(async move {
            super::http3::serve_http3_endpoint(endpoint, http3_state).await;
        });
    }
    let mut signals = match ShutdownSignals::new() {
        Ok(signals) => Some(signals),
        Err(error) => {
            warn!(%error, "shutdown signal handlers could not be installed");
            begin_graceful_drain(&state, http3_endpoint.as_ref());
            None
        }
    };
    if let Some(signals) = signals.as_mut() {
        let mut shutdown = state.connections.shutdown.subscribe();
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let Ok((stream, peer)) = accept else {
                        continue;
                    };
                    if !state.connections.shutdown.is_running() {
                        drop(stream);
                        break;
                    }
                    let local_addr = stream.local_addr().unwrap_or(state.transport.local_addr);
                    state.services.metrics.tcp_connections_accepted_total.fetch_add(1, Ordering::Relaxed);
                    if let Err(error) = stream.set_nodelay(true) {
                        debug!(%peer, %error, "failed to set TCP_NODELAY");
                    }
                    let Ok(connection_permit) = Arc::clone(&state.connections.permits).try_acquire_owned() else {
                        state.services.metrics.connection_limit_rejections_total.fetch_add(1, Ordering::Relaxed);
                        drop(stream);
                        continue;
                    };
                    let state = Arc::clone(&state);
                    let tls = tls.clone();
                    tasks.spawn(async move {
                        let _connection_permit = connection_permit;
                        let _active = ActiveTcpConnection::new(Arc::clone(&state.services.metrics));
                        if let Some(tls) = tls {
                            let Ok(handshake_permit) = Arc::clone(&state.connections.handshake_permits).try_acquire_owned() else {
                                state.services.metrics.connection_limit_rejections_total.fetch_add(1, Ordering::Relaxed);
                                return;
                            };
                            let handshake_guard = ActiveTlsHandshake::new(Arc::clone(&state.services.metrics));
                            match timeout(
                                state.transport.limits.tls_handshake_timeout,
                                accept_tls(stream, &tls, &state.services.metrics),
                            ).await {
                                Ok(Ok(AcceptedTls::Normal(stream))) => {
                                    drop(handshake_permit);
                                    drop(handshake_guard);
                                    serve_connection(stream, state, peer, local_addr).await
                                }
                                Ok(Ok(AcceptedTls::Challenge(mut stream))) => {
                                    drop(handshake_permit);
                                    drop(handshake_guard);
                                    state.services.metrics.acme_challenge_handshakes_completed_total.fetch_add(1, Ordering::Relaxed);
                                    let _ = stream.shutdown().await;
                                }
                                Ok(Err(error)) => {
                                    state.services.metrics.tls_handshake_failures_total.fetch_add(1, Ordering::Relaxed);
                                    if error.kind() == std::io::ErrorKind::InvalidData {
                                        state.services.metrics.tls_handshake_protocol_failures_total.fetch_add(1, Ordering::Relaxed);
                                    } else {
                                        state.services.metrics.tls_handshake_io_failures_total.fetch_add(1, Ordering::Relaxed);
                                    }
                                    warn!(%peer, %error, "TLS handshake failed");
                                }
                                Err(_) => {
                                    state.services.metrics.tls_handshake_timeouts_total.fetch_add(1, Ordering::Relaxed);
                                    debug!(%peer, "TLS handshake timed out");
                                }
                            }
                        } else {
                            serve_connection(stream, state, peer, local_addr).await;
                        }
                    });
                }
                Some(_) = tasks.join_next() => {}
                _ = signals.recv() => {
                    begin_graceful_drain(&state, http3_endpoint.as_ref());
                    break;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow_and_update() != crate::shutdown::ShutdownPhase::Running {
                        begin_graceful_drain(&state, http3_endpoint.as_ref());
                        break;
                    }
                }
            }
        }
    }
    drop(listener);

    let deadline = tokio::time::sleep(state.transport.limits.graceful_shutdown_timeout);
    tokio::pin!(deadline);
    let mut forced = false;
    while !tasks.is_empty() {
        tokio::select! {
            result = tasks.join_next() => {
                if let Some(Err(error)) = result {
                    warn!(%error, "connection task failed during drain");
                }
            }
            _ = next_shutdown_signal(&mut signals) => {
                state.services.metrics.shutdown_second_signals_total.fetch_add(1, Ordering::Relaxed);
                state.services.metrics.forced_shutdowns_total.fetch_add(1, Ordering::Relaxed);
                state.connections.shutdown.force();
                forced = true;
                break;
            }
            () = &mut deadline => {
                state.services.metrics.drain_deadline_exceeded_total.fetch_add(1, Ordering::Relaxed);
                state.services.metrics.forced_shutdowns_total.fetch_add(1, Ordering::Relaxed);
                state.connections.shutdown.force();
                forced = true;
                break;
            }
        }
    }
    if forced {
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }
    if let Some(endpoint) = http3_endpoint {
        endpoint.close(quinn::VarInt::from_u32(0), b"server shutdown");
        let _ = timeout(Duration::from_secs(1), endpoint.wait_idle()).await;
    }
}

enum AcceptedTls {
    Normal(TlsStream<TcpStream>),
    Challenge(TlsStream<TcpStream>),
}

async fn accept_tls(
    stream: TcpStream,
    tls: &TcpTls,
    metrics: &ServerMetrics,
) -> Result<AcceptedTls, std::io::Error> {
    let start =
        LazyConfigAcceptor::new(tokio_rustls::rustls::server::Acceptor::default(), stream).await?;
    match tls {
        TcpTls::Manual(config) => start
            .into_stream(Arc::clone(config))
            .await
            .map(AcceptedTls::Normal),
        TcpTls::Acme(acme) => {
            let challenge = rustls_acme::is_tls_alpn_challenge(&start.client_hello());
            if challenge {
                metrics
                    .acme_challenge_client_hellos_total
                    .fetch_add(1, Ordering::Relaxed);
                if !acme.permits_challenge_sni(start.client_hello().server_name()) {
                    metrics
                        .acme_challenge_unknown_sni_total
                        .fetch_add(1, Ordering::Relaxed);
                    metrics
                        .acme_challenge_handshake_failures_total
                        .fetch_add(1, Ordering::Relaxed);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "TLS-ALPN-01 SNI is not configured",
                    ));
                }
                return start
                    .into_stream(Arc::clone(&acme.challenge_config))
                    .await
                    .map(AcceptedTls::Challenge)
                    .inspect_err(|_| {
                        metrics
                            .acme_challenge_handshake_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                    });
            }
            if !acme.status.certificate_available() {
                metrics
                    .acme_certificate_resolution_misses_total
                    .fetch_add(1, Ordering::Relaxed);
                metrics
                    .normal_tls_handshakes_without_certificate_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            let result = start
                .into_stream(acme.normal_config.clone())
                .await
                .map(AcceptedTls::Normal);
            if result.is_ok() {
                metrics
                    .acme_tcp_certificate_resolutions_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            result
        }
    }
}

fn begin_graceful_drain(state: &AppState, http3_endpoint: Option<&quinn::Endpoint>) {
    if state.connections.shutdown.begin_draining() {
        state
            .services
            .metrics
            .readiness_state
            .store(0, Ordering::Release);
        state
            .services
            .metrics
            .graceful_shutdowns_total
            .fetch_add(1, Ordering::Relaxed);
    }
    if let Some(endpoint) = http3_endpoint {
        endpoint.set_server_config(None);
    }
}

async fn next_shutdown_signal(signals: &mut Option<ShutdownSignals>) {
    match signals {
        Some(signals) => signals.recv().await,
        None => std::future::pending().await,
    }
}

#[cfg(unix)]
struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    fn new() -> std::io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    async fn recv(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
    }
}

#[cfg(not(unix))]
struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    fn new() -> std::io::Result<Self> {
        Ok(Self)
    }

    async fn recv(&mut self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}

struct ActiveTcpConnection {
    metrics: Arc<ServerMetrics>,
}

impl ActiveTcpConnection {
    fn new(metrics: Arc<ServerMetrics>) -> Self {
        metrics
            .tcp_connections_active
            .fetch_add(1, Ordering::Relaxed);
        Self { metrics }
    }
}

impl Drop for ActiveTcpConnection {
    fn drop(&mut self) {
        self.metrics
            .tcp_connections_active
            .fetch_sub(1, Ordering::Relaxed);
    }
}

struct ActiveTlsHandshake {
    metrics: Arc<ServerMetrics>,
}

impl ActiveTlsHandshake {
    fn new(metrics: Arc<ServerMetrics>) -> Self {
        metrics
            .tls_handshakes_active
            .fetch_add(1, Ordering::Relaxed);
        Self { metrics }
    }
}

impl Drop for ActiveTlsHandshake {
    fn drop(&mut self) {
        self.metrics
            .tls_handshakes_active
            .fetch_sub(1, Ordering::Relaxed);
    }
}

pub(crate) async fn serve_connection<S>(
    stream: S,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let activity = Arc::new(ConnectionActivity::default());
    let protocol = Arc::new(ConnectionProtocolTracker::new(Arc::clone(
        &state.services.metrics,
    )));
    let service_activity = Arc::clone(&activity);
    let service_protocol = Arc::clone(&protocol);
    let service_state = Arc::clone(&state);
    let service = service_fn(move |request| {
        service_protocol.observe(request.version());
        let state = Arc::clone(&service_state);
        let connection_request = service_activity.enter();
        async move {
            Ok::<_, Infallible>(handle(request, state, peer, local_addr, connection_request).await)
        }
    });
    let io = TokioIo::new(TransportIo::new(
        stream,
        activity,
        Arc::clone(&state.services.metrics),
        state.transport.limits.connection_idle_timeout,
        state.transport.limits.response_write_idle_timeout,
    ));
    let mut builder = Builder::new(TokioExecutor::new());
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(state.transport.limits.request_header_timeout)
        .half_close(false)
        .keep_alive(true)
        .ignore_invalid_headers(false)
        .max_buf_size(64 * 1024);
    // Keep Hyper's stack-backed 100-header HTTP/1 fast path: calling
    // max_headers(100) would force per-request heap allocation.
    builder
        .http2()
        .timer(TokioTimer::new())
        .max_header_list_size(
            u32::try_from(state.transport.limits.max_request_header_bytes)
                .expect("validated HTTP/2 header limit"),
        )
        .max_concurrent_streams(state.transport.limits.max_streams_per_connection)
        .initial_stream_window_size(1024 * 1024)
        .initial_connection_window_size(8 * 1024 * 1024)
        .adaptive_window(false)
        .max_send_buf_size(256 * 1024)
        .max_pending_accept_reset_streams(20)
        .keep_alive_interval(None);
    // 100 streams × the fixed 1 MiB stream window bounds peer-controlled
    // per-connection stream receive credit. Extended CONNECT remains off:
    // Hyper only enables it through the deliberately unused opt-in method.
    let connection = builder.serve_connection(io, service);
    tokio::pin!(connection);
    let mut shutdown = state.connections.shutdown.subscribe();
    loop {
        tokio::select! {
            result = &mut connection => {
                if let Err(error) = result {
                    match protocol.observed_protocol() {
                        0 | 1 if is_hyper_header_timeout(error.as_ref()) => {
                            state.services.metrics.h1_header_timeouts_total.fetch_add(1, Ordering::Relaxed);
                        }
                        2 if !error_chain_has_io_timeout(error.as_ref()) => {
                            state.services.metrics.h2_protocol_errors_total.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    debug!(%peer, %error, "HTTP connection ended with a transport error");
                }
                break;
            }
            changed = shutdown.changed() => {
                if changed.is_err() {
                    break;
                }
                match *shutdown.borrow_and_update() {
                    crate::shutdown::ShutdownPhase::Running => {}
                    crate::shutdown::ShutdownPhase::Draining => {
                        connection.as_mut().graceful_shutdown();
                    }
                    crate::shutdown::ShutdownPhase::Forced => break,
                }
            }
        }
    }
}

fn is_hyper_header_timeout(error: &(dyn std::error::Error + 'static)) -> bool {
    !error_chain_has_io_timeout(error)
        && error_chain(error).any(|error| {
            error
                .downcast_ref::<hyper::Error>()
                .is_some_and(hyper::Error::is_timeout)
        })
}

fn error_chain_has_io_timeout(error: &(dyn std::error::Error + 'static)) -> bool {
    error_chain(error).any(|error| {
        error
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::TimedOut)
    })
}

fn error_chain<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> impl Iterator<Item = &'a (dyn std::error::Error + 'static)> {
    std::iter::successors(Some(error), |error| error.source())
}

pub(crate) async fn handle(
    request: Request<Incoming>,
    state: Arc<AppState>,
    peer: SocketAddr,
    local_addr: SocketAddr,
    connection_request: ConnectionRequestGuard,
) -> Response<ResponseBody> {
    let (mut parts, body) = request.into_parts();
    parts.extensions.insert(RequestLocalAddr(local_addr));
    let body = idle_request_body(
        incoming_request_body(body),
        state.request.request_body_idle_timeout,
        Arc::clone(&state.services.metrics),
    );
    handle_parts(parts, body, state, peer, connection_request).await
}

pub(crate) async fn handle_parts(
    parts: Parts,
    body: RequestBody,
    state: Arc<AppState>,
    peer: SocketAddr,
    connection_request: ConnectionRequestGuard,
) -> Response<ResponseBody> {
    match admit_request(&parts, &state, peer, connection_request).await {
        Ok(admission) => handle_parts_admitted(parts, body, state, peer, admission).await,
        Err(response) => response,
    }
}

pub(crate) struct RequestAdmission {
    started: Instant,
    request_id: String,
    method: Method,
    request_target: String,
    permit: tokio::sync::OwnedSemaphorePermit,
    connection_request: ConnectionRequestGuard,
    metadata: Option<RequestMetadata>,
    force_connection_close: bool,
    protocol: HttpProtocol,
    admitted_before_drain: bool,
}

pub(crate) async fn admit_request(
    parts: &Parts,
    state: &Arc<AppState>,
    peer: SocketAddr,
    connection_request: ConnectionRequestGuard,
) -> Result<RequestAdmission, Response<ResponseBody>> {
    let started = Instant::now();
    let request_id = state.next_request_id();
    state
        .services
        .metrics
        .requests_total
        .fetch_add(1, Ordering::Relaxed);
    let method = parts.method.clone();
    let protocol = HttpProtocol::from_version(parts.version).unwrap_or(HttpProtocol::Http11);
    let request_target = parts
        .uri
        .path_and_query()
        .map_or_else(|| parts.uri.path().to_string(), |value| value.to_string());
    if !state.connections.shutdown.is_running()
        && !matches!(parts.uri.path(), "/healthz" | "/readyz")
    {
        state
            .services
            .metrics
            .drain_requests_rejected_total
            .fetch_add(1, Ordering::Relaxed);
        let mut response = response::text(StatusCode::SERVICE_UNAVAILABLE, "draining\n");
        response::finalize_response(
            &method,
            protocol,
            state.connections.shutdown.phase(),
            protocol.is_h1(),
            &mut response,
            &state.services.metrics,
        );
        let status = response.status();
        response
            .body_mut()
            .attach_lifecycle(TransferLifecycle::new(TransferContext {
                state: Arc::clone(state),
                request_id,
                started,
                method: method.to_string(),
                request_target,
                route: "drain-rejected",
                status,
                cache_hit: None,
                permit: None,
                php: None,
                execution: None,
                admitted_before_drain: false,
            }));
        response
            .body_mut()
            .attach_connection_request(connection_request);
        return Err(response);
    }
    emit_server_debug_lazy(
        state,
        Some(&request_id),
        "D_PHRUST_SERVER_REQUEST_ACCEPTED",
        "request",
        "server request accepted",
        || {
            BTreeMap::from([
                ("peer".to_string(), peer.to_string()),
                ("method".to_string(), method.to_string()),
                ("path".to_string(), parts.uri.path().to_string()),
                (
                    "query_present".to_string(),
                    parts.uri.query().is_some().to_string(),
                ),
                (
                    "authorization".to_string(),
                    header_debug_value(&parts.headers, header::AUTHORIZATION),
                ),
                (
                    "cookie".to_string(),
                    header_debug_value(&parts.headers, header::COOKIE),
                ),
            ])
        },
    );
    let admission_started = Instant::now();
    let permit = match timeout(
        state.request.request_admission_timeout,
        Arc::clone(&state.concurrency.in_flight).acquire_owned(),
    )
    .await
    {
        Ok(Ok(permit)) => {
            // Queue-wait signal: time spent waiting for an in-flight permit
            // (the blocking-region admission gate). Near-zero under low load;
            // grows as workers saturate.
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::AdmissionWait,
                admission_started.elapsed().as_nanos(),
            );
            permit
        }
        Ok(Err(_)) | Err(_) => {
            state
                .services
                .metrics
                .overload
                .fetch_add(1, Ordering::Relaxed);
            let mut response = overloaded();
            response::finalize_response(
                &method,
                protocol,
                state.connections.shutdown.phase(),
                protocol.is_h1(),
                &mut response,
                &state.services.metrics,
            );
            let status = response.status();
            response
                .body_mut()
                .attach_lifecycle(TransferLifecycle::new(TransferContext {
                    state: Arc::clone(state),
                    request_id,
                    started,
                    method: method.to_string(),
                    request_target,
                    route: "overload",
                    status,
                    cache_hit: None,
                    permit: None,
                    php: None,
                    execution: None,
                    admitted_before_drain: state.connections.shutdown.is_running(),
                }));
            response
                .body_mut()
                .attach_connection_request(connection_request);
            debug!(%peer, "request rejected because max in-flight admission wait expired");
            return Err(response);
        }
    };
    Ok(RequestAdmission {
        started,
        request_id,
        method,
        request_target,
        permit,
        connection_request,
        metadata: None,
        force_connection_close: false,
        protocol,
        admitted_before_drain: true,
    })
}

pub(crate) async fn handle_parts_admitted(
    mut parts: Parts,
    body: RequestBody,
    state: Arc<AppState>,
    peer: SocketAddr,
    mut admission: RequestAdmission,
) -> Response<ResponseBody> {
    match validate_request(&mut parts, &state, peer) {
        Ok(metadata) => {
            admission.metadata = Some(metadata.clone());
            parts.extensions.insert(metadata);
        }
        Err(error) => {
            admission.force_connection_close = error.force_connection_close;
            let response = if error.status == StatusCode::PAYLOAD_TOO_LARGE {
                response::text(error.status, "payload too large\n")
            } else {
                response::text(error.status, "bad request\n")
            };
            return finish_admitted_response(
                admission,
                state,
                response,
                "request-validation",
                None,
            );
        }
    }
    let method = admission.method.clone();
    let request_id = admission.request_id.clone();
    let request_version = parts.version;
    let route_started = Instant::now();
    let route = resolve_route(
        &method,
        &parts.uri,
        &parts.headers,
        &state.route_config,
        &state.static_files,
    )
    .await;
    let route_resolution = route_started.elapsed();
    state.services.metrics.record_phase(
        super::metrics::RequestPhase::RouteResolution,
        route_resolution.as_nanos(),
    );
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_ROUTE_RESOLVED",
        "routing",
        "server route resolved",
        || BTreeMap::from([("route".to_string(), route_debug_name(&route).to_string())]),
    );
    debug!(
        %peer,
        method=%method,
        path=%parts.uri.path(),
        route=?route,
        "classified request"
    );
    let (mut response, route_kind, cache_hit) = match route {
        ResolvedRoute::Health => match method {
            Method::GET => (response::text(StatusCode::OK, "ok\n"), "health", None),
            Method::HEAD => (response::empty(StatusCode::OK), "health", None),
            _ => (method_not_allowed("GET, HEAD"), "health", None),
        },
        ResolvedRoute::Ready => {
            let running = state.is_ready();
            match method {
                Method::GET if running => {
                    (response::text(StatusCode::OK, "ready\n"), "readiness", None)
                }
                Method::HEAD if running => (response::empty(StatusCode::OK), "readiness", None),
                Method::GET => (
                    response::text(StatusCode::SERVICE_UNAVAILABLE, "draining\n"),
                    "readiness",
                    None,
                ),
                Method::HEAD => (
                    response::empty(StatusCode::SERVICE_UNAVAILABLE),
                    "readiness",
                    None,
                ),
                _ => (method_not_allowed("GET, HEAD"), "readiness", None),
            }
        }
        ResolvedRoute::Metrics => (metrics_response(&state, &parts), "metrics", None),
        ResolvedRoute::CacheClear => (
            clear_cache_response(&state, peer).await,
            "cache-clear",
            None,
        ),
        ResolvedRoute::StaticFile(representation) => {
            if let Some(response) = execute_builtin_router_before_normal_route(
                &parts,
                body,
                Arc::clone(&state),
                peer,
                &request_id,
            )
            .await
            {
                (response, "builtin-router", None)
            } else {
                state
                    .services
                    .metrics
                    .static_responses
                    .fetch_add(1, Ordering::Relaxed);
                (
                    static_file_response(&parts, &state.services.metrics, representation).await,
                    "static",
                    None,
                )
            }
        }
        ResolvedRoute::DirectoryRedirect { location } => {
            let mut response = response::empty(StatusCode::PERMANENT_REDIRECT);
            match HeaderValue::from_str(&location) {
                Ok(location) => {
                    response.headers_mut().insert(header::LOCATION, location);
                    (response, "directory-redirect", None)
                }
                Err(error) => {
                    warn!(%error, "generated directory redirect location is invalid");
                    (
                        response::text(StatusCode::INTERNAL_SERVER_ERROR, "redirect failed\n"),
                        "internal-error",
                        None,
                    )
                }
            }
        }
        ResolvedRoute::NotAcceptable {
            vary_accept_encoding,
        } => (
            static_not_acceptable_response(vary_accept_encoding),
            "not-acceptable",
            None,
        ),
        ResolvedRoute::PhpScript {
            script_path,
            path_info,
        } => {
            state
                .services
                .metrics
                .php_responses
                .fetch_add(1, Ordering::Relaxed);
            let route_kind = if path_info.is_some() {
                "front-controller"
            } else {
                "php"
            };
            let (response, cache_hit) = execute_php_request(
                PartsAndBody { parts, body },
                Arc::clone(&state),
                script_path,
                path_info,
                peer,
                request_id.clone(),
                route_resolution,
            )
            .await;
            (response, route_kind, cache_hit)
        }
        ResolvedRoute::NotFound => {
            if let Some(response) = execute_builtin_router_before_normal_route(
                &parts,
                body,
                Arc::clone(&state),
                peer,
                &request_id,
            )
            .await
            {
                (response, "builtin-router", None)
            } else {
                emit_request_diagnostic(
                    &state,
                    &parts,
                    Some(&request_id),
                    RequestDiagnostic::new(
                        "E_PHP_SERVER_SCRIPT_RESOLUTION_FAILED",
                        "routing",
                        "server could not resolve a PHP script for the request",
                        "resolve_route",
                        parts.uri.path(),
                        "",
                    ),
                );
                (
                    response::text(StatusCode::NOT_FOUND, "not found\n"),
                    "not-found",
                    None,
                )
            }
        }
        ResolvedRoute::BadRequest => {
            emit_request_diagnostic(
                &state,
                &parts,
                Some(&request_id),
                RequestDiagnostic::new(
                    "E_PHP_SERVER_SCRIPT_RESOLUTION_FAILED",
                    "routing",
                    "server could not parse the request path for script resolution",
                    "resolve_route",
                    parts.uri.path(),
                    "",
                ),
            );
            (
                response::text(StatusCode::BAD_REQUEST, "bad request\n"),
                "bad-request",
                None,
            )
        }
        ResolvedRoute::MethodNotAllowed { allow } => {
            (method_not_allowed(allow), "method-not-allowed", None)
        }
        ResolvedRoute::InternalError(error) => {
            warn!(%error, "static route resolution failed");
            (
                response::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "route resolution failed\n",
                ),
                "internal-error",
                None,
            )
        }
    };
    if let Some(alt_svc) = &state.transport.http3_alt_svc
        && request_version != hyper::Version::HTTP_3
    {
        match HeaderValue::from_str(alt_svc) {
            Ok(value) => {
                response
                    .headers_mut()
                    .entry(header::ALT_SVC)
                    .or_insert(value);
            }
            Err(error) => {
                warn!(%alt_svc, %error, "HTTP/3 Alt-Svc header value is invalid");
            }
        }
    }
    finish_admitted_response(admission, state, response, route_kind, cache_hit)
}

pub(crate) fn finish_admitted_response(
    admission: RequestAdmission,
    state: Arc<AppState>,
    mut response: Response<ResponseBody>,
    route: &'static str,
    cache_hit: Option<bool>,
) -> Response<ResponseBody> {
    response::finalize_response(
        &admission.method,
        admission
            .metadata
            .as_ref()
            .map_or(admission.protocol, |metadata| metadata.protocol),
        state.connections.shutdown.phase(),
        admission.force_connection_close
            || admission
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.force_connection_close),
        &mut response,
        &state.services.metrics,
    );
    let status = response.status();
    let php = response.extensions_mut().remove::<PhpTransferCompletion>();
    let execution = response
        .extensions_mut()
        .remove::<PhpExecutionCoordinator>();
    response
        .body_mut()
        .attach_lifecycle(TransferLifecycle::new(TransferContext {
            state,
            request_id: admission.request_id,
            started: admission.started,
            method: admission.method.to_string(),
            request_target: admission.request_target,
            route,
            status,
            cache_hit,
            permit: Some(admission.permit),
            php,
            execution,
            admitted_before_drain: admission.admitted_before_drain,
        }));
    response
        .body_mut()
        .attach_connection_request(admission.connection_request);
    response
}
pub(crate) fn write_access_log(state: &AppState, entry: AccessLogEntry<'_>) {
    if let Some(access_log) = &state.observability.access_log
        && let Err(error) = access_log.write(&entry)
    {
        warn!(%error, "access log write failed");
    }
}
async fn execute_builtin_router_before_normal_route(
    parts: &Parts,
    body: RequestBody,
    state: Arc<AppState>,
    peer: SocketAddr,
    request_id: &str,
) -> Option<Response<ResponseBody>> {
    state.route_config.builtin_router.as_ref()?;
    emit_server_debug_lazy(
        &state,
        Some(request_id),
        "D_PHRUST_SERVER_BODY_READ_START",
        "body_read",
        "request body read started",
        || {
            BTreeMap::from([(
                "max_body_bytes".to_string(),
                state.request.max_body_bytes.to_string(),
            )])
        },
    );
    let body_started = Instant::now();
    let prepared = match timeout(
        state.request.request_body_timeout,
        prepare_request_data(body, parts, &state),
    )
    .await
    {
        Err(_) => {
            state
                .services
                .metrics
                .request_body_total_timeouts_total
                .fetch_add(1, Ordering::Relaxed);
            emit_server_debug_lazy(
                &state,
                Some(request_id),
                "D_PHRUST_SERVER_BODY_READ_TIMEOUT",
                "body_read",
                "request body read timed out",
                || {
                    BTreeMap::from([(
                        "timeout_ms".to_string(),
                        state.request.request_body_timeout.as_millis().to_string(),
                    )])
                },
            );
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::BodyRead,
                body_started.elapsed().as_nanos(),
            );
            return Some(response::text(
                StatusCode::REQUEST_TIMEOUT,
                "request timeout\n",
            ));
        }
        Ok(Ok(prepared)) => prepared,
        Ok(Err(BodyReadError::TooLarge)) => {
            state
                .services
                .metrics
                .body_too_large
                .fetch_add(1, Ordering::Relaxed);
            emit_server_debug_lazy(
                &state,
                Some(request_id),
                "D_PHRUST_SERVER_BODY_TOO_LARGE",
                "body_read",
                "request body exceeded configured limit",
                || {
                    BTreeMap::from([(
                        "max_body_bytes".to_string(),
                        state.request.max_body_bytes.to_string(),
                    )])
                },
            );
            debug!(%peer, max_body_bytes=state.request.max_body_bytes, "request body too large");
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::BodyRead,
                body_started.elapsed().as_nanos(),
            );
            return Some(response::text(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload too large\n",
            ));
        }
        Ok(Err(BodyReadError::IdleTimeout)) => {
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::BodyRead,
                body_started.elapsed().as_nanos(),
            );
            return Some(response::text(
                StatusCode::REQUEST_TIMEOUT,
                "request timeout\n",
            ));
        }
        Ok(Err(BodyReadError::Invalid)) => {
            emit_server_debug_lazy(
                &state,
                Some(request_id),
                "D_PHRUST_SERVER_BODY_INVALID",
                "body_read",
                "request body read failed",
                BTreeMap::new,
            );
            warn!(%peer, "failed to read request body");
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::BodyRead,
                body_started.elapsed().as_nanos(),
            );
            return Some(response::text(StatusCode::BAD_REQUEST, "bad request\n"));
        }
        Ok(Err(BodyReadError::Internal)) => {
            warn!(%peer, "request body temporary storage failed");
            state.services.metrics.record_phase(
                super::metrics::RequestPhase::BodyRead,
                body_started.elapsed().as_nanos(),
            );
            return Some(response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "body storage failed\n",
            ));
        }
    };
    state.services.metrics.record_phase(
        super::metrics::RequestPhase::BodyRead,
        body_started.elapsed().as_nanos(),
    );
    emit_server_debug_lazy(
        &state,
        Some(request_id),
        "D_PHRUST_SERVER_BODY_READ_END",
        "body_read",
        "request body read completed",
        || BTreeMap::from([("body_bytes".to_string(), prepared.body.len().to_string())]),
    );
    execute_builtin_router_if_configured(
        parts,
        state,
        prepared.body,
        prepared.parsed,
        request_id,
        None,
    )
    .await
}

pub(crate) async fn clear_cache_response(
    state: &AppState,
    peer: SocketAddr,
) -> Response<ResponseBody> {
    if !peer.ip().is_loopback() {
        return response::text(StatusCode::FORBIDDEN, "forbidden\n");
    }
    if let Err(error) = state.static_files.rebuild_index().await {
        warn!(%error, "failed to rebuild immutable static index");
        return response::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            "static index rebuild failed\n",
        );
    }
    state.services.engine.script_cache.clear();
    if let Err(error) = state.services.engine.include_cache.clear() {
        warn!(%error, "failed to clear include cache");
        return response::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            "include cache clear failed\n",
        );
    }
    response::text(StatusCode::OK, "cache cleared\n")
}

pub(crate) fn method_not_allowed(allow: &'static str) -> Response<ResponseBody> {
    let mut response = response::text(StatusCode::METHOD_NOT_ALLOWED, "method not allowed\n");
    response
        .headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static(allow));
    response
}

pub(crate) fn overloaded() -> Response<ResponseBody> {
    let mut response = response::text(StatusCode::SERVICE_UNAVAILABLE, "server overloaded\n");
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}

pub(crate) fn incoming_request_body(body: Incoming) -> RequestBody {
    body.map_err(|error| std::io::Error::other(error.to_string()))
        .boxed()
}
