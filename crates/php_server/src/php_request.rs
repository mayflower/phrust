use super::{
    diagnostics::{RequestDiagnostic, emit_request_diagnostic, emit_server_debug_lazy},
    metrics::RequestPhase,
    perf_trace::PerfTraceEvent,
    request_pipeline::{PhpTransferCompletion, RequestCleanup, RequestOutcome, RequestStage},
    sessions::{SessionRequestCallbacks, seed_session_state},
    state::{AppState, RequestExecutorCacheKey},
};
use crate::{
    multipart::{
        MultipartError, ParsedRequestData, parse_multipart_stream, validated_multipart_boundary,
    },
    response::{self, RequestBody, ResponseBody},
    routing::RequestRewriteRule,
    transfer::PhpExecutionCoordinator,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use bytes::{Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::{
    Method, Response, StatusCode,
    header::{self, HeaderName, HeaderValue},
    http::{HeaderMap, request::Parts},
};
use php_executor::{
    CompiledPhpScript, CompiledScriptCacheLookup, PhpExecutionError, PhpExecutionOutput,
    PhpExecutionStatus, PhpExecutor, PhpRequestExecutionInput,
};
use php_runtime::api::{
    OutputDeliveryError, OutputSink, OutputSinkHandle, RequestParseBodyError,
    RequestParseBodyOptions, RequestParserCallback, RuntimeCancellationState, RuntimeContext,
    RuntimeHttpRequestContext, RuntimeHttpResponseState, RuntimeParsedRequestData,
    RuntimeRequestBody, SessionState, Value, parse_cookie_header, parse_form_urlencoded_reader,
    parse_form_urlencoded_reader_with_separators,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{OwnedSemaphorePermit, TryAcquireError, mpsc, oneshot},
    time::timeout,
};
use tracing::{debug, warn};

pub(crate) struct PartsAndBody {
    pub(crate) parts: Parts,
    pub(crate) body: RequestBody,
}

const PHP_OUTPUT_QUEUE_CAPACITY: usize = 4;
const ROUTER_MEMORY_SPOOL_BYTES: usize = 64 * 1024;
static ROUTER_SPOOL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct PhpResponseHead {
    response: RuntimeHttpResponseState,
    complete_length: Option<u64>,
}

struct PhpResponseSink {
    head: Mutex<Option<oneshot::Sender<PhpResponseHead>>>,
    chunks: Option<mpsc::Sender<Result<Vec<u8>, std::io::Error>>>,
    suppress_body: std::sync::atomic::AtomicBool,
    cancellation: RuntimeCancellationState,
    metrics: Arc<super::metrics::ServerMetrics>,
    defer_head_until_finish: bool,
}

impl OutputSink for PhpResponseSink {
    fn commit_before_write(&self) -> bool {
        !self.defer_head_until_finish
    }

    fn commit(
        &self,
        response: &RuntimeHttpResponseState,
        complete_length: Option<u64>,
    ) -> Result<(), OutputDeliveryError> {
        let status = StatusCode::from_u16(response.status_code).unwrap_or(StatusCode::OK);
        let timeout_body = (status == StatusCode::GATEWAY_TIMEOUT && complete_length == Some(0))
            .then_some(b"php execution timeout\n".to_vec());
        let complete_length = timeout_body
            .as_ref()
            .map_or(complete_length, |body| Some(body.len() as u64));
        if status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED {
            self.suppress_body.store(true, Ordering::Release);
        }
        let sender = self
            .head
            .lock()
            .map_err(|_| OutputDeliveryError::new("PHP response head lock poisoned"))?
            .take();
        if let Some(sender) = sender
            && sender
                .send(PhpResponseHead {
                    response: response.clone(),
                    complete_length,
                })
                .is_err()
        {
            self.cancellation.cancel();
            return Err(OutputDeliveryError::new(
                "PHP response head receiver closed",
            ));
        }
        if let Some(body) = timeout_body {
            self.write(body)?;
        }
        Ok(())
    }

    fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError> {
        if self.suppress_body.load(Ordering::Acquire) {
            return Ok(());
        }
        if self.cancellation.is_cancelled() {
            return Ok(());
        }
        let Some(sender) = &self.chunks else {
            return Ok(());
        };
        let wait_started = Instant::now();
        let result = sender.blocking_send(Ok(chunk));
        self.metrics.php_output_backpressure_nanos.fetch_add(
            wait_started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        if result.is_err() {
            self.cancellation.cancel();
            if self.cancellation.ignore_user_abort() {
                Ok(())
            } else {
                Err(OutputDeliveryError::new(
                    "PHP response body receiver closed",
                ))
            }
        } else {
            self.metrics
                .php_output_chunks
                .fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
}

type PhpResponseBridge = (
    OutputSinkHandle,
    oneshot::Receiver<PhpResponseHead>,
    mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
    Option<mpsc::Sender<Result<Vec<u8>, std::io::Error>>>,
    RuntimeCancellationState,
);

fn php_response_bridge(
    is_head: bool,
    metrics: Arc<super::metrics::ServerMetrics>,
) -> PhpResponseBridge {
    let (head_sender, head_receiver) = oneshot::channel();
    let (chunk_sender, chunk_receiver) = mpsc::channel(PHP_OUTPUT_QUEUE_CAPACITY);
    let cancellation = RuntimeCancellationState::new();
    let sink = PhpResponseSink {
        head: Mutex::new(Some(head_sender)),
        chunks: (!is_head).then_some(chunk_sender),
        suppress_body: std::sync::atomic::AtomicBool::new(is_head),
        cancellation: cancellation.clone(),
        metrics,
        defer_head_until_finish: is_head,
    };
    let failure_sender = sink.chunks.clone();
    (
        OutputSinkHandle::new(sink),
        head_receiver,
        chunk_receiver,
        failure_sender,
        cancellation,
    )
}

#[derive(Clone)]
struct DeferredRouterSink {
    state: Arc<Mutex<DeferredRouterState>>,
    suppress_body: bool,
}

struct DeferredRouterState {
    head: Option<PhpResponseHead>,
    storage: DeferredRouterStorage,
    produced_bytes: u64,
}

enum DeferredRouterStorage {
    Memory(Vec<u8>),
    File(DeferredRouterFile),
    Taken,
}

struct DeferredRouterFile {
    file: Option<File>,
    path: PathBuf,
}

impl Drop for DeferredRouterFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

enum DeferredRouterBody {
    Memory(Vec<u8>),
    File(File),
}

struct DeferredRouterOutput {
    head: PhpResponseHead,
    body: DeferredRouterBody,
}

impl DeferredRouterSink {
    fn new(suppress_body: bool) -> Self {
        Self {
            state: Arc::new(Mutex::new(DeferredRouterState {
                head: None,
                storage: DeferredRouterStorage::Memory(Vec::new()),
                produced_bytes: 0,
            })),
            suppress_body,
        }
    }

    fn take_output(&self) -> Result<DeferredRouterOutput, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "router output spool lock poisoned".to_owned())?;
        let mut head = state
            .head
            .take()
            .ok_or_else(|| "router response head was not committed".to_owned())?;
        head.complete_length = Some(state.produced_bytes);
        let storage = std::mem::replace(&mut state.storage, DeferredRouterStorage::Taken);
        let body = match storage {
            DeferredRouterStorage::Memory(bytes) => DeferredRouterBody::Memory(bytes),
            DeferredRouterStorage::File(mut spool) => {
                let mut file = spool
                    .file
                    .take()
                    .ok_or_else(|| "router output spool file was already consumed".to_owned())?;
                file.flush()
                    .and_then(|_| file.seek(SeekFrom::Start(0)))
                    .map_err(|error| format!("failed to rewind router output spool: {error}"))?;
                // The open handle remains readable on the server's supported
                // Unix platforms; unlinking now guarantees cleanup on normal
                // completion, transport abort, or response drop.
                std::fs::remove_file(&spool.path)
                    .map_err(|error| format!("failed to unlink router output spool: {error}"))?;
                DeferredRouterBody::File(file)
            }
            DeferredRouterStorage::Taken => {
                return Err("router output spool was already consumed".to_owned());
            }
        };
        Ok(DeferredRouterOutput { head, body })
    }

    fn create_spool_file() -> Result<DeferredRouterFile, std::io::Error> {
        for _ in 0..32 {
            let sequence = ROUTER_SPOOL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "phrust-router-{}-{sequence}.spool",
                std::process::id()
            ));
            match OpenOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    return Ok(DeferredRouterFile {
                        file: Some(file),
                        path,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate a unique router output spool",
        ))
    }
}

impl OutputSink for DeferredRouterSink {
    fn commit(
        &self,
        response: &RuntimeHttpResponseState,
        complete_length: Option<u64>,
    ) -> Result<(), OutputDeliveryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| OutputDeliveryError::new("router output spool lock poisoned"))?;
        if state.head.is_none() {
            state.head = Some(PhpResponseHead {
                response: response.clone(),
                complete_length,
            });
        }
        Ok(())
    }

    fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError> {
        if chunk.is_empty() {
            return Ok(());
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| OutputDeliveryError::new("router output spool lock poisoned"))?;
        state.produced_bytes = state.produced_bytes.saturating_add(chunk.len() as u64);
        if self.suppress_body {
            return Ok(());
        }
        match &mut state.storage {
            DeferredRouterStorage::Memory(bytes)
                if bytes.len().saturating_add(chunk.len()) <= ROUTER_MEMORY_SPOOL_BYTES =>
            {
                bytes.extend_from_slice(&chunk);
                Ok(())
            }
            DeferredRouterStorage::Memory(bytes) => {
                let mut spool = Self::create_spool_file().map_err(|error| {
                    OutputDeliveryError::new(format!(
                        "failed to create router output spool: {error}"
                    ))
                })?;
                let file = spool.file.as_mut().expect("new router spool owns a file");
                file.write_all(bytes)
                    .and_then(|_| file.write_all(&chunk))
                    .map_err(|error| {
                        OutputDeliveryError::new(format!(
                            "failed to write router output spool: {error}"
                        ))
                    })?;
                state.storage = DeferredRouterStorage::File(spool);
                Ok(())
            }
            DeferredRouterStorage::File(spool) => spool
                .file
                .as_mut()
                .ok_or_else(|| OutputDeliveryError::new("router output spool file missing"))?
                .write_all(&chunk)
                .map_err(|error| {
                    OutputDeliveryError::new(format!(
                        "failed to write router output spool: {error}"
                    ))
                }),
            DeferredRouterStorage::Taken => Err(OutputDeliveryError::new(
                "router output spool was already consumed",
            )),
        }
    }
}

