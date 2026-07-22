use crate::{
    access_log::AccessLogger,
    acme::prepare_acme_cache_dir,
    config::{ConfigError, ServerConfig, TlsMode},
    http3::build_http3_endpoint,
    metrics::ServerMetrics,
    multipart::MultipartConfig,
    routing::RouteConfig,
    serve::serve_until_shutdown,
    session_store::SessionStore,
    shutdown::ShutdownCoordinator,
    state::{
        AppState, CapabilityState, ConcurrencyServices, ConnectionServices, ObservabilityState,
        RequestRuntimeConfig, RequestTransport, RuntimeServices, ServerEngineState, SessionConfig,
        SessionServices, TransportLimits, preload_script_cache, server_env_snapshot,
    },
    static_files::StaticFileService,
    tls::{TcpTls, build_tls},
};
use php_diagnostics::{
    DiagnosticCause, DiagnosticEnvelope, DiagnosticLayer, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticSuggestion,
};
use php_executor::{
    CompiledScriptCache, DeploymentRootFingerprint, IncludeCache, PhpExecutionError,
    SERVER_INCLUDE_REVALIDATION_INTERVAL, include_revalidation_interval_from_env,
};
use php_vm::api::VmError;
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{net::TcpListener, sync::Semaphore};
use tracing::debug;

#[derive(Debug)]
pub enum ServerError {
    Config(Box<ConfigError>),
    Io(std::io::Error),
    Preload(Box<PreloadError>),
    Tls(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Preload(error) => write!(f, "{error}"),
            Self::Tls(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ServerError {}

impl ServerError {
    #[must_use]
    pub fn diagnostic(&self) -> DiagnosticEnvelope {
        match self {
            Self::Config(error) => error.diagnostic().clone(),
            Self::Io(error) => {
                let cwd = std::env::current_dir()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|cwd_error| format!("<unavailable: {cwd_error}>"));
                let mut diagnostic = DiagnosticEnvelope::new(
                    "E_PHRUST_SERVER_IO",
                    DiagnosticLayer::server(),
                    DiagnosticPhase::new("startup"),
                    DiagnosticSeverity::Error,
                    format!("server startup I/O failed: {error}"),
                )
                .with_context(BTreeMap::from([
                    ("operation".to_string(), "server startup".to_string()),
                    ("cwd".to_string(), cwd),
                ]));
                diagnostic.cause = Some(DiagnosticCause::new(
                    error.to_string(),
                    Some("std::io::Error"),
                ));
                diagnostic.suggestion = Some(DiagnosticSuggestion::new(
                    "check listen address availability and filesystem permissions",
                ));
                diagnostic
            }
            Self::Preload(error) => error.diagnostic().clone(),
            Self::Tls(message) => {
                let mut diagnostic = DiagnosticEnvelope::new(
                    "E_PHRUST_SERVER_TLS",
                    DiagnosticLayer::server(),
                    DiagnosticPhase::new("tls"),
                    DiagnosticSeverity::Error,
                    message.clone(),
                );
                diagnostic.suggestion = Some(DiagnosticSuggestion::new(
                    "provide matching manual PEM files or validate the ACME directory/CA configuration",
                ));
                diagnostic
            }
        }
    }
}

#[derive(Debug)]
pub struct PreloadError {
    message: String,
    diagnostic: DiagnosticEnvelope,
}

impl PreloadError {
    pub(crate) fn manifest_read(path: &Path, error: &std::io::Error) -> Self {
        let message = format!(
            "script cache preload file `{}` cannot be read: {error}",
            path.display()
        );
        let diagnostic = DiagnosticEnvelope::new(
            "E_PHRUST_SERVER_PRELOAD_READ",
            DiagnosticLayer::server(),
            DiagnosticPhase::new("preload_manifest"),
            DiagnosticSeverity::Error,
            message.clone(),
        )
        .with_context(BTreeMap::from([
            ("preload_file".to_string(), path.display().to_string()),
            ("stage".to_string(), "manifest_read".to_string()),
        ]));
        Self {
            message,
            diagnostic,
        }
    }

    pub(crate) fn compile_entry(
        preload_file: &Path,
        line: usize,
        script_path: &Path,
        error: PhpExecutionError,
    ) -> Self {
        let message = format!(
            "script cache preload entry {line} in `{}` failed for `{}`",
            preload_file.display(),
            script_path.display()
        );
        let mut diagnostic = match error {
            PhpExecutionError::Compile(output) => {
                output.diagnostics.first().cloned().unwrap_or_else(|| {
                    DiagnosticEnvelope::new(
                        "E_PHRUST_SERVER_PRELOAD_COMPILE",
                        DiagnosticLayer::server(),
                        DiagnosticPhase::new("preload_compile"),
                        DiagnosticSeverity::Error,
                        output.diagnostics_text,
                    )
                })
            }
            PhpExecutionError::Engine(error) => DiagnosticEnvelope::new(
                "E_PHRUST_SERVER_PRELOAD_ENGINE",
                DiagnosticLayer::server(),
                DiagnosticPhase::new("preload_compile"),
                DiagnosticSeverity::Error,
                error,
            ),
        };
        diagnostic
            .context
            .extend(preload_context(preload_file, line, script_path, "compile"));
        diagnostic.suggestion = Some(DiagnosticSuggestion::new(
            "fix the preload entry or run without --strict-preload",
        ));
        Self {
            message,
            diagnostic,
        }
    }

    pub(crate) fn include_entry(
        preload_file: &Path,
        line: usize,
        script_path: &Path,
        error: VmError,
    ) -> Self {
        let message = format!(
            "script cache preload entry {line} in `{}` failed for `{}`",
            preload_file.display(),
            script_path.display()
        );
        let mut diagnostic = error.to_diagnostic_envelope();
        diagnostic.context.extend(preload_context(
            preload_file,
            line,
            script_path,
            "include_compile",
        ));
        diagnostic.suggestion = Some(DiagnosticSuggestion::new(
            "fix the preload entry or run without --strict-preload",
        ));
        Self {
            message,
            diagnostic,
        }
    }

    pub(crate) fn diagnostic(&self) -> &DiagnosticEnvelope {
        &self.diagnostic
    }
}

impl fmt::Display for PreloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

fn preload_context(
    preload_file: &Path,
    line: usize,
    script_path: &Path,
    stage: &str,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "preload_file".to_string(),
            preload_file.display().to_string(),
        ),
        ("preload_line".to_string(), line.to_string()),
        ("script_path".to_string(), script_path.display().to_string()),
        ("stage".to_string(), stage.to_string()),
    ])
}