fn deferred_router_response(output: DeferredRouterOutput) -> Response<ResponseBody> {
    let status = StatusCode::from_u16(output.head.response.status_code).unwrap_or(StatusCode::OK);
    let suppress_body = status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED;
    let body = if suppress_body {
        response::full_body(Bytes::new())
    } else {
        match output.body {
            DeferredRouterBody::Memory(bytes) => response::full_body(Bytes::from(bytes)),
            DeferredRouterBody::File(file) => response::reader_body_with_length(
                tokio::fs::File::from_std(file),
                output.head.complete_length.unwrap_or_default(),
            ),
        }
    };
    let mut response = Response::builder()
        .status(status)
        .body(body)
        .expect("deferred router response builder is valid");
    apply_php_headers(response.headers_mut(), &output.head.response);
    if let Some(content_length) = output.head.complete_length {
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_length.to_string())
                .expect("content length header is valid"),
        );
    }
    response
}

struct PhpExecutionCompletionGuard {
    coordinator: PhpExecutionCoordinator,
    failure_sender: Option<mpsc::Sender<Result<Vec<u8>, std::io::Error>>>,
    metrics: Arc<super::metrics::ServerMetrics>,
    fallback_trace: Option<PerfTraceEvent>,
    completed: bool,
}

enum PhpWorkerRequestResult {
    Response(Response<ResponseBody>, Option<bool>, PhpTransferCompletion),
    Streamed(Option<bool>, PhpTransferCompletion),
}

enum PhpWorkerReply {
    Response(Response<ResponseBody>, Option<bool>),
    Streamed(Option<bool>),
}

fn worker_response_result(
    mut result: (Response<ResponseBody>, Option<bool>),
) -> PhpWorkerRequestResult {
    let completion = result
        .0
        .extensions_mut()
        .remove::<PhpTransferCompletion>()
        .unwrap_or(PhpTransferCompletion {
            trace: None,
            failure_stage: Some(RequestStage::Execution),
        });
    PhpWorkerRequestResult::Response(result.0, result.1, completion)
}

impl PhpExecutionCompletionGuard {
    fn new(
        coordinator: PhpExecutionCoordinator,
        failure_sender: Option<mpsc::Sender<Result<Vec<u8>, std::io::Error>>>,
        metrics: Arc<super::metrics::ServerMetrics>,
        fallback_trace: Option<PerfTraceEvent>,
    ) -> Self {
        Self {
            coordinator,
            failure_sender,
            metrics,
            fallback_trace,
            completed: false,
        }
    }

    fn complete(&mut self, completion: PhpTransferCompletion) {
        self.completed = true;
        self.failure_sender.take();
        self.fallback_trace.take();
        self.coordinator.complete(completion);
    }
}

impl Drop for PhpExecutionCompletionGuard {
    fn drop(&mut self) {
        if !self.completed {
            self.metrics
                .worker_pool_failures
                .fetch_add(1, Ordering::Relaxed);
            if let Some(sender) = self.failure_sender.take() {
                let _ = sender.blocking_send(Err(std::io::Error::other(
                    "PHP worker terminated unexpectedly",
                )));
            }
            self.coordinator.complete(PhpTransferCompletion {
                trace: self.fallback_trace.take(),
                failure_stage: Some(RequestStage::Execution),
            });
        }
    }
}

thread_local! {
    static REQUEST_EXECUTOR_CACHE: RefCell<Option<CachedRequestExecutor>> = const { RefCell::new(None) };
}

struct CachedRequestExecutor {
    key: RequestExecutorCacheKey,
    executor: PhpExecutor,
}

struct RequestTarget {
    script_path: PathBuf,
    path_info: Option<String>,
}

fn route_target_selection_stage(script_path: PathBuf, path_info: Option<String>) -> RequestTarget {
    RequestTarget {
        script_path,
        path_info,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RequestLocalAddr(pub(crate) SocketAddr);

pub(crate) async fn execute_php_request(
    request: PartsAndBody,
    state: Arc<AppState>,
    script_path: PathBuf,
    path_info: Option<String>,
    peer: SocketAddr,
    request_id: String,
    route_resolution: Duration,
) -> (Response<ResponseBody>, Option<bool>) {
    let RequestTarget {
        script_path,
        path_info,
    } = route_target_selection_stage(script_path, path_info);
    let PartsAndBody { parts, body } = request;
    // Trace events only reach the perf-trace/request-profile writers; skip
    // the per-request string clones and phase vector entirely when neither
    // consumer is configured.
    let collect_request_trace =
        state.observability.perf_trace.is_some() || state.observability.request_profile.is_some();
    let mut trace = collect_request_trace.then(|| PerfTraceEvent {
        request_id: request_id.clone(),
        method: parts.method.to_string(),
        path: parts
            .uri
            .path_and_query()
            .map_or_else(|| parts.uri.path().to_string(), |value| value.to_string()),
        script_path: script_path.display().to_string(),
        phases: vec![("route_resolution", route_resolution.as_nanos())],
        ..PerfTraceEvent::default()
    });
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
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
    let prepared = match body_and_multipart_stage(&state, &parts, body).await {
        Err(_) => {
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_BODY_READ_TIMEOUT",
                "body_read",
                "request body read timed out",
                || {
                    BTreeMap::from([(
                        "timeout_ms".to_string(),
                        state.request.request_timeout.as_millis().to_string(),
                    )])
                },
            );
            record_phase(
                &state,
                &mut trace,
                RequestPhase::BodyRead,
                "body_read",
                body_started.elapsed(),
            );
            let response = response::text(StatusCode::REQUEST_TIMEOUT, "request timeout\n");
            return finish_php_request(
                &state,
                trace,
                response,
                None,
                Some(RequestStage::BodyAndMultipart),
            );
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
                Some(&request_id),
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
            record_phase(
                &state,
                &mut trace,
                RequestPhase::BodyRead,
                "body_read",
                body_started.elapsed(),
            );
            let response = response::text(StatusCode::PAYLOAD_TOO_LARGE, "payload too large\n");
            return finish_php_request(
                &state,
                trace,
                response,
                None,
                Some(RequestStage::BodyAndMultipart),
            );
        }
        Ok(Err(BodyReadError::Invalid)) => {
            let script_filename = script_path.display().to_string();
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_BODY_INVALID",
                "body_read",
                "request body read failed",
                BTreeMap::new,
            );
            emit_request_diagnostic(
                &state,
                &parts,
                Some(&request_id),
                RequestDiagnostic::new(
                    "E_PHP_REQUEST_BODY_PARSE_FAILED",
                    "body_read",
                    "server could not read the request body",
                    "request_body_ingest",
                    parts.uri.path(),
                    &script_filename,
                ),
            );
            warn!(%peer, "failed to read request body");
            record_phase(
                &state,
                &mut trace,
                RequestPhase::BodyRead,
                "body_read",
                body_started.elapsed(),
            );
            let response = response::text(StatusCode::BAD_REQUEST, "bad request\n");
            return finish_php_request(
                &state,
                trace,
                response,
                None,
                Some(RequestStage::BodyAndMultipart),
            );
        }
        Ok(Err(BodyReadError::Internal)) => {
            warn!(%peer, "request body temporary storage failed");
            record_phase(
                &state,
                &mut trace,
                RequestPhase::BodyRead,
                "body_read",
                body_started.elapsed(),
            );
            return finish_php_request(
                &state,
                trace,
                response::text(StatusCode::INTERNAL_SERVER_ERROR, "body storage failed\n"),
                None,
                Some(RequestStage::BodyAndMultipart),
            );
        }
    };
    let PreparedRequestData { body, parsed } = prepared;
    record_phase(
        &state,
        &mut trace,
        RequestPhase::BodyRead,
        "body_read",
        body_started.elapsed(),
    );
    if let Some(trace) = trace.as_mut() {
        trace.body_bytes = body.len();
    }
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_BODY_READ_END",
        "body_read",
        "request body read completed",
        || BTreeMap::from([("body_bytes".to_string(), body.len().to_string())]),
    );
    if let Some(response) = execute_builtin_router_if_configured(
        &parts,
        Arc::clone(&state),
        body.clone(),
        parsed.clone(),
        peer,
        &request_id,
        Some(&script_path),
    )
    .await
    {
        return finish_php_request(
            &state,
            trace,
            response,
            None,
            Some(RequestStage::RouteTargetSelection),
        );
    }
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_SCRIPT_RESOLVED",
        "routing",
        "PHP script resolved",
        || {
            BTreeMap::from([
                ("script_path".to_string(), script_path.display().to_string()),
                (
                    "path_info".to_string(),
                    path_info.clone().unwrap_or_default(),
                ),
            ])
        },
    );
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_SCRIPT_CACHE_START",
        "cache",
        "script cache lookup started",
        || BTreeMap::from([("script_path".to_string(), script_path.display().to_string())]),
    );
    // Cache-stat deltas only surface through the perf-trace/request-profile
    // writers; snapshotting locks every script-cache shard, so skip it when
    // no trace consumer is configured.
    let script_cache_before =
        collect_request_trace.then(|| state.services.engine.script_cache.cache_stats());
    let script_cache_started = Instant::now();
    let lookup = match executor_acquisition_stage(&state, &script_path) {
        Ok(lookup) => {
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_SCRIPT_CACHE_END",
                "cache",
                "script cache lookup completed",
                || {
                    BTreeMap::from([
                        ("script_path".to_string(), script_path.display().to_string()),
                        ("cache_hit".to_string(), lookup.hit.to_string()),
                    ])
                },
            );
            debug!(script=%script_path.display(), hit=lookup.hit, "compiled script cache lookup");
            lookup
        }
        Err(PhpExecutionError::Compile(output)) => {
            log_php_execution_failure(&script_path, &output);
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_SCRIPT_CACHE_ERROR",
                "cache",
                "script compile failed",
                || {
                    BTreeMap::from([
                        ("script_path".to_string(), script_path.display().to_string()),
                        (
                            "diagnostic_text_bytes".to_string(),
                            output.diagnostics_text.len().to_string(),
                        ),
                    ])
                },
            );
            let response = php_compile_error_response(*output, parts.method == Method::HEAD);
            return finish_php_request(
                &state,
                trace,
                response,
                None,
                Some(RequestStage::ExecutorAcquisition),
            );
        }
        Err(PhpExecutionError::Engine(_)) => {
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_SCRIPT_CACHE_ERROR",
                "cache",
                "script compile engine error",
                || BTreeMap::from([("script_path".to_string(), script_path.display().to_string())]),
            );
            warn!(script=%script_path.display(), "php execution engine error");
            let response =
                response::text(StatusCode::INTERNAL_SERVER_ERROR, "php execution failed\n");
            return finish_php_request(
                &state,
                trace,
                response,
                None,
                Some(RequestStage::ExecutorAcquisition),
            );
        }
    };
    record_phase(
        &state,
        &mut trace,
        RequestPhase::ScriptCache,
        "script_cache_lookup",
        script_cache_started.elapsed(),
    );
    if let Some((script_cache_before, trace)) = script_cache_before.as_ref().zip(trace.as_mut()) {
        let script_cache_after = state.services.engine.script_cache.cache_stats();
        trace.counters.extend([
            (
                "entry_script_cache_hits",
                script_cache_after
                    .hits
                    .saturating_sub(script_cache_before.hits),
            ),
            (
                "entry_script_cache_misses",
                script_cache_after
                    .misses
                    .saturating_sub(script_cache_before.misses),
            ),
            (
                "entry_script_source_reads",
                script_cache_after
                    .source_reads
                    .saturating_sub(script_cache_before.source_reads),
            ),
        ]);
    }
    let script_cache_hit = Some(lookup.hit);
    let Some(cpu_permit) = acquire_cpu_execution_permit(&state).await else {
        let response = response::text(
            StatusCode::SERVICE_UNAVAILABLE,
            "PHP execution queue full\n",
        );
        return finish_php_request(
            &state,
            trace,
            response,
            script_cache_hit,
            Some(RequestStage::ExecutorAcquisition),
        );
    };
    let is_head = parts.method == Method::HEAD;
    let (output_sink, mut head_receiver, body_receiver, failure_sender, cancellation) =
        php_response_bridge(is_head, Arc::clone(&state.services.metrics));
    let coordinator = PhpExecutionCoordinator::new();
    let worker_coordinator = coordinator.clone();
    let worker_cancellation = cancellation.clone();
    let worker_state = Arc::clone(&state);
    let workers = Arc::clone(&state.concurrency.php_workers);
    let submission_trace = trace.clone();
    let panic_trace = trace.clone();
    let mut worker_reply = match workers
        .submit(move || {
            let mut completion_guard = PhpExecutionCompletionGuard::new(
                worker_coordinator.clone(),
                failure_sender,
                Arc::clone(&worker_state.services.metrics),
                panic_trace,
            );
            let result = run_php_request_on_worker(
                worker_state,
                parts,
                body,
                parsed,
                script_path,
                path_info,
                peer,
                request_id,
                trace,
                lookup,
                script_cache_hit,
                cpu_permit,
                output_sink,
                worker_cancellation,
            );
            match result {
                PhpWorkerRequestResult::Response(mut response, cache_hit, completion) => {
                    completion_guard.complete(completion);
                    response.extensions_mut().insert(worker_coordinator);
                    PhpWorkerReply::Response(response, cache_hit)
                }
                PhpWorkerRequestResult::Streamed(cache_hit, completion) => {
                    completion_guard.complete(completion);
                    PhpWorkerReply::Streamed(cache_hit)
                }
            }
        })
        .await
    {
        Ok(reply) => reply,
        Err(error) => {
            state
                .services
                .metrics
                .worker_pool_failures
                .fetch_add(1, Ordering::Relaxed);
            warn!(%error, "PHP worker submission failed");
            coordinator.complete(PhpTransferCompletion {
                trace: submission_trace,
                failure_stage: Some(RequestStage::Execution),
            });
            let mut response =
                response::text(StatusCode::SERVICE_UNAVAILABLE, "PHP worker unavailable\n");
            response.extensions_mut().insert(coordinator);
            return (response, script_cache_hit);
        }
    };

    tokio::select! {
        head = &mut head_receiver => {
            match head {
                Ok(head) => {
                    let response_started = Instant::now();
                    let mut response = php_streaming_response(
                        head,
                        is_head,
                        body_receiver,
                        cancellation,
                    );
                    state.services.metrics.record_phase(
                        RequestPhase::ResponseBuild,
                        response_started.elapsed().as_nanos(),
                    );
                    response.extensions_mut().insert(coordinator);
                    (response, script_cache_hit)
                }
                Err(_) => worker_result_response(
                    worker_reply.await,
                    coordinator,
                    script_cache_hit,
                    &state,
                ),
            }
        }
        result = &mut worker_reply => {
            match head_receiver.try_recv() {
                Ok(head) => {
                    let response_started = Instant::now();
                    let mut response = php_streaming_response(
                        head,
                        is_head,
                        body_receiver,
                        cancellation,
                    );
                    state.services.metrics.record_phase(
                        RequestPhase::ResponseBuild,
                        response_started.elapsed().as_nanos(),
                    );
                    response.extensions_mut().insert(coordinator);
                    (response, script_cache_hit)
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    worker_result_response(result, coordinator, script_cache_hit, &state)
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    worker_result_response(result, coordinator, script_cache_hit, &state)
                }
            }
        }
    }
}

fn worker_result_response(
    result: Result<
        Result<PhpWorkerReply, crate::worker_pool::WorkerPoolError>,
        tokio::sync::oneshot::error::RecvError,
    >,
    coordinator: PhpExecutionCoordinator,
    cache_hit: Option<bool>,
    state: &AppState,
) -> (Response<ResponseBody>, Option<bool>) {
    match result {
        Ok(Ok(PhpWorkerReply::Response(response, cache_hit))) => (response, cache_hit),
        Ok(Ok(PhpWorkerReply::Streamed(cache_hit))) => {
            warn!("PHP worker completed streaming without committing a response head");
            let mut response = response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "PHP response head unavailable\n",
            );
            response.extensions_mut().insert(coordinator);
            (response, cache_hit)
        }
        Ok(Err(error)) => {
            state
                .services
                .metrics
                .worker_pool_failures
                .fetch_add(1, Ordering::Relaxed);
            warn!(%error, "PHP worker execution failed");
            let mut response =
                response::text(StatusCode::SERVICE_UNAVAILABLE, "PHP worker unavailable\n");
            response.extensions_mut().insert(coordinator);
            (response, cache_hit)
        }
        Err(error) => {
            state
                .services
                .metrics
                .worker_pool_failures
                .fetch_add(1, Ordering::Relaxed);
            warn!(%error, "PHP worker reply dropped");
            let mut response =
                response::text(StatusCode::SERVICE_UNAVAILABLE, "PHP worker unavailable\n");
            response.extensions_mut().insert(coordinator);
            (response, cache_hit)
        }
    }
}