impl From<ConfigError> for ServerError {
    fn from(error: ConfigError) -> Self {
        Self::Config(Box::new(error))
    }
}

impl From<std::io::Error> for ServerError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub async fn run(config: ServerConfig) -> Result<(), ServerError> {
    let docroot = config.validated_docroot()?;
    prepare_private_temp_root(&config.limits.request_body_temp_dir)?;
    prepare_private_temp_root(&config.sessions_uploads.upload_temp_dir)?;
    let script_cache_preload = config.engine.script_cache_preload.clone();
    let strict_preload = config.engine.strict_preload;
    let startup_front_controller = config.routing.front_controller.clone();
    let startup_upload_temp_dir = config.sessions_uploads.upload_temp_dir.clone();
    let startup_session_save_path = config.sessions_uploads.session_save_path.clone();
    let startup_script_cache_enabled = config.engine.script_cache_enabled;
    let startup_script_cache_shards = config.engine.script_cache_shards;
    let startup_script_cache_max_entries = config.engine.script_cache_max_entries;
    let startup_metrics_endpoint_enabled = config.routing.metrics_endpoint_enabled;
    let startup_metrics_token_enabled = config.observability.metrics_token.is_some();
    let startup_access_log = config.observability.access_log.clone();
    let startup_perf_trace = config.observability.perf_trace.clone();
    let startup_request_profile = config.observability.request_profile.clone();
    let startup_tls_enabled = config.transport.tls_mode.is_enabled();
    let startup_tls_mode = match &config.transport.tls_mode {
        TlsMode::Disabled => "disabled",
        TlsMode::Manual(_) => "manual",
        TlsMode::Acme(_) => "acme",
    };
    let startup_http3_enabled = config.transport.http3_enabled;
    let max_connections = config.transport.max_connections;
    let engine_profile = config.engine.engine_preset;
    let access_log = config
        .observability
        .access_log
        .as_deref()
        .map(AccessLogger::open)
        .transpose()?
        .map(Arc::new);
    let perf_trace = config
        .observability
        .perf_trace
        .map(crate::perf_trace::PerfTraceWriter::open)
        .transpose()?
        .map(Arc::new);
    let request_profile = config
        .observability
        .request_profile
        .map(crate::request_profile::RequestProfileWriter::open)
        .transpose()?
        .map(Arc::new);
    let session_store = Arc::new(SessionStore::with_lock_timeout(
        config.sessions_uploads.session_save_path.clone(),
        Duration::from_millis(config.sessions_uploads.session_lock_timeout_ms),
    ));
    if config.sessions_uploads.sessions_enabled {
        session_store
            .ensure_ready()
            .map_err(std::io::Error::other)?;
    }
    if let TlsMode::Acme(acme) = &config.transport.tls_mode {
        let cache = prepare_acme_cache_dir(&acme.cache_dir)?;
        for (name, path) in [
            ("request body", &config.limits.request_body_temp_dir),
            ("upload", &config.sessions_uploads.upload_temp_dir),
            ("session", &config.sessions_uploads.session_save_path),
        ] {
            if path.canonicalize().ok().as_ref() == Some(&cache) {
                return Err(ConfigError::new(format!(
                    "ACME cache directory must not be the {name} directory"
                ))
                .into());
            }
        }
    }
    debug!(docroot=%docroot.display(), "initializing phrust server");
    let script_cache = Arc::new(if config.engine.script_cache_enabled {
        CompiledScriptCache::new_with_limits(
            config.engine.script_cache_shards,
            config.engine.script_cache_max_entries,
            Duration::from_millis(config.engine.script_cache_check_interval_ms),
        )
    } else {
        CompiledScriptCache::disabled()
    });
    // Server default mirrors an opcache deployment (validate_timestamps=1,
    // revalidate_freq=2): cached includes serve without filesystem probes for
    // two seconds. PHRUST_INCLUDE_REVALIDATE_MS overrides; 0 validates every
    // hit like the reference CLI.
    let include_cache = Arc::new(IncludeCache::new_with_revalidation_interval(
        config.engine.script_cache_shards,
        include_revalidation_interval_from_env().unwrap_or(SERVER_INCLUDE_REVALIDATION_INTERVAL),
    ));
    // Deployment-root fingerprint: metadata + counters only. A root that
    // cannot be observed counts as `deployment_fingerprint_missing` and keeps
    // every fingerprint-gated persistent reuse blocked.
    include_cache.set_deployment_root_fingerprint(DeploymentRootFingerprint::observe(
        &docroot,
        config.engine.deployment_mode,
    ));
    let engine = Arc::new(ServerEngineState::new(
        engine_profile,
        config.engine.native_cache,
        config.engine.native_cache_dir.clone(),
        script_cache,
        include_cache,
        config.engine.perf_ablation,
    ));
    let metrics = Arc::new(ServerMetrics::default());
    let mut prepared_tls = build_tls(
        &config.transport.tls_mode,
        config.transport.http3_enabled,
        u32::try_from(config.transport.max_streams_per_connection)
            .expect("validated max_streams_per_connection fits u32"),
        Duration::from_millis(config.transport.connection_idle_timeout_ms),
    )?;
    let acme_status = match prepared_tls.tcp.as_ref() {
        Some(TcpTls::Acme(acme)) => Some(Arc::clone(&acme.status)),
        _ => None,
    };
    let acme_enabled = acme_status.is_some();
    if let TlsMode::Acme(acme) = &config.transport.tls_mode {
        eprintln!("{}", acme.startup_summary());
    }
    metrics
        .readiness_state
        .store(u64::from(!acme_enabled), Ordering::Release);
    metrics
        .acme_enabled
        .store(u64::from(acme_enabled), Ordering::Release);
    let static_files = Arc::new(
        StaticFileService::new(
            docroot.clone(),
            config.engine.deployment_mode,
            config.routing.indexes,
            config.routing.php_extensions,
            Arc::clone(&metrics),
        )
        .map_err(ServerError::Io)?,
    );
    let listener = TcpListener::bind(config.transport.listen).await?;
    metrics
        .tcp_listener_binds_total
        .fetch_add(1, Ordering::Relaxed);
    metrics.http_listener_count.store(1, Ordering::Release);
    let local_addr = listener.local_addr()?;
    debug!(%local_addr, docroot=%docroot.display(), "starting phrust server listener");
    let http3_listen = config.transport.http3_listen.unwrap_or(local_addr);
    let http3_endpoint = if config.transport.http3_enabled {
        let quic = prepared_tls.quic.take().ok_or_else(|| {
            ConfigError::new("HTTP/3 requires a prepared Manual or ACME TLS configuration")
        })?;
        Some(build_http3_endpoint(quic, http3_listen)?)
    } else {
        None
    };
    let http3_local_addr = http3_endpoint
        .as_ref()
        .map(|endpoint| endpoint.local_addr())
        .transpose()?;
    metrics
        .quic_endpoint_count
        .store(u64::from(http3_endpoint.is_some()), Ordering::Release);
    let http3_alt_svc = http3_local_addr.map(|addr| format!("h3=\":{}\"; ma=86400", addr.port()));
    let state = Arc::new(AppState {
        route_config: RouteConfig {
            docroot,
            front_controller: config.routing.front_controller,
            builtin_router: config.routing.builtin_router,
            request_rewrites: config.routing.request_rewrites,
            metrics_endpoint_enabled: config.routing.metrics_endpoint_enabled,
            cache_clear_endpoint_enabled: config.routing.cache_clear_endpoint_enabled,
        },
        static_files,
        request: RequestRuntimeConfig {
            max_body_bytes: config.limits.max_body_bytes,
            post_max_bytes: config.limits.post_max_bytes,
            request_body_memory_bytes: config.limits.request_body_memory_bytes,
            request_body_temp_dir: config.limits.request_body_temp_dir,
            enable_post_data_reading: config.limits.enable_post_data_reading,
            multipart_config: MultipartConfig {
                upload_temp_dir: config.sessions_uploads.upload_temp_dir,
                max_body_bytes: config.limits.max_body_bytes,
                post_max_bytes: config.limits.post_max_bytes,
                max_upload_files: config.sessions_uploads.max_upload_files,
                max_upload_file_bytes: config.sessions_uploads.max_upload_file_bytes,
                max_multipart_parts: config.sessions_uploads.max_multipart_parts,
                max_input_vars: config.sessions_uploads.max_input_vars,
                file_uploads: config.sessions_uploads.file_uploads,
                throw_limit_errors: false,
            },
            request_admission_timeout: Duration::from_millis(
                config.limits.request_admission_timeout_ms,
            ),
            cpu_queue_timeout: Duration::from_millis(config.limits.cpu_queue_timeout_ms),
            request_body_timeout: Duration::from_millis(config.limits.request_body_timeout_ms),
            request_body_idle_timeout: Duration::from_millis(
                config.limits.request_body_idle_timeout_ms,
            ),
            execution_time_limit: config
                .limits
                .execution_deadline_enabled
                .then(|| Duration::from_millis(config.limits.max_execution_ms)),
        },
        concurrency: ConcurrencyServices {
            in_flight: Arc::new(Semaphore::new(config.limits.max_in_flight)),
            max_in_flight: config.limits.max_in_flight,
            cpu_execution: Arc::new(Semaphore::new(config.limits.cpu_execution_limit)),
            cpu_execution_limit: config.limits.cpu_execution_limit,
            php_workers: Arc::new(crate::worker_pool::PhpWorkerPool::new(
                config.limits.cpu_execution_limit,
            )),
        },
        connections: ConnectionServices {
            permits: Arc::new(Semaphore::new(max_connections)),
            handshake_permits: Arc::new(Semaphore::new(max_connections.min(128))),
            max_connections,
            shutdown: ShutdownCoordinator::new(),
        },
        observability: ObservabilityState {
            metrics_token: config.observability.metrics_token,
            access_log,
            perf_trace,
            perf_trace_vm_counters: config.observability.perf_trace_vm_counters,
            request_profile,
            request_profile_vm_counters: config.observability.request_profile_vm_counters,
            request_profile_trigger_header: config.observability.request_profile_trigger_header,
            debug: config.observability.debug,
            error_format: config.observability.error_format,
            debug_log: config.observability.debug_log,
        },
        capabilities: CapabilityState {
            network_requests_enabled: config.capabilities.network_requests_enabled,
            env_snapshot: server_env_snapshot(std::env::vars()),
        },
        sessions: SessionServices {
            config: SessionConfig {
                enabled: config.sessions_uploads.sessions_enabled,
                cookie_name: config.sessions_uploads.session_cookie_name,
                serialize_handler: config.sessions_uploads.session_serialize_handler,
                use_strict_mode: config.sessions_uploads.session_use_strict_mode,
                cookie_lifetime: config.sessions_uploads.session_cookie_lifetime,
                cookie_path: config.sessions_uploads.session_cookie_path,
                cookie_domain: config.sessions_uploads.session_cookie_domain,
                cookie_secure: config.sessions_uploads.session_cookie_secure,
                cookie_httponly: config.sessions_uploads.session_cookie_httponly,
                cookie_samesite: config.sessions_uploads.session_cookie_samesite,
                cookie_partitioned: config.sessions_uploads.session_cookie_partitioned,
                use_cookies: config.sessions_uploads.session_use_cookies,
                use_only_cookies: config.sessions_uploads.session_use_only_cookies,
                save_path: config.sessions_uploads.session_save_path,
            },
            session_store,
        },
        transport: RequestTransport {
            local_addr,
            request_scheme: if startup_tls_enabled { "https" } else { "http" },
            http3_alt_svc,
            limits: TransportLimits {
                request_header_timeout: Duration::from_millis(
                    config.transport.request_header_timeout_ms,
                ),
                response_write_idle_timeout: Duration::from_millis(
                    config.transport.response_write_idle_timeout_ms,
                ),
                connection_idle_timeout: Duration::from_millis(
                    config.transport.connection_idle_timeout_ms,
                ),
                tls_handshake_timeout: Duration::from_millis(
                    config.transport.tls_handshake_timeout_ms,
                ),
                graceful_shutdown_timeout: Duration::from_millis(
                    config.transport.graceful_shutdown_timeout_ms,
                ),
                max_request_header_bytes: config.transport.max_request_header_bytes,
                max_request_target_bytes: config.transport.max_request_target_bytes,
                max_streams_per_connection: u32::try_from(
                    config.transport.max_streams_per_connection,
                )
                .expect("validated max_streams_per_connection fits u32"),
            },
        },
        services: RuntimeServices {
            metrics,
            engine,
            request_counter: Arc::new(AtomicU64::new(0)),
            tokio_handle: tokio::runtime::Handle::current(),
        },
        acme_status,
    });
    preload_script_cache(&state, script_cache_preload.as_deref(), strict_preload)?;
    debug!(
        max_connections = state.connections.max_connections,
        "transport connection budget initialized"
    );
    let startup_scheme = if startup_tls_enabled { "https" } else { "http" };
    println!("listening {startup_scheme}://{local_addr}");
    eprintln!(
        "startup docroot={} front_controller={} engine_preset={} script_cache={} script_cache_shards={} script_cache_max_entries={} upload_temp_dir={} session_save_path={} metrics_endpoint={} metrics_token={} access_log={} perf_trace={} request_profile={} tls={} tls_mode={} tls_alpn={} http3={} http3_addr={}",
        state.route_config.docroot.display(),
        startup_front_controller
            .as_ref()
            .map_or("-", |path| path.to_str().unwrap_or("<non-utf8>")),
        engine_profile,
        startup_script_cache_enabled,
        startup_script_cache_shards,
        startup_script_cache_max_entries,
        startup_upload_temp_dir.display(),
        startup_session_save_path.display(),
        startup_metrics_endpoint_enabled,
        startup_metrics_token_enabled,
        startup_access_log.as_deref().unwrap_or("-"),
        startup_perf_trace
            .as_ref()
            .map_or("-", |path| path.to_str().unwrap_or("<non-utf8>")),
        startup_request_profile
            .as_ref()
            .map_or("-", |path| path.to_str().unwrap_or("<non-utf8>")),
        startup_tls_enabled,
        startup_tls_mode,
        if startup_tls_enabled {
            "h2,http/1.1"
        } else {
            "-"
        },
        startup_http3_enabled,
        http3_local_addr
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    let native_isa = php_vm::api::cranelift_host_isa_identity()
        .map_err(|error| ServerError::Io(std::io::Error::other(error)))?;
    // phrust-diagnostics-allow: approved top-level native startup metadata stderr
    eprintln!(
        "native_startup compiler=cranelift compiler_version={} runtime_abi={:016x} helper_abi={:016x} target={} cpu_features={:016x} cache_mode={} cache_path={} preset={} artifacts_loaded=0 artifacts_compiled={}",
        php_vm::api::CRANELIFT_VERSION,
        php_vm::api::JIT_RUNTIME_ABI_HASH,
        php_vm::api::JIT_HELPER_REGISTRY_ABI_HASH,
        native_isa.target_triple,
        native_isa.feature_fingerprint,
        state.services.engine.native_cache.as_str(),
        state.services.engine.native_cache_dir.display(),
        state.services.engine.engine_profile,
        state
            .services
            .metrics
            .native_prewarm_entries
            .load(Ordering::Relaxed),
    );
    serve_until_shutdown(
        listener,
        state,
        prepared_tls.tcp,
        http3_endpoint,
        prepared_tls.acme_manager,
    )
    .await;
    Ok(())
}

fn prepare_private_temp_root(path: &Path) -> Result<(), ServerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ServerError::Io(std::io::Error::other(format!(
                    "temporary root `{}` must be a real directory, not a symlink",
                    path.display()
                ))));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            }
        }
        Err(error) => return Err(ServerError::Io(error)),
    }
    let probe = tempfile::Builder::new()
        .prefix(".phrust-probe-")
        .tempfile_in(path)?;
    drop(probe);
    Ok(())
}

pub fn run_blocking(config: ServerConfig) -> Result<(), ServerError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ServerError::Io)?;
    runtime.block_on(run(config))
}