/// Synchronous request core: builds the PHP runtime context, parses
/// multipart bodies, seeds the session, executes the compiled script, and
/// renders the HTTP response. Everything execution-side (`RuntimeContext`,
/// `PhpExecutionOutput`) is created and consumed here, so this function can
/// run unchanged on a dedicated worker thread — its inputs and its return
/// value are the `Send` payload set pinned by `worker_payload_tests`.
#[allow(clippy::too_many_arguments)]
fn run_php_request_on_worker(
    state: Arc<AppState>,
    parts: Parts,
    body: RuntimeRequestBody,
    parsed: ParsedRequestData,
    script_path: PathBuf,
    path_info: Option<String>,
    peer: SocketAddr,
    request_id: String,
    mut trace: Option<PerfTraceEvent>,
    lookup: CompiledScriptCacheLookup,
    script_cache_hit: Option<bool>,
    cpu_permit: OwnedSemaphorePermit,
    output_sink: OutputSinkHandle,
    cancellation: RuntimeCancellationState,
) -> PhpWorkerRequestResult {
    let collect_request_trace =
        state.observability.perf_trace.is_some() || state.observability.request_profile.is_some();
    let script_name = script_name_for(&state.route_config.docroot, &script_path);
    let request_context_started = Instant::now();
    let mut request_context = request_globals_stage(
        &parts,
        &state,
        &script_path,
        &script_name,
        path_info,
        body.clone(),
        peer,
    );
    request_context
        .startup_warnings
        .extend(parsed.startup_warnings);
    request_context.parsed_post.extend(parsed.post);
    request_context.uploaded_files.extend(parsed.files);
    if parsed.stats.parts_total > 0 {
        emit_server_debug_lazy(
            &state,
            Some(&request_id),
            "D_PHRUST_SERVER_MULTIPART_PARSED",
            "multipart",
            "multipart body parsed",
            || {
                BTreeMap::from([
                    ("parts".to_string(), parsed.stats.parts_total.to_string()),
                    (
                        "upload_count".to_string(),
                        parsed.stats.uploads_total.to_string(),
                    ),
                    (
                        "upload_bytes".to_string(),
                        parsed.stats.upload_bytes_accepted.to_string(),
                    ),
                    (
                        "post_limit_exceeded".to_string(),
                        parsed.post_limit_exceeded.to_string(),
                    ),
                ])
            },
        );
    }
    record_phase(
        &state,
        &mut trace,
        RequestPhase::RequestContext,
        "request_context",
        request_context_started.elapsed(),
    );
    let session_callbacks = SessionRequestCallbacks::new(&state, cancellation.clone());
    let cleanup = RequestCleanup::new(parsed.uploads, Some(session_callbacks.clone()));
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_SESSION_SEED_START",
        "session",
        "session seed started",
        || {
            BTreeMap::from([(
                "sessions_enabled".to_string(),
                state.sessions.config.enabled.to_string(),
            )])
        },
    );
    let session_seed_started = Instant::now();
    let session_state = match session_load_stage(&request_context, &state) {
        Ok(session) => session,
        Err(error) => {
            emit_request_diagnostic(
                &state,
                &parts,
                Some(&request_id),
                RequestDiagnostic::new(
                    "E_PHP_SESSION_STORE_UNAVAILABLE",
                    "session",
                    "server session store failed while preparing request state",
                    "seed_session_state",
                    parts.uri.path(),
                    &script_path.display().to_string(),
                ),
            );
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_SESSION_ERROR",
                "session",
                "session seed failed",
                || BTreeMap::from([("error".to_string(), error.clone())]),
            );
            warn!(%peer, error=%error, "session state preparation failed");
            let response = response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "session storage failed\n",
            );
            return worker_response_result(finish_php_request(
                &state,
                trace,
                response,
                script_cache_hit,
                Some(RequestStage::SessionLoad),
            ));
        }
    };
    record_phase(
        &state,
        &mut trace,
        RequestPhase::SessionSeed,
        "session_seed",
        session_seed_started.elapsed(),
    );
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_SESSION_SEED_END",
        "session",
        "session seed completed",
        || {
            BTreeMap::from([(
                "session_active".to_string(),
                (!session_state.id().is_empty()).to_string(),
            )])
        },
    );
    let mut runtime_context = php_runtime_context_for_http(
        &state,
        request_context,
        session_state,
        server_env_for_request(&state),
        &session_callbacks,
    );
    runtime_context = runtime_context.with_output_sink(output_sink);
    runtime_context = runtime_context.with_cancellation(cancellation);
    if state.request.execution_time_limit.is_none() {
        state
            .services
            .metrics
            .execution_deadline_disabled
            .fetch_add(1, Ordering::Relaxed);
    }
    let is_head = parts.method == Method::HEAD;
    let script_log_path = script_path.clone();
    let execution_started = Instant::now();
    emit_server_debug_lazy(
        &state,
        Some(&request_id),
        "D_PHRUST_SERVER_EXECUTE_START",
        "execute",
        "PHP execution started",
        || {
            BTreeMap::from([(
                "script_path".to_string(),
                script_log_path.display().to_string(),
            )])
        },
    );
    let include_cache_before =
        collect_request_trace.then(|| state.services.engine.include_cache.cache_stats());
    let profile_requested = request_profile_requested(&state, &parts.headers);
    let result = execution_stage(
        Arc::clone(&state),
        lookup,
        script_path,
        runtime_context,
        profile_requested,
    );
    drop(cpu_permit);
    record_phase(
        &state,
        &mut trace,
        RequestPhase::VmExecution,
        "php_vm_execution",
        execution_started.elapsed(),
    );
    if let Some((include_cache_before, trace)) = include_cache_before.as_ref().zip(trace.as_mut()) {
        let include_cache_after = state.services.engine.include_cache.cache_stats();
        trace.counters.extend([
            (
                "include_resolution_hits",
                include_cache_after
                    .resolution_hits
                    .saturating_sub(include_cache_before.resolution_hits),
            ),
            (
                "include_resolution_misses",
                include_cache_after
                    .resolution_misses
                    .saturating_sub(include_cache_before.resolution_misses),
            ),
            (
                "include_compile_hits",
                include_cache_after
                    .compile_hits
                    .saturating_sub(include_cache_before.compile_hits),
            ),
            (
                "include_compile_misses",
                include_cache_after
                    .compile_misses
                    .saturating_sub(include_cache_before.compile_misses),
            ),
            (
                "include_source_reads",
                include_cache_after
                    .source_reads
                    .saturating_sub(include_cache_before.source_reads),
            ),
            (
                "include_source_bytes_hashed",
                include_cache_after
                    .source_bytes_hashed
                    .saturating_sub(include_cache_before.source_bytes_hashed),
            ),
            (
                "include_content_validations",
                include_cache_after
                    .content_validations
                    .saturating_sub(include_cache_before.content_validations),
            ),
            (
                "include_identity_only_hits",
                include_cache_after
                    .identity_only_hits
                    .saturating_sub(include_cache_before.identity_only_hits),
            ),
            (
                "include_content_mismatches",
                include_cache_after
                    .content_mismatches
                    .saturating_sub(include_cache_before.content_mismatches),
            ),
            (
                "include_conservative_misses",
                include_cache_after
                    .conservative_misses
                    .saturating_sub(include_cache_before.conservative_misses),
            ),
        ]);
    }
    match result {
        Ok(mut output) => {
            if let Some(trace) = trace.as_mut() {
                append_vm_counters_to_trace(trace, output.counters.as_ref());
                if state.observability.request_profile.is_some() {
                    trace.profile_counters = output.counters.clone();
                }
                trace.profile_requested = profile_requested;
            }
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_EXECUTE_END",
                "execute",
                "PHP execution completed",
                || {
                    let mut execute_end_context = BTreeMap::from([
                        ("status".to_string(), format!("{:?}", output.status)),
                        (
                            "duration_ms".to_string(),
                            execution_started.elapsed().as_millis().to_string(),
                        ),
                        (
                            "runtime_diagnostic_count".to_string(),
                            output.runtime_diagnostics.len().to_string(),
                        ),
                    ]);
                    if !output.runtime_diagnostics.is_empty() {
                        execute_end_context.insert(
                            "runtime_diagnostic_codes".to_string(),
                            output
                                .runtime_diagnostics
                                .iter()
                                .map(|diagnostic| diagnostic.id())
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                        execute_end_context.insert(
                            "runtime_diagnostic_samples".to_string(),
                            runtime_diagnostic_samples(&output),
                        );
                    }
                    execute_end_context
                },
            );
            let session_finalize_started = Instant::now();
            if let Err(error) = cleanup.finalize_output(&mut output, &state) {
                emit_request_diagnostic(
                    &state,
                    &parts,
                    Some(&request_id),
                    RequestDiagnostic::new(
                        "E_PHP_SESSION_STORE_UNAVAILABLE",
                        "session",
                        "server session store failed while finalizing request state",
                        "finalize_session_state",
                        parts.uri.path(),
                        &script_log_path.display().to_string(),
                    ),
                );
                emit_server_debug_lazy(
                    &state,
                    Some(&request_id),
                    "D_PHRUST_SERVER_SESSION_ERROR",
                    "session",
                    "session finalization failed",
                    || BTreeMap::from([("error".to_string(), error.clone())]),
                );
                warn!(%peer, error=%error, "session state finalization failed");
                return PhpWorkerRequestResult::Streamed(
                    script_cache_hit,
                    PhpTransferCompletion {
                        trace,
                        failure_stage: Some(RequestStage::SessionAndUploadCleanup),
                    },
                );
            }
            record_phase(
                &state,
                &mut trace,
                RequestPhase::SessionFinalize,
                "session_finalize",
                session_finalize_started.elapsed(),
            );
            let runtime_diagnostics = output.runtime_diagnostics.len() as u64;
            if let Some(trace) = trace.as_mut() {
                trace.runtime_diagnostics = runtime_diagnostics;
            }
            state
                .services
                .metrics
                .runtime_diagnostics
                .fetch_add(runtime_diagnostics, Ordering::Relaxed);
            if php_execution_timed_out(&output) {
                state
                    .services
                    .metrics
                    .execution_timeouts
                    .fetch_add(1, Ordering::Relaxed);
                return PhpWorkerRequestResult::Streamed(
                    script_cache_hit,
                    PhpTransferCompletion {
                        trace,
                        failure_stage: Some(RequestStage::Execution),
                    },
                );
            }
            log_php_execution_failure(&script_log_path, &output);
            PhpWorkerRequestResult::Streamed(
                script_cache_hit,
                PhpTransferCompletion {
                    trace,
                    failure_stage: (output.status != PhpExecutionStatus::Success)
                        .then_some(RequestStage::Execution),
                },
            )
        }
        Err(PhpExecutionError::Compile(output)) => {
            log_php_execution_failure(&script_log_path, &output);
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_EXECUTE_END",
                "execute",
                "PHP execution produced compile diagnostics",
                || {
                    BTreeMap::from([
                        ("status".to_string(), "CompileError".to_string()),
                        (
                            "duration_ms".to_string(),
                            execution_started.elapsed().as_millis().to_string(),
                        ),
                        (
                            "diagnostic_text_bytes".to_string(),
                            output.diagnostics_text.len().to_string(),
                        ),
                    ])
                },
            );
            let response = php_compile_error_response(*output, is_head);
            worker_response_result(finish_php_request(
                &state,
                trace,
                response,
                script_cache_hit,
                Some(RequestStage::Execution),
            ))
        }
        Err(PhpExecutionError::Engine(error)) => {
            emit_server_debug_lazy(
                &state,
                Some(&request_id),
                "D_PHRUST_SERVER_EXECUTE_END",
                "execute",
                "PHP execution engine error",
                || {
                    BTreeMap::from([
                        ("status".to_string(), "EngineError".to_string()),
                        (
                            "duration_ms".to_string(),
                            execution_started.elapsed().as_millis().to_string(),
                        ),
                        ("error".to_string(), error.to_string()),
                    ])
                },
            );
            warn!(script=%script_log_path.display(), %error, "php execution engine error");
            let response =
                response::text(StatusCode::INTERNAL_SERVER_ERROR, "php execution failed\n");
            worker_response_result(finish_php_request(
                &state,
                trace,
                response,
                script_cache_hit,
                Some(RequestStage::Execution),
            ))
        }
    }
}

pub(crate) async fn execute_builtin_router_if_configured(
    parts: &Parts,
    state: Arc<AppState>,
    body: RuntimeRequestBody,
    parsed: ParsedRequestData,
    peer: SocketAddr,
    request_id: &str,
    target_script_path: Option<&Path>,
) -> Option<Response<ResponseBody>> {
    let router = state.route_config.builtin_router.as_ref()?;
    let router_path = state.route_config.docroot.join(router);
    let Ok(router_path) = router_path.canonicalize() else {
        return Some(response::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            "router script not found\n",
        ));
    };
    if !router_path.starts_with(&state.route_config.docroot) {
        return Some(response::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            "router script outside document root\n",
        ));
    }
    if target_script_path.is_some_and(|target| router_path == target) {
        return None;
    }
    let Some(cpu_permit) = acquire_cpu_execution_permit(&state).await else {
        return Some(response::text(
            StatusCode::SERVICE_UNAVAILABLE,
            "PHP execution queue full\n",
        ));
    };
    let script_name = script_name_for(&state.route_config.docroot, &router_path);
    let mut request_context = http_runtime_context(
        parts,
        &state,
        &router_path,
        &script_name,
        None,
        body.clone(),
        peer,
    );
    request_context
        .startup_warnings
        .extend(parsed.startup_warnings);
    request_context.parsed_post.extend(parsed.post);
    request_context.uploaded_files.extend(parsed.files);
    let router_uploads = parsed.uploads;
    let router_env = server_env_for_request(&state);
    let lookup = match executor_acquisition_stage(&state, &router_path) {
        Ok(lookup) => lookup,
        Err(PhpExecutionError::Compile(output)) => {
            return Some(php_compile_error_response(*output, false));
        }
        Err(PhpExecutionError::Engine(error)) => {
            warn!(script=%router_path.display(), %error, "router compile engine error");
            return Some(response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "router execution failed\n",
            ));
        }
    };
    let execution_state = Arc::clone(&state);
    let runtime_state = Arc::clone(&state);
    let execution_path = router_path.clone();
    let is_head = parts.method == Method::HEAD;
    let workers = Arc::clone(&state.concurrency.php_workers);
    let router_response = match workers
        .execute(move || {
            let router_cancellation = RuntimeCancellationState::new();
            let session_callbacks =
                SessionRequestCallbacks::new(&runtime_state, router_cancellation.clone());
            let _cleanup = RequestCleanup::new(router_uploads, Some(session_callbacks.clone()));
            let deferred_sink = DeferredRouterSink::new(is_head);
            let result = session_load_stage(&request_context, &runtime_state)
                .map_err(|error| format!("router session state preparation failed: {error}"))
                .and_then(|session_state| {
                    let runtime_context = php_runtime_context_for_http(
                        &runtime_state,
                        request_context,
                        session_state,
                        router_env,
                        &session_callbacks,
                    )
                    .with_output_sink(OutputSinkHandle::new(deferred_sink.clone()))
                    .with_cancellation(router_cancellation);
                    execution_stage(
                        execution_state,
                        lookup,
                        execution_path,
                        runtime_context,
                        true,
                    )
                    .map_err(|error| format!("{error:?}"))
                });
            let result = match result {
                Ok(output) if matches!(output.return_value, Some(Value::Bool(false))) => Ok(None),
                Ok(_) => deferred_sink
                    .take_output()
                    .map(deferred_router_response)
                    .map(Some),
                Err(error) => Err(error),
            };
            drop(cpu_permit);
            result
        })
        .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            warn!(script=%router_path.display(), error=?error, "router execution engine error");
            return Some(response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "router execution failed\n",
            ));
        }
        Err(error) => {
            state
                .services
                .metrics
                .worker_pool_failures
                .fetch_add(1, Ordering::Relaxed);
            warn!(script=%router_path.display(), %error, "router PHP worker unavailable");
            return Some(response::text(
                StatusCode::SERVICE_UNAVAILABLE,
                "PHP worker unavailable\n",
            ));
        }
    };
    emit_server_debug_lazy(
        &state,
        Some(request_id),
        "D_PHRUST_SERVER_BUILTIN_ROUTER_END",
        "routing",
        "built-in router executed",
        || {
            BTreeMap::from([(
                "fallthrough".to_string(),
                router_response.is_none().to_string(),
            )])
        },
    );
    router_response
}

struct CpuQueueCancellationGuard<'a> {
    metrics: &'a super::metrics::ServerMetrics,
    completed: bool,
}

impl Drop for CpuQueueCancellationGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.metrics
                .cpu_execution_cancelled
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

async fn acquire_cpu_execution_permit(state: &AppState) -> Option<OwnedSemaphorePermit> {
    let started = Instant::now();
    match Arc::clone(&state.concurrency.cpu_execution).try_acquire_owned() {
        Ok(permit) => {
            state
                .services
                .metrics
                .cpu_execution_admitted
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .record_phase(RequestPhase::CpuQueue, started.elapsed().as_nanos());
            return Some(permit);
        }
        Err(TryAcquireError::Closed) => {
            state
                .services
                .metrics
                .cpu_execution_rejected
                .fetch_add(1, Ordering::Relaxed);
            return None;
        }
        Err(TryAcquireError::NoPermits) => {
            state
                .services
                .metrics
                .cpu_execution_queued
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .cpu_execution_saturated
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    let mut cancellation = CpuQueueCancellationGuard {
        metrics: &state.services.metrics,
        completed: false,
    };
    let permit = timeout(
        state.request.request_timeout,
        Arc::clone(&state.concurrency.cpu_execution).acquire_owned(),
    )
    .await;
    cancellation.completed = true;
    state
        .services
        .metrics
        .record_phase(RequestPhase::CpuQueue, started.elapsed().as_nanos());
    match permit {
        Ok(Ok(permit)) => {
            state
                .services
                .metrics
                .cpu_execution_admitted
                .fetch_add(1, Ordering::Relaxed);
            Some(permit)
        }
        Ok(Err(_)) => {
            state
                .services
                .metrics
                .cpu_execution_rejected
                .fetch_add(1, Ordering::Relaxed);
            None
        }
        Err(_) => {
            state
                .services
                .metrics
                .cpu_execution_timeouts
                .fetch_add(1, Ordering::Relaxed);
            state
                .services
                .metrics
                .cpu_execution_rejected
                .fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

/// Executes a compiled script on its pinned PHP worker thread.
fn executor_acquisition_stage(
    state: &AppState,
    script_path: &Path,
) -> Result<CompiledScriptCacheLookup, PhpExecutionError> {
    state.compile_script(script_path)
}

pub(crate) fn execution_stage(
    state: Arc<AppState>,
    lookup: CompiledScriptCacheLookup,
    script_path: PathBuf,
    runtime_context: RuntimeContext,
    profile_requested: bool,
) -> Result<PhpExecutionOutput, PhpExecutionError> {
    let mode = if profile_requested {
        request_counter_mode(&state)
    } else {
        perf_trace_counter_mode(&state)
    };
    execute_compiled_php_with_state(&state, lookup, script_path, runtime_context, mode)
}

/// Counter mode for requests that did not ask for a profile (only relevant
/// with `--request-profile-trigger-header`); perf-trace VM counters still
/// apply because they are a process-wide policy.
pub(crate) fn perf_trace_counter_mode(state: &AppState) -> RequestCounterMode {
    if state.observability.perf_trace.is_some() && state.observability.perf_trace_vm_counters {
        return RequestCounterMode::VmCounters;
    }
    RequestCounterMode::Off
}

/// True when this request opts into profiling: only header-triggered by
/// default; config/env can explicitly disable the header trigger for
/// profiling every request in controlled benchmark runs.
pub(crate) fn request_profile_requested(state: &AppState, headers: &HeaderMap) -> bool {
    if !state.observability.request_profile_trigger_header {
        return true;
    }
    headers
        .get("x-phrust-request-profile")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "on"))
}

/// How much VM accounting a request pays. `--request-profile` alone stays in
/// `Summary` (phase JSON only); native hot counters are an explicit opt-in
/// because they distort the measured request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RequestCounterMode {
    Off,
    Summary,
    VmCounters,
}

impl RequestCounterMode {
    pub(crate) fn collects_vm_counters(self) -> bool {
        matches!(self, Self::VmCounters)
    }
}

pub(crate) fn request_counter_mode(state: &AppState) -> RequestCounterMode {
    if state.observability.request_profile.is_some()
        && state.observability.request_profile_vm_counters
    {
        return RequestCounterMode::VmCounters;
    }
    if state.observability.perf_trace.is_some() && state.observability.perf_trace_vm_counters {
        return RequestCounterMode::VmCounters;
    }
    if state.observability.request_profile.is_some() {
        return RequestCounterMode::Summary;
    }
    RequestCounterMode::Off
}

pub(crate) fn execute_compiled_php_with_state(
    state: &AppState,
    lookup: CompiledScriptCacheLookup,
    script_path: PathBuf,
    runtime_context: RuntimeContext,
    mode: RequestCounterMode,
) -> Result<PhpExecutionOutput, PhpExecutionError> {
    state
        .services
        .metrics
        .persistent_engine_request_local_resets
        .fetch_add(1, Ordering::Relaxed);
    // PHP-visible state is always rebuilt. The worker retains only explicitly
    // engine-owned plans, constants, builtin/JIT handles, tiering hotness, and
    // guarded adaptive tables; rejecting frames/globals/resources remains visible.
    state
        .services
        .metrics
        .persistent_engine_request_local_rejections
        .fetch_add(1, Ordering::Relaxed);
    state
        .services
        .metrics
        .persistent_engine_policy_reuses
        .fetch_add(1, Ordering::Relaxed);
    let output = execute_compiled_with_request_executor(
        state,
        &lookup.compiled,
        PhpRequestExecutionInput {
            real_path: Some(script_path),
            cwd: state.route_config.docroot.clone(),
            include_roots: include_roots_for_docroot(&state.route_config.docroot),
            runtime_context,
            collect_counters: mode.collects_vm_counters(),
        },
    );
    Ok(output)
}

fn execute_compiled_with_request_executor(
    state: &AppState,
    compiled: &CompiledPhpScript,
    input: PhpRequestExecutionInput,
) -> PhpExecutionOutput {
    let options = state
        .services
        .engine
        .executor_options_for_request(compiled.path(), &state.services.metrics);
    let key = state.services.engine.request_executor_cache_key();
    REQUEST_EXECUTOR_CACHE.with(|cache| {
        let mut cached = cache.borrow_mut();
        let refresh = match cached.as_ref() {
            Some(cached) => cached.key != key,
            None => true,
        };
        if refresh {
            *cached = Some(CachedRequestExecutor {
                key,
                executor: state.services.engine.executor(options.clone()),
            });
        }
        match cached.as_mut() {
            Some(cached) => {
                cached.executor.reconfigure(options);
                cached.executor.execute_compiled(compiled, input)
            }
            None => state
                .services
                .engine
                .executor(options)
                .execute_compiled(compiled, input),
        }
    })
}

pub(crate) fn log_php_execution_failure(script_path: &Path, output: &PhpExecutionOutput) {
    if output.status == PhpExecutionStatus::Success {
        return;
    }

    let diagnostics = output
        .runtime_diagnostics
        .iter()
        .take(5)
        .map(|diagnostic| diagnostic.to_json())
        .collect::<Vec<_>>()
        .join(" | ");
    let diagnostic_summary = if diagnostics.is_empty() {
        output.diagnostics_text.trim()
    } else {
        diagnostics.as_str()
    };

    warn!(
        script=%script_path.display(),
        status=?output.status,
        runtime_diagnostics=output.runtime_diagnostics.len(),
        stdout_bytes=output.stdout.len(),
        diagnostics=%diagnostic_summary,
        "php execution failed"
    );
}

pub(crate) fn append_vm_counters_to_trace(
    trace: &mut PerfTraceEvent,
    counters: Option<&php_vm::api::VmCounters>,
) {
    let Some(counters) = counters else {
        return;
    };
    trace.counters.extend([
        ("native_compile_attempts", counters.native_compile_attempts),
        (
            "native_compile_successes",
            counters.native_compile_successes,
        ),
        ("native_compile_failures", counters.native_compile_failures),
        ("native_cache_hits", counters.native_cache_hits),
        ("native_cache_misses", counters.native_cache_misses),
        ("native_cache_writes", counters.native_cache_writes),
        (
            "native_cache_compile_waits",
            counters.native_cache_compile_waits,
        ),
        ("native_cache_evictions", counters.native_cache_evictions),
        (
            "native_compile_time_nanos",
            counters.native_compile_time_nanos,
        ),
        (
            "native_execution_entries",
            counters.native_execution_entries,
        ),
        ("native_region_entries", counters.native_region_entries),
        (
            "native_region_side_exits",
            counters.native_region_side_exits,
        ),
        ("native_call_direct", counters.native_call_direct),
        ("native_call_dynamic", counters.native_call_dynamic),
        (
            "native_version_published",
            counters.native_version_published,
        ),
        ("native_version_retired", counters.native_version_retired),
        ("native_transition_count", counters.native_transition_count),
        (
            "native_transition_time_nanos",
            counters.native_transition_time_nanos,
        ),
        ("runtime_helper_calls", counters.runtime_helper_calls),
        (
            "runtime_helper_time_nanos",
            counters.runtime_helper_time_nanos,
        ),
        (
            "runtime_helper_object_release_fast_paths",
            counters.runtime_helper_object_release_fast_paths,
        ),
        (
            "runtime_helper_object_release_root_scans",
            counters.runtime_helper_object_release_root_scans,
        ),
        ("gc_safepoint_polls", counters.gc_safepoint_polls),
        (
            "gc_safepoint_collections",
            counters.gc_safepoint_collections,
        ),
    ]);
}

fn runtime_diagnostic_samples(output: &PhpExecutionOutput) -> String {
    output
        .runtime_diagnostics
        .iter()
        .take(5)
        .map(|diagnostic| {
            let mut sample = String::new();
            sample.push_str(diagnostic.id());
            sample.push_str(": ");
            sample.push_str(&truncate_debug_value(diagnostic.message(), 240));
            let span = diagnostic.source_span();
            if let Some(file) = &span.file {
                sample.push_str(" @ ");
                sample.push_str(&truncate_debug_value(file, 160));
                sample.push(':');
                sample.push_str(&span.start.to_string());
            }
            sample
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn truncate_debug_value(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            out.push_str("...");
            break;
        }
        if ch.is_control() {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}
fn php_streaming_response(
    head: PhpResponseHead,
    is_head: bool,
    body_receiver: mpsc::Receiver<Result<Vec<u8>, std::io::Error>>,
    cancellation: RuntimeCancellationState,
) -> Response<ResponseBody> {
    let status = StatusCode::from_u16(head.response.status_code).unwrap_or(StatusCode::OK);
    let suppress_body =
        is_head || status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED;
    let mut body = if suppress_body {
        response::full_body(Bytes::new())
    } else {
        response::channel_body(body_receiver)
    };
    body.attach_cancellation(cancellation);
    if let Some(complete_length) = head.complete_length {
        body.set_expected_bytes(complete_length);
    }
    let mut response = Response::builder()
        .status(status)
        .body(body)
        .expect("PHP streaming response builder is valid");
    apply_php_headers(response.headers_mut(), &head.response);
    if let Some(content_length) = head.complete_length {
        response.headers_mut().insert(
            header::CONTENT_LENGTH,
            HeaderValue::from_str(&content_length.to_string())
                .expect("content length header is valid"),
        );
    }
    response
}

fn php_compile_error_response(output: PhpExecutionOutput, is_head: bool) -> Response<ResponseBody> {
    debug_assert_ne!(output.status, PhpExecutionStatus::Success);
    let captured_output = output.stdout;
    let captured = if captured_output.is_empty() {
        Bytes::from_static(b"php execution failed\n")
    } else {
        Bytes::from(captured_output)
    };
    let content_length = captured.len();
    let body = if is_head { Bytes::new() } else { captured };
    let mut response = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(response::full_body(body))
        .expect("PHP compile-error response builder is valid");
    apply_php_headers(response.headers_mut(), &output.http_response);
    response.headers_mut().insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&content_length.to_string()).expect("content length header is valid"),
    );
    response
}

pub(crate) fn php_execution_timed_out(output: &PhpExecutionOutput) -> bool {
    output
        .runtime_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.id() == "E_PHP_VM_EXECUTION_TIMEOUT")
}

fn apply_php_headers(headers: &mut HeaderMap, http_response: &RuntimeHttpResponseState) {
    for header in &http_response.headers {
        if header.name.eq_ignore_ascii_case("Content-Length") {
            continue;
        }
        let Ok(name) = HeaderName::from_bytes(header.name.as_bytes()) else {
            continue;
        };
        let Ok(value) = HeaderValue::from_str(&header.value) else {
            continue;
        };
        headers.append(name, value);
    }
    if !headers.contains_key(header::CONTENT_TYPE) {
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(PHP_CONTENT_TYPE),
        );
    }
}

pub(crate) const PHP_CONTENT_TYPE: &str = "text/html; charset=UTF-8";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BodyReadError {
    TooLarge,
    Invalid,
    Internal,
}

async fn body_and_multipart_stage(
    state: &AppState,
    parts: &Parts,
    body: RequestBody,
) -> Result<Result<PreparedRequestData, BodyReadError>, tokio::time::error::Elapsed> {
    timeout(
        state.request.request_timeout,
        prepare_request_data(body, parts, state),
    )
    .await
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedRequestData {
    pub(crate) body: RuntimeRequestBody,
    pub(crate) parsed: ParsedRequestData,
}

pub(crate) async fn prepare_request_data(
    body: RequestBody,
    parts: &Parts,
    state: &AppState,
) -> Result<PreparedRequestData, BodyReadError> {
    let content_type = header_value(&parts.headers, header::CONTENT_TYPE);
    let automatic_post = state.request.enable_post_data_reading && parts.method == Method::POST;
    let declared_length = header_value(&parts.headers, header::CONTENT_LENGTH)
        .and_then(|value| value.parse::<u64>().ok());
    if automatic_post
        && declared_length.is_some_and(|length| length > state.request.post_max_bytes as u64)
    {
        let body = ingest_request_body(body, state).await?;
        let mut parsed = ParsedRequestData::empty(&state.services.metrics);
        parsed.startup_warnings.push(format!(
            "PHP Request Startup: POST Content-Length of {} bytes exceeds the limit of {} bytes",
            declared_length.unwrap_or_default(),
            state.request.post_max_bytes
        ));
        return Ok(PreparedRequestData { body, parsed });
    }
    let boundary = validated_multipart_boundary(content_type.as_deref());
    let automatic_multipart = automatic_post && matches!(boundary, Ok(Some(_)));
    if automatic_post && boundary.is_err() {
        state
            .services
            .metrics
            .upload_parse_errors
            .fetch_add(1, Ordering::Relaxed);
        // PHP does not consume a multipart POST that lacks its boundary. Keep
        // the raw body replayable while exposing empty automatic inputs.
        let body = ingest_request_body(body, state).await?;
        let mut parsed = ParsedRequestData::empty(&state.services.metrics);
        parsed.startup_warnings.push(
            "PHP Request Startup: Missing boundary in multipart/form-data POST data".to_string(),
        );
        return Ok(PreparedRequestData { body, parsed });
    }
    if automatic_multipart {
        let parsed = match parse_multipart_stream(
            body,
            content_type.as_deref().unwrap_or_default(),
            &state.request.multipart_config,
            &state.services.metrics,
        )
        .await
        {
            Ok(parsed) => parsed,
            Err(MultipartError::TooLarge) => return Err(BodyReadError::TooLarge),
            Err(MultipartError::Malformed(_) | MultipartError::Limit(_)) => {
                state
                    .services
                    .metrics
                    .upload_parse_errors
                    .fetch_add(1, Ordering::Relaxed);
                ParsedRequestData::empty(&state.services.metrics)
            }
        };
        return Ok(PreparedRequestData {
            body: RuntimeRequestBody::auto_parsed_multipart(),
            parsed,
        });
    }
    let body = ingest_request_body(body, state).await?;
    Ok(PreparedRequestData {
        body,
        parsed: ParsedRequestData::empty(&state.services.metrics),
    })
}

pub(crate) struct RequestBodyIngest<'a> {
    state: &'a AppState,
}

struct RequestBodyTempGauge {
    metrics: std::sync::Weak<super::metrics::ServerMetrics>,
    bytes: u64,
    transferred: bool,
}

impl RequestBodyTempGauge {
    fn new(metrics: &Arc<super::metrics::ServerMetrics>) -> Self {
        metrics
            .request_body_tempfiles_active
            .fetch_add(1, Ordering::Relaxed);
        Self {
            metrics: Arc::downgrade(metrics),
            bytes: 0,
            transferred: false,
        }
    }

    fn wrote(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_add(bytes);
        if let Some(metrics) = self.metrics.upgrade() {
            metrics
                .request_body_tempfile_bytes_active
                .fetch_add(bytes, Ordering::Relaxed);
        }
    }

    fn transfer(mut self) -> Arc<dyn Fn(u64) + Send + Sync + 'static> {
        self.transferred = true;
        let metrics = self.metrics.clone();
        Arc::new(move |length| {
            if let Some(metrics) = metrics.upgrade() {
                metrics
                    .request_body_tempfiles_active
                    .fetch_sub(1, Ordering::Relaxed);
                metrics
                    .request_body_tempfile_bytes_active
                    .fetch_sub(length, Ordering::Relaxed);
            }
        })
    }
}

impl Drop for RequestBodyTempGauge {
    fn drop(&mut self) {
        if self.transferred {
            return;
        }
        if let Some(metrics) = self.metrics.upgrade() {
            metrics
                .request_body_tempfiles_active
                .fetch_sub(1, Ordering::Relaxed);
            metrics
                .request_body_tempfile_bytes_active
                .fetch_sub(self.bytes, Ordering::Relaxed);
        }
    }
}

impl<'a> RequestBodyIngest<'a> {
    #[must_use]
    pub(crate) fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub(crate) async fn ingest(
        &self,
        mut body: RequestBody,
    ) -> Result<RuntimeRequestBody, BodyReadError> {
        let memory_limit = self.state.request.request_body_memory_bytes;
        let hard_limit = self.state.request.max_body_bytes;
        let mut total = 0usize;
        let mut memory = BytesMut::with_capacity(memory_limit.min(16 * 1024));
        let mut spool: Option<(tokio::fs::File, tempfile::TempPath, RequestBodyTempGauge)> = None;

        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| BodyReadError::Invalid)?;
            let Ok(data) = frame.into_data() else {
                continue;
            };
            if data.len() > hard_limit.saturating_sub(total) {
                self.state
                    .services
                    .metrics
                    .request_body_hard_limit_rejections_total
                    .fetch_add(1, Ordering::Relaxed);
                return Err(BodyReadError::TooLarge);
            }
            total += data.len();

            if spool.is_none() && total <= memory_limit {
                memory.extend_from_slice(&data);
                continue;
            }

            if spool.is_none() {
                let named = tempfile::Builder::new()
                    .prefix("phrust-body-")
                    .tempfile_in(&self.state.request.request_body_temp_dir)
                    .map_err(|_| {
                        self.state
                            .services
                            .metrics
                            .request_body_tempfile_failures_total
                            .fetch_add(1, Ordering::Relaxed);
                        BodyReadError::Internal
                    })?;
                let (file, path) = named.into_parts();
                let mut file = tokio::fs::File::from_std(file);
                let mut gauge = RequestBodyTempGauge::new(&self.state.services.metrics);
                if !memory.is_empty() {
                    file.write_all(&memory)
                        .await
                        .map_err(|_| BodyReadError::Internal)?;
                    gauge.wrote(memory.len() as u64);
                    memory.clear();
                }
                spool = Some((file, path, gauge));
            }
            if let Some((file, _, gauge)) = spool.as_mut() {
                file.write_all(&data)
                    .await
                    .map_err(|_| BodyReadError::Internal)?;
                gauge.wrote(data.len() as u64);
            }
        }

        let Some((mut file, path, gauge)) = spool else {
            self.state
                .services
                .metrics
                .request_body_memory_total
                .fetch_add(1, Ordering::Relaxed);
            return Ok(RuntimeRequestBody::memory(memory.freeze().to_vec()));
        };
        file.flush().await.map_err(|_| BodyReadError::Internal)?;
        drop(file);

        let length = total as u64;
        self.state
            .services
            .metrics
            .request_body_spooled_total
            .fetch_add(1, Ordering::Relaxed);
        self.state
            .services
            .metrics
            .request_body_spooled_bytes_total
            .fetch_add(length, Ordering::Relaxed);
        Ok(RuntimeRequestBody::file(
            path,
            length,
            Some(gauge.transfer()),
        ))
    }
}

pub(crate) async fn ingest_request_body(
    body: RequestBody,
    state: &AppState,
) -> Result<RuntimeRequestBody, BodyReadError> {
    RequestBodyIngest::new(state).ingest(body).await
}

#[allow(clippy::too_many_arguments)]
fn request_globals_stage(
    parts: &Parts,
    state: &AppState,
    script_path: &Path,
    script_name: &str,
    path_info: Option<String>,
    body: RuntimeRequestBody,
    peer: SocketAddr,
) -> RuntimeHttpRequestContext {
    http_runtime_context(
        parts,
        state,
        script_path,
        script_name,
        path_info,
        body,
        peer,
    )
}

fn session_load_stage(
    request: &RuntimeHttpRequestContext,
    state: &AppState,
) -> Result<SessionState, String> {
    seed_session_state(request, state)
}

pub(crate) fn http_runtime_context(
    parts: &Parts,
    state: &AppState,
    script_path: &Path,
    script_name: &str,
    path_info: Option<String>,
    body: RuntimeRequestBody,
    peer: SocketAddr,
) -> RuntimeHttpRequestContext {
    let request_uri = parts.uri.path_and_query().map_or_else(
        || parts.uri.path().to_string(),
        |value| value.as_str().to_string(),
    );
    let request_uri = rewrite_request_uri(&request_uri, &state.route_config.request_rewrites);
    let host =
        header_value(&parts.headers, header::HOST).unwrap_or_else(|| "localhost".to_string());
    let (request_time, request_time_float_micros) = request_time_pair();
    let mut context = RuntimeHttpRequestContext::new(
        parts.method.as_str(),
        host.clone(),
        request_uri,
        script_name.to_string(),
        script_path.to_string_lossy().into_owned(),
        state.route_config.docroot.to_string_lossy().into_owned(),
    );
    context.scheme = state.transport.request_scheme.to_string();
    context.host = host;
    context.server_name = server_name_from_host(&context.host);
    let local_addr = parts
        .extensions
        .get::<RequestLocalAddr>()
        .map_or(state.transport.local_addr, |addr| addr.0);
    context.server_addr = local_addr.ip().to_string();
    context.server_port = local_addr.port();
    context.server_protocol = format!("{:?}", parts.version);
    context.https = state.transport.request_scheme == "https";
    context.php_self = php_self_for(script_name, path_info.as_deref());
    context.path_info = path_info;
    context.remote_addr = peer.ip().to_string();
    context.remote_port = Some(peer.port());
    if let Some((user, password)) = basic_authorization(&parts.headers) {
        context.auth_type = Some("Basic".to_string());
        context.remote_user = Some(user.clone());
        context.php_auth_user = Some(user);
        context.php_auth_pw = Some(password);
    }
    context.request_time = request_time;
    context.request_time_float_micros = request_time_float_micros;
    let header_snapshot = runtime_headers(&parts.headers);
    state
        .services
        .metrics
        .request_headers_seen
        .fetch_add(header_snapshot.seen, Ordering::Relaxed);
    state
        .services
        .metrics
        .request_headers_materialized
        .fetch_add(header_snapshot.entries.len() as u64, Ordering::Relaxed);
    state
        .services
        .metrics
        .request_headers_skipped_direct
        .fetch_add(header_snapshot.skipped_direct, Ordering::Relaxed);
    context.headers = header_snapshot.entries;
    context.content_type = header_value(&parts.headers, header::CONTENT_TYPE);
    context.content_length = header_value(&parts.headers, header::CONTENT_LENGTH)
        .and_then(|value| value.parse::<u64>().ok());
    context.raw_body = body.clone();
    if state.request.enable_post_data_reading
        && parts.method == Method::POST
        && context.content_length.is_none()
        && body.len() > state.request.post_max_bytes as u64
    {
        context.startup_warnings.push(format!(
            "PHP Request Startup: POST data of {} bytes exceeds the limit of {} bytes",
            body.len(),
            state.request.post_max_bytes
        ));
    }
    if state.request.enable_post_data_reading
        && parts.method == Method::POST
        && context
            .content_type
            .as_deref()
            .is_some_and(is_form_urlencoded_content_type)
        && body.len() <= state.request.post_max_bytes as u64
        && let Ok(reader) = body.independent_reader()
    {
        context.parsed_post =
            parse_form_urlencoded_reader(reader, state.request.multipart_config.max_input_vars)
                .unwrap_or_default();
    }
    if let Some(cookie) = header_value(&parts.headers, header::COOKIE) {
        context.parsed_cookie = parse_cookie_header(&cookie);
    }
    context
}

fn rewrite_request_uri(request_uri: &str, rules: &[RequestRewriteRule]) -> String {
    let (path, query) = request_uri
        .split_once('?')
        .map_or((request_uri, ""), |(path, query)| (path, query));
    for rule in rules {
        let Some(route) = rewritten_route_for_prefix(path, &rule.path_prefix) else {
            continue;
        };
        let rewrite_query = format!(
            "{}={}",
            rule.query_parameter,
            percent_encode_query_value(&route)
        );
        return if query.is_empty() {
            format!("/?{rewrite_query}")
        } else {
            format!("/?{rewrite_query}&{query}")
        };
    }
    request_uri.to_string()
}

fn rewritten_route_for_prefix(path: &str, prefix: &str) -> Option<String> {
    if prefix == "/" {
        return Some(if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        });
    }
    if path == prefix {
        return Some("/".to_string());
    }
    let remainder = path.strip_prefix(prefix)?;
    remainder
        .starts_with('/')
        .then(|| if remainder.is_empty() { "/" } else { remainder }.to_string())
}

fn percent_encode_query_value(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(hex_digit(byte >> 4));
                encoded.push(hex_digit(byte & 0x0f));
            }
        }
    }
    encoded
}

fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => unreachable!("hex nibble is four bits"),
    }
}

pub(crate) fn server_env_for_request(state: &AppState) -> Arc<Vec<(String, String)>> {
    if !state.capabilities.network_requests_enabled
        || state
            .capabilities
            .env_snapshot
            .iter()
            .any(|(name, _)| name == "PHRUST_NET_TESTS")
    {
        return Arc::clone(&state.capabilities.env_snapshot);
    }

    let mut env = state
        .capabilities
        .env_snapshot
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    env.push(("PHRUST_NET_TESTS".to_string(), "1".to_string()));
    env.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    Arc::new(env)
}

pub(crate) fn php_runtime_context_for_http(
    state: &AppState,
    request_context: RuntimeHttpRequestContext,
    session_state: SessionState,
    env: Arc<Vec<(String, String)>>,
    sessions: &SessionRequestCallbacks,
) -> RuntimeContext {
    let request_parser = request_parser_callback(state, &request_context);
    RuntimeContext::controlled_http(request_context)
        .with_cwd(state.route_config.docroot.clone())
        .with_include_path(vec![state.route_config.docroot.clone()])
        .with_ini_overrides(vec![
            (
                "session.name".to_owned(),
                state.sessions.config.cookie_name.clone(),
            ),
            (
                "session.cookie_path".to_owned(),
                state.sessions.config.cookie_path.clone(),
            ),
            (
                "session.cookie_domain".to_owned(),
                state.sessions.config.cookie_domain.clone(),
            ),
            (
                "session.cookie_lifetime".to_owned(),
                state.sessions.config.cookie_lifetime.to_string(),
            ),
            (
                "session.cookie_secure".to_owned(),
                u8::from(state.sessions.config.cookie_secure).to_string(),
            ),
            (
                "session.cookie_httponly".to_owned(),
                u8::from(state.sessions.config.cookie_httponly).to_string(),
            ),
            (
                "session.cookie_samesite".to_owned(),
                state.sessions.config.cookie_samesite.clone(),
            ),
            (
                "session.cookie_partitioned".to_owned(),
                u8::from(state.sessions.config.cookie_partitioned).to_string(),
            ),
            (
                "session.use_cookies".to_owned(),
                u8::from(state.sessions.config.use_cookies).to_string(),
            ),
            (
                "session.use_only_cookies".to_owned(),
                u8::from(state.sessions.config.use_only_cookies).to_string(),
            ),
            (
                "session.use_strict_mode".to_owned(),
                u8::from(state.sessions.config.use_strict_mode).to_string(),
            ),
            (
                "session.save_path".to_owned(),
                state.sessions.config.save_path.display().to_string(),
            ),
            (
                "session.serialize_handler".to_owned(),
                state.sessions.config.serialize_handler.clone(),
            ),
        ])
        .with_session_state(session_state)
        .with_session_loader(sessions.loader.clone())
        .with_session_id_generator(sessions.id_generator.clone())
        .with_session_writer(sessions.writer.clone())
        .with_session_destroyer(sessions.destroyer.clone())
        .with_session_aborter(sessions.aborter.clone())
        .with_session_regenerator(sessions.regenerator.clone())
        .with_session_gc(sessions.gc.clone())
        .with_request_parser(request_parser)
        .with_execution_time_limit(state.request.execution_time_limit)
        .with_sorted_env_arc(env)
}

fn request_parser_callback(
    state: &AppState,
    request: &RuntimeHttpRequestContext,
) -> RequestParserCallback {
    let body = request.raw_body.clone();
    let content_type = request.content_type.clone().unwrap_or_default();
    let base_config = state.request.multipart_config.clone();
    let metrics = Arc::clone(&state.services.metrics);
    let tokio_handle = state.services.tokio_handle.clone();
    let retained_uploads = Arc::new(Mutex::new(Vec::new()));
    RequestParserCallback::new(move |options: RequestParseBodyOptions| {
        metrics
            .request_parse_body_calls_total
            .fetch_add(1, Ordering::Relaxed);
        let parse = || {
            let mut config = base_config.clone();
            if let Some(value) = options.max_file_uploads {
                config.max_upload_files = value;
            }
            if let Some(value) = options.max_input_vars {
                config.max_input_vars = value;
            }
            if let Some(value) = options.max_multipart_body_parts {
                config.max_multipart_parts = Some(value);
            }
            if let Some(value) = options.post_max_size {
                if value > config.max_body_bytes {
                    return Err(RequestParseBodyError::InvalidOptions(
                        "post_max_size exceeds the server hard body limit".to_string(),
                    ));
                }
                config.post_max_bytes = value;
            }
            if let Some(value) = options.upload_max_filesize {
                if value > config.max_body_bytes {
                    return Err(RequestParseBodyError::InvalidOptions(
                        "upload_max_filesize exceeds the server hard body limit".to_string(),
                    ));
                }
                config.max_upload_file_bytes = value;
            }
            config.throw_limit_errors = true;
            match body.consume_for_request_parse() {
                Ok(()) => {}
                Err(php_runtime::api::RuntimeRequestBodyConsumeError::RawInputObserved) => {
                    return Ok(RuntimeParsedRequestData::default());
                }
                Err(error) => {
                    return Err(RequestParseBodyError::Parse(format!(
                        "request body is not available: {error:?}"
                    )));
                }
            }
            if is_form_urlencoded_content_type(&content_type) {
                if body.len() > config.post_max_bytes as u64 {
                    return Err(RequestParseBodyError::Parse(format!(
                        "POST Content-Length of {} bytes exceeds the limit of {} bytes",
                        body.len(),
                        config.post_max_bytes
                    )));
                }
                let reader = body
                    .reader_for_parser()
                    .map_err(|error| RequestParseBodyError::Parse(error.to_string()))?;
                let post = parse_form_urlencoded_reader_with_separators(
                    reader,
                    config.max_input_vars,
                    options.arg_separator_input.as_bytes(),
                )
                .map_err(|error| RequestParseBodyError::Parse(error.to_string()))?;
                return Ok(RuntimeParsedRequestData {
                    post,
                    files: Vec::new(),
                });
            }
            if validated_multipart_boundary(Some(&content_type))
                .map_err(|error| RequestParseBodyError::Parse(error.to_string()))?
                .is_none()
            {
                return Err(RequestParseBodyError::Parse(
                    "Content-Type is not application/x-www-form-urlencoded or multipart/form-data"
                        .to_string(),
                ));
            }
            let reader = tokio_handle
                .block_on(body.async_reader_for_parser())
                .map_err(|error| RequestParseBodyError::Parse(error.to_string()))?;
            let request_body = response::request_reader_body(reader);
            let parsed = tokio_handle
                .block_on(parse_multipart_stream(
                    request_body,
                    &content_type,
                    &config,
                    &metrics,
                ))
                .map_err(|error| RequestParseBodyError::Parse(error.to_string()))?;
            retained_uploads
                .lock()
                .map_err(|_| {
                    RequestParseBodyError::Parse("upload owner lock poisoned".to_string())
                })?
                .push(Arc::clone(&parsed.uploads));
            Ok(RuntimeParsedRequestData {
                post: parsed.post,
                files: parsed.files,
            })
        };
        let result = parse();
        if result.is_err() {
            metrics
                .request_parse_body_errors_total
                .fetch_add(1, Ordering::Relaxed);
        }
        result
    })
}

fn record_phase(
    state: &AppState,
    trace: &mut Option<PerfTraceEvent>,
    phase: RequestPhase,
    name: &'static str,
    duration: Duration,
) {
    let nanos = duration.as_nanos();
    state.services.metrics.record_phase(phase, nanos);
    if let Some(trace) = trace.as_mut() {
        trace.phases.push((name, nanos));
    }
}

fn finish_php_request(
    _state: &AppState,
    trace: Option<PerfTraceEvent>,
    response: Response<ResponseBody>,
    cache_hit: Option<bool>,
    failure_stage: Option<RequestStage>,
) -> (Response<ResponseBody>, Option<bool>) {
    match failure_stage {
        Some(stage) => RequestOutcome::failure(response, cache_hit, stage),
        None => RequestOutcome::success(response, cache_hit),
    }
    .into_response(trace)
}

pub(crate) fn header_value(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn basic_authorization(headers: &HeaderMap) -> Option<(String, String)> {
    let authorization = header_value(headers, header::AUTHORIZATION)?;
    let mut parts = authorization.splitn(2, char::is_whitespace);
    let scheme = parts.next()?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() {
        return None;
    }
    let decoded = BASE64_STANDARD.decode(token.as_bytes()).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (user, password) = decoded.split_once(':')?;
    Some((user.to_string(), password.to_string()))
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeHeaderSnapshot {
    pub(crate) entries: Vec<(String, String)>,
    pub(crate) seen: u64,
    pub(crate) skipped_direct: u64,
}

pub(crate) fn runtime_headers(headers: &HeaderMap) -> RuntimeHeaderSnapshot {
    let mut snapshot = RuntimeHeaderSnapshot {
        seen: headers.len() as u64,
        ..RuntimeHeaderSnapshot::default()
    };
    for (name, value) in headers {
        if matches!(name.as_str(), "host" | "content-type" | "content-length") {
            snapshot.skipped_direct = snapshot.skipped_direct.saturating_add(1);
            continue;
        }
        let Some(value) = value.to_str().ok() else {
            continue;
        };
        snapshot
            .entries
            .push((name.as_str().to_string(), value.to_string()));
    }
    snapshot
}

pub(crate) fn is_form_urlencoded_content_type(value: &str) -> bool {
    value.split(';').next().is_some_and(|media_type| {
        media_type
            .trim()
            .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    })
}

pub(crate) fn script_name_for(docroot: &Path, script_path: &Path) -> String {
    let relative = script_path.strip_prefix(docroot).unwrap_or(script_path);
    let mut value = String::from("/");
    value.push_str(
        &relative
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
    );
    value
}

pub(crate) fn include_roots_for_docroot(docroot: &Path) -> Vec<PathBuf> {
    let mut roots = vec![docroot.to_path_buf()];
    if let Some(parent) = docroot.parent()
        && parent != docroot
    {
        roots.push(parent.to_path_buf());
    }
    roots
}

pub(crate) fn php_self_for(script_name: &str, path_info: Option<&str>) -> String {
    path_info.map_or_else(
        || script_name.to_string(),
        |path_info| format!("{script_name}{path_info}"),
    )
}

pub(crate) fn server_name_from_host(host: &str) -> String {
    if let Some(rest) = host.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        return rest[..end].to_string();
    }
    host.rsplit_once(':')
        .filter(|(_, port)| port.bytes().all(|byte| byte.is_ascii_digit()))
        .map_or_else(|| host.to_string(), |(name, _)| name.to_string())
}

pub(crate) fn request_time_pair() -> (i64, i64) {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (
        duration.as_secs() as i64,
        duration
            .as_secs()
            .saturating_mul(1_000_000)
            .saturating_add(u64::from(duration.subsec_micros())) as i64,
    )
}

#[cfg(test)]
mod worker_payload_tests {
    use super::*;
    use std::io::Read;

    fn assert_send<T: Send>() {}

    /// The dedicated PHP worker pool moves a request job onto a pinned
    /// worker thread and a finished HTTP response back. Everything the job
    /// captures and everything it returns must stay `Send`; execution-side
    /// types (`RuntimeContext`, `PhpExecutionOutput`) are deliberately NOT
    /// in this list — they are `Rc`-based and must be created and consumed
    /// entirely inside the worker.
    #[test]
    fn worker_job_payload_types_are_send() {
        assert_send::<Arc<AppState>>();
        assert_send::<CompiledScriptCacheLookup>();
        assert_send::<PathBuf>();
        assert_send::<Parts>();
        assert_send::<Arc<[u8]>>();
        assert_send::<Option<crate::perf_trace::PerfTraceEvent>>();
        assert_send::<OwnedSemaphorePermit>();
        assert_send::<Response<ResponseBody>>();
    }

    #[test]
    fn deferred_router_sink_keeps_small_output_in_memory() {
        let sink = DeferredRouterSink::new(false);
        sink.commit(&RuntimeHttpResponseState::default(), Some(5))
            .expect("commit router head");
        sink.write(b"hello".to_vec()).expect("write router output");

        let output = sink.take_output().expect("take router output");
        assert_eq!(output.head.complete_length, Some(5));
        match output.body {
            DeferredRouterBody::Memory(bytes) => assert_eq!(bytes, b"hello"),
            DeferredRouterBody::File(_) => panic!("small output unexpectedly spooled to file"),
        }
    }

    #[test]
    fn deferred_router_sink_spills_and_unlinks_large_output() {
        let sink = DeferredRouterSink::new(false);
        sink.commit(&RuntimeHttpResponseState::default(), None)
            .expect("commit router head");
        let bytes = vec![b'x'; ROUTER_MEMORY_SPOOL_BYTES + 1];
        sink.write(bytes.clone()).expect("write router output");
        let spool_path = {
            let state = sink.state.lock().expect("router state lock");
            match &state.storage {
                DeferredRouterStorage::File(spool) => spool.path.clone(),
                _ => panic!("large output did not spill to a file"),
            }
        };
        assert!(spool_path.exists());

        let output = sink.take_output().expect("take router output");
        assert!(!spool_path.exists());
        assert_eq!(output.head.complete_length, Some(bytes.len() as u64));
        let mut actual = Vec::new();
        match output.body {
            DeferredRouterBody::File(mut file) => {
                file.read_to_end(&mut actual).expect("read router spool");
            }
            DeferredRouterBody::Memory(_) => panic!("large output remained in memory"),
        }
        assert_eq!(actual, bytes);
    }

    #[test]
    fn dropping_deferred_router_sink_removes_unpublished_spool() {
        let spool_path = {
            let sink = DeferredRouterSink::new(false);
            sink.write(vec![b'x'; ROUTER_MEMORY_SPOOL_BYTES + 1])
                .expect("write router output");
            let state = sink.state.lock().expect("router state lock");
            match &state.storage {
                DeferredRouterStorage::File(spool) => spool.path.clone(),
                _ => panic!("large output did not spill to a file"),
            }
        };
        assert!(!spool_path.exists());
    }

    #[tokio::test]
    async fn php_chunk_bridge_blocks_only_producer_at_fixed_capacity() {
        let (head_sender, _head_receiver) = oneshot::channel();
        let (chunk_sender, mut chunk_receiver) = mpsc::channel(PHP_OUTPUT_QUEUE_CAPACITY);
        let sink = Arc::new(PhpResponseSink {
            head: Mutex::new(Some(head_sender)),
            chunks: Some(chunk_sender),
            suppress_body: std::sync::atomic::AtomicBool::new(false),
            cancellation: RuntimeCancellationState::new(),
            metrics: Arc::new(super::super::metrics::ServerMetrics::default()),
            defer_head_until_finish: false,
        });
        let writer_sink = Arc::clone(&sink);
        let (finished_sender, finished_receiver) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            for index in 0..=PHP_OUTPUT_QUEUE_CAPACITY {
                writer_sink
                    .write(vec![index as u8])
                    .expect("write PHP bridge chunk");
            }
            finished_sender.send(()).expect("signal producer finish");
        });

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(finished_receiver.try_recv().is_err());
        assert_eq!(
            chunk_receiver
                .recv()
                .await
                .expect("queued PHP chunk")
                .expect("successful PHP chunk"),
            vec![0]
        );
        finished_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("producer resumes after one queue slot opens");
        writer.join().expect("join PHP bridge producer");
    }

    #[test]
    fn php_chunk_bridge_receiver_drop_respects_ignore_user_abort() {
        let (head_sender, _head_receiver) = oneshot::channel();
        let (chunk_sender, chunk_receiver) = mpsc::channel(1);
        drop(chunk_receiver);
        let cancellation = RuntimeCancellationState::new();
        let sink = PhpResponseSink {
            head: Mutex::new(Some(head_sender)),
            chunks: Some(chunk_sender),
            suppress_body: std::sync::atomic::AtomicBool::new(false),
            cancellation: cancellation.clone(),
            metrics: Arc::new(super::super::metrics::ServerMetrics::default()),
            defer_head_until_finish: false,
        };

        assert!(sink.write(vec![1]).is_err());
        assert!(cancellation.is_cancelled());
        cancellation.set_ignore_user_abort(true);
        assert!(sink.write(vec![2]).is_ok());
    }
}
