use std::{
    collections::HashMap,
    env, fmt, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use php_diagnostics::{
    DiagnosticEnvelope, DiagnosticLayer, DiagnosticOutputFormat, DiagnosticPhase,
    DiagnosticSeverity, DiagnosticSuggestion,
};
use php_executor::EngineProfileName;
use php_vm::api::{DeploymentRootMode, NativeCacheConfig, NativeCacheMode};

use crate::routing::RequestRewriteRule;

const DEFAULT_LISTEN: &str = "127.0.0.1:8080";
const DEFAULT_INDEX: &str = "index.php,index.html";
const DEFAULT_PHP_EXTENSIONS: &str = "php";
const DEFAULT_MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const DEFAULT_POST_MAX_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_REQUEST_BODY_MEMORY_BYTES: usize = 256 * 1024;
const DEFAULT_MAX_UPLOAD_FILES: usize = 20;
const DEFAULT_MAX_UPLOAD_FILE_BYTES: usize = 2 * 1024 * 1024;
const DEFAULT_MAX_INPUT_VARS: usize = 1_000;
const DEFAULT_MAX_CONNECTIONS: usize = 1_024;
const DEFAULT_REQUEST_ADMISSION_TIMEOUT_MS: u64 = 500;
const DEFAULT_CPU_QUEUE_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_REQUEST_HEADER_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_REQUEST_BODY_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_REQUEST_BODY_IDLE_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_RESPONSE_WRITE_IDLE_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_CONNECTION_IDLE_TIMEOUT_MS: u64 = 75_000;
const DEFAULT_TLS_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_MAX_REQUEST_HEADER_BYTES: usize = 65_536;
const DEFAULT_MAX_REQUEST_TARGET_BYTES: usize = 16_384;
const DEFAULT_MAX_STREAMS_PER_CONNECTION: usize = 100;
const DEFAULT_MAX_EXECUTION_MS: u64 = 30_000;
const DEFAULT_SCRIPT_CACHE_SHARDS: usize = 16;
const DEFAULT_SCRIPT_CACHE_MAX_ENTRIES: usize = 4096;
// Serve cached entry scripts without canonicalize/stat/retain for the same
// opcache.revalidate_freq window the include cache defaults to; 0 restores
// per-request metadata validation.
const DEFAULT_SCRIPT_CACHE_CHECK_INTERVAL_MS: u64 = 2_000;
const DEFAULT_SESSION_COOKIE_NAME: &str = "PHPSESSID";
const DEFAULT_SESSION_COOKIE_PATH: &str = "/";
const DEFAULT_MAX_IN_FLIGHT: usize = 200;
pub(crate) const BUILTIN_SERVER_REWRITE_PREFIX_QUERY_ENV: &str =
    "PHRUST_SERVER_REWRITE_PREFIX_QUERY";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerConfig {
    pub transport: TransportConfig,
    pub routing: ServerRoutingConfig,
    pub limits: RequestLimitsConfig,
    pub engine: EngineConfig,
    pub observability: ObservabilityConfig,
    pub sessions_uploads: SessionsUploadsConfig,
    pub capabilities: CapabilityConfig,
    pub help: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportConfig {
    pub listen: SocketAddr,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub http3_enabled: bool,
    pub http3_listen: Option<SocketAddr>,
    pub max_connections: usize,
    pub request_header_timeout_ms: u64,
    pub response_write_idle_timeout_ms: u64,
    pub connection_idle_timeout_ms: u64,
    pub tls_handshake_timeout_ms: u64,
    pub graceful_shutdown_timeout_ms: u64,
    pub max_request_header_bytes: usize,
    pub max_request_target_bytes: usize,
    pub max_streams_per_connection: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerRoutingConfig {
    pub docroot: PathBuf,
    pub indexes: Vec<String>,
    pub php_extensions: Vec<String>,
    pub front_controller: Option<PathBuf>,
    pub builtin_router: Option<PathBuf>,
    pub request_rewrites: Vec<RequestRewriteRule>,
    pub metrics_endpoint_enabled: bool,
    pub cache_clear_endpoint_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestLimitsConfig {
    pub max_body_bytes: usize,
    pub post_max_bytes: usize,
    pub request_body_memory_bytes: usize,
    pub request_body_temp_dir: PathBuf,
    pub enable_post_data_reading: bool,
    pub max_in_flight: usize,
    pub cpu_execution_limit: usize,
    pub request_admission_timeout_ms: u64,
    pub cpu_queue_timeout_ms: u64,
    pub request_body_timeout_ms: u64,
    pub request_body_idle_timeout_ms: u64,
    pub max_execution_ms: u64,
    pub execution_deadline_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineConfig {
    pub engine_preset: EngineProfileName,
    pub native_cache: NativeCacheMode,
    pub native_cache_dir: PathBuf,
    /// Declared mutability of the deployment root. `dev` (default) marks the
    /// docroot as mutable, which keeps every deployment-fingerprint-gated
    /// persistent reuse blocked; `immutable` is an operator declaration for
    /// atomically swapped release directories whose cached source remains
    /// trusted until cache clear or restart.
    pub deployment_mode: DeploymentRootMode,
    pub perf_ablation: ServerPerfAblation,
    pub script_cache_enabled: bool,
    pub script_cache_shards: usize,
    pub script_cache_max_entries: usize,
    pub script_cache_preload: Option<PathBuf>,
    pub script_cache_check_interval_ms: u64,
    pub strict_preload: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservabilityConfig {
    pub debug: bool,
    pub error_format: DiagnosticOutputFormat,
    pub debug_log: Option<PathBuf>,
    pub metrics_token: Option<String>,
    pub access_log: Option<String>,
    pub perf_trace: Option<PathBuf>,
    pub perf_trace_vm_counters: bool,
    pub request_profile: Option<PathBuf>,
    pub request_profile_vm_counters: bool,
    pub request_profile_trigger_header: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionsUploadsConfig {
    pub upload_temp_dir: PathBuf,
    pub max_upload_files: usize,
    pub max_upload_file_bytes: usize,
    pub max_multipart_parts: Option<usize>,
    pub max_input_vars: usize,
    pub file_uploads: bool,
    pub session_save_path: PathBuf,
    pub session_cookie_name: String,
    pub session_serialize_handler: String,
    pub session_use_strict_mode: bool,
    pub session_cookie_lifetime: u64,
    pub session_cookie_path: String,
    pub session_cookie_domain: String,
    pub session_cookie_secure: bool,
    pub session_cookie_httponly: bool,
    pub session_cookie_samesite: String,
    pub session_cookie_partitioned: bool,
    pub session_use_cookies: bool,
    pub session_use_only_cookies: bool,
    pub sessions_enabled: bool,
    pub session_lock_timeout_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityConfig {
    pub network_requests_enabled: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServerPerfAblation {
    pub disable_inline_caches: bool,
    pub disable_include_o2: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    message: String,
    diagnostic: Box<DiagnosticEnvelope>,
}

impl ConfigError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let mut diagnostic = DiagnosticEnvelope::new(
            "E_PHRUST_SERVER_CONFIG",
            DiagnosticLayer::server(),
            DiagnosticPhase::new("config"),
            DiagnosticSeverity::Error,
            message.clone(),
        );
        diagnostic.suggestion = Some(DiagnosticSuggestion::new(
            "run phrust-server --help and check the configured flag or path",
        ));
        Self {
            message,
            diagnostic: Box::new(diagnostic),
        }
    }

    pub fn diagnostic(&self) -> &DiagnosticEnvelope {
        self.diagnostic.as_ref()
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

impl ServerConfig {
    pub fn parse_env() -> Result<Self, ConfigError> {
        Self::parse_from(env::args().skip(1))
    }

    pub fn parse_from<I, S>(args: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::parse_from_with_env(args, |name| env::var(name).ok())
    }

    fn parse_from_with_env<I, S, F>(args: I, env_value: F) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        F: Fn(&str) -> Option<String>,
    {
        let raw_args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let help_requested = raw_args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--help" | "-h"));
        let file_config = if help_requested {
            FileConfig::default()
        } else if let Some(path) = config_path_from_args(&raw_args)? {
            FileConfig::load(&path)?
        } else {
            FileConfig::default()
        };

        let mut listen = file_config
            .parse_listen("listen")?
            .unwrap_or(parse_listen(DEFAULT_LISTEN)?);
        let mut docroot = file_config.path("docroot");
        let mut indexes = file_config
            .string("index")
            .map(|value| parse_indexes("index", &value))
            .transpose()?
            .unwrap_or_else(|| {
                parse_indexes("index", DEFAULT_INDEX).expect("default indexes are valid")
            });
        let mut php_extensions = file_config
            .string("php_extensions")
            .map(|value| parse_php_extensions("php_extensions", &value))
            .transpose()?
            .unwrap_or_else(|| {
                parse_php_extensions("php_extensions", DEFAULT_PHP_EXTENSIONS)
                    .expect("default PHP extensions are valid")
            });
        let mut front_controller = file_config.path("front_controller");
        let mut request_rewrites = file_config.request_rewrites("rewrite_prefix_query")?;
        let mut max_body_bytes = file_config
            .positive_usize("max_body_bytes")?
            .unwrap_or(DEFAULT_MAX_BODY_BYTES);
        let mut post_max_bytes_explicit = file_config.values.contains_key("post_max_bytes");
        let mut post_max_bytes = file_config
            .positive_usize("post_max_bytes")?
            .unwrap_or(DEFAULT_POST_MAX_BYTES.min(max_body_bytes));
        let mut request_body_memory_bytes_explicit =
            file_config.values.contains_key("request_body_memory_bytes");
        let mut request_body_memory_bytes = file_config
            .positive_usize("request_body_memory_bytes")?
            .unwrap_or(DEFAULT_REQUEST_BODY_MEMORY_BYTES.min(max_body_bytes));
        let mut request_body_temp_dir = file_config
            .path("request_body_temp_dir")
            .unwrap_or_else(|| std::env::temp_dir().join("phrust-request-bodies"));
        let mut enable_post_data_reading = file_config
            .bool("enable_post_data_reading")?
            .unwrap_or(true);
        let mut enable_post_data_reading_flag = false;
        let mut disable_post_data_reading_flag = false;
        let mut upload_temp_dir = file_config
            .path("upload_temp_dir")
            .unwrap_or_else(|| std::env::temp_dir().join("phrust-uploads"));
        let mut max_upload_files = file_config
            .positive_usize("max_upload_files")?
            .unwrap_or(DEFAULT_MAX_UPLOAD_FILES);
        let mut max_upload_file_bytes = file_config.positive_usize("max_upload_file_bytes")?;
        let mut max_multipart_parts = file_config
            .string("max_multipart_parts")
            .map(|value| parse_max_multipart_parts("max_multipart_parts", &value))
            .transpose()?
            .flatten();
        let mut max_input_vars = file_config
            .positive_usize("max_input_vars")?
            .unwrap_or(DEFAULT_MAX_INPUT_VARS);
        let mut file_uploads = file_config.bool("file_uploads")?.unwrap_or(true);
        let mut session_save_path = file_config
            .path("session_save_path")
            .unwrap_or_else(|| std::env::temp_dir().join("phrust-sessions"));
        let mut session_cookie_name = file_config
            .string("session_cookie_name")
            .unwrap_or_else(|| DEFAULT_SESSION_COOKIE_NAME.to_string());
        let mut session_serialize_handler = file_config
            .string("session_serialize_handler")
            .unwrap_or_else(|| "php".to_string());
        let mut session_use_strict_mode = file_config
            .bool("session_use_strict_mode")?
            .unwrap_or(false);
        let mut session_cookie_lifetime = file_config
            .nonnegative_u64("session_cookie_lifetime")?
            .unwrap_or(0);
        let mut session_cookie_path = file_config
            .string("session_cookie_path")
            .unwrap_or_else(|| DEFAULT_SESSION_COOKIE_PATH.to_string());
        let mut session_cookie_domain = file_config
            .string("session_cookie_domain")
            .unwrap_or_default();
        let mut session_cookie_secure = file_config.bool("session_cookie_secure")?.unwrap_or(false);
        let mut session_cookie_httponly =
            file_config.bool("session_cookie_httponly")?.unwrap_or(true);
        let mut session_cookie_samesite = file_config
            .string("session_cookie_samesite")
            .unwrap_or_default();
        let mut session_cookie_partitioned = file_config
            .bool("session_cookie_partitioned")?
            .unwrap_or(false);
        let mut session_use_cookies = file_config.bool("session_use_cookies")?.unwrap_or(true);
        let mut session_use_only_cookies = file_config
            .bool("session_use_only_cookies")?
            .unwrap_or(true);
        let mut sessions_enabled = file_config.bool("sessions_enabled")?.unwrap_or(true);
        let mut session_lock_timeout_ms = file_config
            .positive_u64("session_lock_timeout_ms")?
            .unwrap_or(5_000);
        if file_config.string("request_timeout_ms").is_some() {
            return Err(ConfigError::new(
                "request_timeout_ms was removed; migrate body reads to request_body_timeout_ms and CPU admission to cpu_queue_timeout_ms",
            ));
        }
        let mut max_in_flight = file_config
            .positive_usize("max_in_flight")?
            .unwrap_or_else(default_max_in_flight);
        let mut cpu_execution_limit = file_config
            .positive_usize("cpu_execution_limit")?
            .unwrap_or_else(default_cpu_execution_limit);
        let mut request_admission_timeout_ms = file_config
            .positive_u64("request_admission_timeout_ms")?
            .unwrap_or(DEFAULT_REQUEST_ADMISSION_TIMEOUT_MS);
        let mut cpu_queue_timeout_ms = file_config
            .positive_u64("cpu_queue_timeout_ms")?
            .unwrap_or(DEFAULT_CPU_QUEUE_TIMEOUT_MS);
        let mut request_body_timeout_ms = file_config
            .positive_u64("request_body_timeout_ms")?
            .unwrap_or(DEFAULT_REQUEST_BODY_TIMEOUT_MS);
        let mut request_body_idle_timeout_ms = file_config
            .positive_u64("request_body_idle_timeout_ms")?
            .unwrap_or(DEFAULT_REQUEST_BODY_IDLE_TIMEOUT_MS);
        let mut max_execution_ms = file_config
            .positive_u64("max_execution_ms")?
            .unwrap_or(DEFAULT_MAX_EXECUTION_MS);
        let mut execution_deadline_enabled = file_config
            .bool("execution_deadline_enabled")?
            .unwrap_or(true);
        let mut engine_preset = file_config
            .string("engine_preset")
            .map(|value| parse_engine_preset("engine_preset", &value))
            .transpose()?
            .unwrap_or_default();
        let native_cache_defaults = NativeCacheConfig::default();
        let mut native_cache = file_config
            .string("native_cache")
            .or_else(|| env_value("PHRUST_NATIVE_CACHE"))
            .map(|value| parse_native_cache("native_cache", &value))
            .transpose()?
            .unwrap_or(native_cache_defaults.mode);
        let mut native_cache_dir = file_config
            .path("native_cache_dir")
            .or_else(|| env_value("PHRUST_NATIVE_CACHE_DIR").map(PathBuf::from))
            .unwrap_or(native_cache_defaults.directory);
        let mut deployment_mode = file_config
            .string("deployment_mode")
            .map(|value| parse_deployment_mode("deployment_mode", &value))
            .transpose()?
            .unwrap_or(DeploymentRootMode::DevMutable);
        let file_perf_ablation = file_config
            .string("perf_ablation")
            .map(|value| parse_perf_ablation("perf_ablation", &value))
            .transpose()?;
        let env_perf_ablation = env_value("PHRUST_PERF_ABLATION")
            .map(|value| parse_perf_ablation("PHRUST_PERF_ABLATION", &value))
            .transpose()?;
        let mut perf_ablation = file_perf_ablation.or(env_perf_ablation).unwrap_or_default();
        let mut metrics_endpoint_enabled = file_config
            .bool("metrics_endpoint_enabled")?
            .unwrap_or(true);
        let mut metrics_token = file_config.string("metrics_token");
        let mut tls_cert = file_config.path("tls_cert");
        let mut tls_key = file_config.path("tls_key");
        let mut http3_enabled = file_config.bool("http3_enabled")?.unwrap_or(false);
        let mut http3_listen = file_config.parse_listen("http3_listen")?;
        let mut max_connections = file_config
            .positive_usize("max_connections")?
            .unwrap_or(DEFAULT_MAX_CONNECTIONS);
        let mut request_header_timeout_ms = file_config
            .positive_u64("request_header_timeout_ms")?
            .unwrap_or(DEFAULT_REQUEST_HEADER_TIMEOUT_MS);
        let mut response_write_idle_timeout_ms = file_config
            .positive_u64("response_write_idle_timeout_ms")?
            .unwrap_or(DEFAULT_RESPONSE_WRITE_IDLE_TIMEOUT_MS);
        let mut connection_idle_timeout_ms = file_config
            .positive_u64("connection_idle_timeout_ms")?
            .unwrap_or(DEFAULT_CONNECTION_IDLE_TIMEOUT_MS);
        let mut tls_handshake_timeout_ms = file_config
            .positive_u64("tls_handshake_timeout_ms")?
            .unwrap_or(DEFAULT_TLS_HANDSHAKE_TIMEOUT_MS);
        let mut graceful_shutdown_timeout_ms = file_config
            .positive_u64("graceful_shutdown_timeout_ms")?
            .unwrap_or(DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_MS);
        let mut max_request_header_bytes = file_config
            .positive_usize("max_request_header_bytes")?
            .unwrap_or(DEFAULT_MAX_REQUEST_HEADER_BYTES);
        let mut max_request_target_bytes = file_config
            .positive_usize("max_request_target_bytes")?
            .unwrap_or(DEFAULT_MAX_REQUEST_TARGET_BYTES);
        let mut max_streams_per_connection = file_config
            .positive_usize("max_streams_per_connection")?
            .unwrap_or(DEFAULT_MAX_STREAMS_PER_CONNECTION);
        let mut script_cache_enabled = file_config.bool("script_cache_enabled")?.unwrap_or(true);
        let mut script_cache_shards = file_config
            .positive_usize("script_cache_shards")?
            .unwrap_or(DEFAULT_SCRIPT_CACHE_SHARDS);
        let mut script_cache_max_entries = file_config
            .positive_usize("script_cache_max_entries")?
            .unwrap_or(DEFAULT_SCRIPT_CACHE_MAX_ENTRIES);
        let mut script_cache_preload = file_config.path("script_cache_preload");
        let mut script_cache_check_interval_ms = file_config
            .nonnegative_u64("script_cache_check_interval_ms")?
            .unwrap_or(DEFAULT_SCRIPT_CACHE_CHECK_INTERVAL_MS);
        let mut strict_preload = file_config.bool("strict_preload")?.unwrap_or(false);
        let mut cache_clear_endpoint_enabled = file_config
            .bool("cache_clear_endpoint_enabled")?
            .unwrap_or(false);
        let mut access_log = file_config.string("access_log");
        let mut perf_trace = file_config
            .path("perf_trace")
            .or_else(|| env_perf_trace_path(&env_value));
        let mut perf_trace_vm_counters = file_config
            .bool("perf_trace_vm_counters")?
            .unwrap_or_else(|| env_bool(&env_value, "PHRUST_SERVER_PERF_TRACE_VM_COUNTERS"));
        let mut request_profile = file_config
            .path("request_profile")
            .or_else(|| env_request_profile_path(&env_value));
        let mut request_profile_vm_counters = file_config
            .bool("request_profile_vm_counters")?
            .unwrap_or_else(|| env_bool(&env_value, "PHRUST_REQUEST_PROFILE_VM_COUNTERS"));
        let mut request_profile_trigger_header = file_config
            .bool("request_profile_trigger_header")?
            .unwrap_or_else(|| {
                env_value("PHRUST_REQUEST_PROFILE_TRIGGER_HEADER").is_none_or(|value| {
                    matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on")
                })
            });
        let mut network_requests_enabled = file_config
            .bool("network_requests_enabled")?
            .unwrap_or_else(|| env_bool(&env_value, "PHRUST_SERVER_ENABLE_NETWORK_REQUESTS"));
        let mut debug = env_bool(&env_value, "PHRUST_SERVER_DEBUG");
        let mut error_format = env_output_format(&env_value, "PHRUST_SERVER_ERROR_FORMAT")?;
        let mut debug_log = env_value("PHRUST_SERVER_DEBUG_LOG")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let mut help = false;
        let mut args = raw_args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => help = true,
                "--config" => {
                    let _ = required_value(&arg, &mut args)?;
                }
                "--listen" => listen = parse_listen(&required_value(&arg, &mut args)?)?,
                "--max-connections" => {
                    max_connections =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--docroot" => docroot = Some(PathBuf::from(required_value(&arg, &mut args)?)),
                "--index" => {
                    indexes = parse_indexes(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--php-extensions" => {
                    php_extensions = parse_php_extensions(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--front-controller" => {
                    let value = required_value(&arg, &mut args)?;
                    let path = PathBuf::from(value);
                    validate_relative_path("--front-controller", &path)?;
                    front_controller = Some(path);
                }
                "--rewrite-prefix-query" => {
                    request_rewrites.push(parse_request_rewrite_rule(
                        &arg,
                        &required_value(&arg, &mut args)?,
                    )?);
                }
                "--max-body-bytes" => {
                    max_body_bytes = parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--post-max-bytes" => {
                    post_max_bytes = parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                    post_max_bytes_explicit = true;
                }
                "--request-body-memory-bytes" => {
                    request_body_memory_bytes =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                    request_body_memory_bytes_explicit = true;
                }
                "--request-body-temp-dir" => {
                    request_body_temp_dir = PathBuf::from(required_value(&arg, &mut args)?);
                }
                "--enable-post-data-reading" => {
                    enable_post_data_reading = true;
                    enable_post_data_reading_flag = true;
                }
                "--disable-post-data-reading" => {
                    enable_post_data_reading = false;
                    disable_post_data_reading_flag = true;
                }
                "--upload-temp-dir" => {
                    upload_temp_dir = PathBuf::from(required_value(&arg, &mut args)?);
                }
                "--max-upload-files" => {
                    max_upload_files =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-upload-file-bytes" => {
                    max_upload_file_bytes = Some(parse_positive_usize(
                        &arg,
                        &required_value(&arg, &mut args)?,
                    )?);
                }
                "--max-multipart-parts" => {
                    max_multipart_parts =
                        parse_max_multipart_parts(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-input-vars" => {
                    max_input_vars = parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--enable-file-uploads" => file_uploads = true,
                "--disable-file-uploads" => file_uploads = false,
                "--session-save-path" => {
                    session_save_path = PathBuf::from(required_value(&arg, &mut args)?);
                }
                "--session-cookie-name" => {
                    session_cookie_name = required_value(&arg, &mut args)?;
                    validate_cookie_name("--session-cookie-name", &session_cookie_name)?;
                }
                "--session-serialize-handler" => {
                    session_serialize_handler = required_value(&arg, &mut args)?;
                }
                "--enable-session-strict-mode" => session_use_strict_mode = true,
                "--disable-session-strict-mode" => session_use_strict_mode = false,
                "--session-cookie-lifetime" => {
                    session_cookie_lifetime =
                        parse_nonnegative_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--session-lock-timeout-ms" => {
                    session_lock_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--session-cookie-path" => {
                    session_cookie_path = required_value(&arg, &mut args)?;
                    validate_cookie_path("--session-cookie-path", &session_cookie_path)?;
                }
                "--session-cookie-domain" => {
                    session_cookie_domain = required_value(&arg, &mut args)?;
                }
                "--enable-session-cookie-secure" => session_cookie_secure = true,
                "--disable-session-cookie-secure" => session_cookie_secure = false,
                "--enable-session-cookie-httponly" => session_cookie_httponly = true,
                "--disable-session-cookie-httponly" => session_cookie_httponly = false,
                "--session-cookie-samesite" => {
                    session_cookie_samesite = required_value(&arg, &mut args)?;
                }
                "--enable-session-cookie-partitioned" => session_cookie_partitioned = true,
                "--disable-session-cookie-partitioned" => session_cookie_partitioned = false,
                "--enable-session-cookies" => session_use_cookies = true,
                "--disable-session-cookies" => session_use_cookies = false,
                "--enable-session-only-cookies" => session_use_only_cookies = true,
                "--disable-session-only-cookies" => session_use_only_cookies = false,
                "--disable-sessions" => sessions_enabled = false,
                "--max-in-flight" => {
                    max_in_flight = parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--cpu-execution-limit" => {
                    cpu_execution_limit =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--request-timeout-ms" => {
                    return Err(ConfigError::new(
                        "--request-timeout-ms was removed; use --request-body-timeout-ms and --cpu-queue-timeout-ms",
                    ));
                }
                "--request-admission-timeout-ms" => {
                    request_admission_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--cpu-queue-timeout-ms" => {
                    cpu_queue_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--request-header-timeout-ms" => {
                    request_header_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--request-body-timeout-ms" => {
                    request_body_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--request-body-idle-timeout-ms" => {
                    request_body_idle_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--response-write-idle-timeout-ms" => {
                    response_write_idle_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--connection-idle-timeout-ms" => {
                    connection_idle_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--tls-handshake-timeout-ms" => {
                    tls_handshake_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--graceful-shutdown-timeout-ms" => {
                    graceful_shutdown_timeout_ms =
                        parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-request-header-bytes" => {
                    max_request_header_bytes =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-request-target-bytes" => {
                    max_request_target_bytes =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-streams-per-connection" => {
                    max_streams_per_connection =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--max-execution-ms" => {
                    max_execution_ms = parse_positive_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--disable-execution-deadline" => execution_deadline_enabled = false,
                "--engine-preset" => {
                    engine_preset = parse_engine_preset(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--native-cache" => {
                    native_cache = parse_native_cache(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--native-cache-dir" => {
                    native_cache_dir = PathBuf::from(required_value(&arg, &mut args)?);
                }
                "--deployment-mode" => {
                    deployment_mode =
                        parse_deployment_mode(&arg, &required_value(&arg, &mut args)?)?;
                }
                _ if arg.starts_with("--deployment-mode=") => {
                    let value = arg.trim_start_matches("--deployment-mode=");
                    deployment_mode = parse_deployment_mode("--deployment-mode", value)?;
                }
                "--perf-ablation" => {
                    perf_ablation = parse_perf_ablation(&arg, &required_value(&arg, &mut args)?)?;
                }
                _ if arg.starts_with("--perf-ablation=") => {
                    let value = arg.trim_start_matches("--perf-ablation=");
                    perf_ablation = parse_perf_ablation("--perf-ablation", value)?;
                }
                "--disable-metrics-endpoint" => metrics_endpoint_enabled = false,
                "--metrics-token" => {
                    metrics_token = Some(required_value(&arg, &mut args)?);
                }
                "--tls-cert" => tls_cert = Some(PathBuf::from(required_value(&arg, &mut args)?)),
                "--tls-key" => tls_key = Some(PathBuf::from(required_value(&arg, &mut args)?)),
                "--enable-http3" => http3_enabled = true,
                "--http3-listen" => {
                    http3_listen = Some(parse_listen(&required_value(&arg, &mut args)?)?)
                }
                "--no-script-cache" => script_cache_enabled = false,
                "--script-cache-shards" => {
                    script_cache_shards =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--script-cache-max-entries" => {
                    script_cache_max_entries =
                        parse_positive_usize(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--script-cache-preload" => {
                    script_cache_preload = Some(PathBuf::from(required_value(&arg, &mut args)?));
                }
                "--script-cache-check-interval-ms" => {
                    script_cache_check_interval_ms =
                        parse_nonnegative_u64(&arg, &required_value(&arg, &mut args)?)?;
                }
                "--strict-preload" => strict_preload = true,
                "--enable-cache-clear-endpoint" => cache_clear_endpoint_enabled = true,
                "--access-log" => access_log = Some(required_value(&arg, &mut args)?),
                "--perf-trace" => {
                    perf_trace = Some(PathBuf::from(required_value(&arg, &mut args)?))
                }
                "--perf-trace-vm-counters" => perf_trace_vm_counters = true,
                "--request-profile" => {
                    request_profile = Some(PathBuf::from(required_value(&arg, &mut args)?))
                }
                "--request-profile-vm-counters" => request_profile_vm_counters = true,
                "--request-profile-trigger-header" => request_profile_trigger_header = true,
                "--enable-network-requests" => network_requests_enabled = true,
                "--debug" => debug = true,
                "--error-format" => {
                    error_format = parse_output_format(&required_value(&arg, &mut args)?)?;
                }
                "--debug-log" => debug_log = Some(PathBuf::from(required_value(&arg, &mut args)?)),
                _ if arg.starts_with('-') => {
                    return Err(ConfigError::new(format!(
                        "unknown flag `{arg}`; accepted flags include --docroot, --listen, --debug, --error-format, and --help"
                    )));
                }
                _ => return Err(ConfigError::new(format!("unexpected argument `{arg}`"))),
            }
        }

        let docroot = if help {
            docroot.unwrap_or_default()
        } else {
            docroot.ok_or_else(|| {
                ConfigError::new("--docroot is required; example: phrust-server --docroot public")
            })?
        };
        if !post_max_bytes_explicit {
            post_max_bytes = DEFAULT_POST_MAX_BYTES.min(max_body_bytes);
        }
        if !request_body_memory_bytes_explicit {
            request_body_memory_bytes = DEFAULT_REQUEST_BODY_MEMORY_BYTES.min(max_body_bytes);
        }
        if enable_post_data_reading_flag && disable_post_data_reading_flag {
            return Err(ConfigError::new(
                "--enable-post-data-reading and --disable-post-data-reading are mutually exclusive",
            ));
        }
        let config = Self {
            transport: TransportConfig {
                listen,
                tls_cert,
                tls_key,
                http3_enabled,
                http3_listen,
                max_connections,
                request_header_timeout_ms,
                response_write_idle_timeout_ms,
                connection_idle_timeout_ms,
                tls_handshake_timeout_ms,
                graceful_shutdown_timeout_ms,
                max_request_header_bytes,
                max_request_target_bytes,
                max_streams_per_connection,
            },
            routing: ServerRoutingConfig {
                docroot,
                indexes,
                php_extensions,
                front_controller,
                builtin_router: None,
                request_rewrites,
                metrics_endpoint_enabled,
                cache_clear_endpoint_enabled,
            },
            limits: RequestLimitsConfig {
                max_body_bytes,
                post_max_bytes,
                request_body_memory_bytes,
                request_body_temp_dir,
                enable_post_data_reading,
                max_in_flight,
                cpu_execution_limit,
                request_admission_timeout_ms,
                cpu_queue_timeout_ms,
                request_body_timeout_ms,
                request_body_idle_timeout_ms,
                max_execution_ms,
                execution_deadline_enabled,
            },
            engine: EngineConfig {
                engine_preset,
                native_cache,
                native_cache_dir,
                deployment_mode,
                perf_ablation,
                script_cache_enabled,
                script_cache_shards,
                script_cache_max_entries,
                script_cache_preload,
                script_cache_check_interval_ms,
                strict_preload,
            },
            observability: ObservabilityConfig {
                debug,
                error_format,
                debug_log,
                metrics_token,
                access_log,
                perf_trace,
                perf_trace_vm_counters,
                request_profile,
                request_profile_vm_counters,
                request_profile_trigger_header,
            },
            sessions_uploads: SessionsUploadsConfig {
                upload_temp_dir,
                max_upload_files,
                max_upload_file_bytes: max_upload_file_bytes
                    .unwrap_or(DEFAULT_MAX_UPLOAD_FILE_BYTES),
                max_multipart_parts,
                max_input_vars,
                file_uploads,
                session_save_path,
                session_cookie_name,
                session_serialize_handler,
                session_use_strict_mode,
                session_cookie_lifetime,
                session_cookie_path,
                session_cookie_domain,
                session_cookie_secure,
                session_cookie_httponly,
                session_cookie_samesite,
                session_cookie_partitioned,
                session_use_cookies,
                session_use_only_cookies,
                sessions_enabled,
                session_lock_timeout_ms,
            },
            capabilities: CapabilityConfig {
                network_requests_enabled,
            },
            help,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn builtin_cli_server(
        listen: &str,
        docroot: PathBuf,
        router: Option<PathBuf>,
    ) -> Result<Self, ConfigError> {
        Self::builtin_cli_server_with_env(listen, docroot, router, |name| env::var(name).ok())
    }

    fn builtin_cli_server_with_env<F>(
        listen: &str,
        docroot: PathBuf,
        router: Option<PathBuf>,
        env_get: F,
    ) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let listen = parse_listen(listen)?;
        let request_rewrites = env_get(BUILTIN_SERVER_REWRITE_PREFIX_QUERY_ENV)
            .map(|value| {
                parse_request_rewrite_rules(BUILTIN_SERVER_REWRITE_PREFIX_QUERY_ENV, &value)
            })
            .transpose()?
            .unwrap_or_default();
        let config = Self {
            transport: TransportConfig {
                listen,
                tls_cert: None,
                tls_key: None,
                http3_enabled: false,
                http3_listen: None,
                max_connections: DEFAULT_MAX_CONNECTIONS,
                request_header_timeout_ms: DEFAULT_REQUEST_HEADER_TIMEOUT_MS,
                response_write_idle_timeout_ms: DEFAULT_RESPONSE_WRITE_IDLE_TIMEOUT_MS,
                connection_idle_timeout_ms: DEFAULT_CONNECTION_IDLE_TIMEOUT_MS,
                tls_handshake_timeout_ms: DEFAULT_TLS_HANDSHAKE_TIMEOUT_MS,
                graceful_shutdown_timeout_ms: DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT_MS,
                max_request_header_bytes: DEFAULT_MAX_REQUEST_HEADER_BYTES,
                max_request_target_bytes: DEFAULT_MAX_REQUEST_TARGET_BYTES,
                max_streams_per_connection: DEFAULT_MAX_STREAMS_PER_CONNECTION,
            },
            routing: ServerRoutingConfig {
                docroot,
                indexes: parse_indexes("index", DEFAULT_INDEX).expect("default indexes are valid"),
                php_extensions: parse_php_extensions("php_extensions", DEFAULT_PHP_EXTENSIONS)
                    .expect("default PHP extensions are valid"),
                front_controller: None,
                builtin_router: router,
                request_rewrites,
                metrics_endpoint_enabled: false,
                cache_clear_endpoint_enabled: false,
            },
            limits: RequestLimitsConfig {
                max_body_bytes: DEFAULT_MAX_BODY_BYTES,
                post_max_bytes: DEFAULT_POST_MAX_BYTES,
                request_body_memory_bytes: DEFAULT_REQUEST_BODY_MEMORY_BYTES,
                request_body_temp_dir: std::env::temp_dir().join("phrust-request-bodies"),
                enable_post_data_reading: true,
                max_in_flight: default_max_in_flight(),
                cpu_execution_limit: default_cpu_execution_limit(),
                request_admission_timeout_ms: DEFAULT_REQUEST_ADMISSION_TIMEOUT_MS,
                cpu_queue_timeout_ms: DEFAULT_CPU_QUEUE_TIMEOUT_MS,
                request_body_timeout_ms: DEFAULT_REQUEST_BODY_TIMEOUT_MS,
                request_body_idle_timeout_ms: DEFAULT_REQUEST_BODY_IDLE_TIMEOUT_MS,
                max_execution_ms: DEFAULT_MAX_EXECUTION_MS,
                execution_deadline_enabled: true,
            },
            engine: EngineConfig {
                engine_preset: EngineProfileName::default(),
                native_cache: NativeCacheConfig::default().mode,
                native_cache_dir: NativeCacheConfig::default().directory,
                deployment_mode: DeploymentRootMode::DevMutable,
                perf_ablation: env_value_perf_ablation()?.unwrap_or_default(),
                script_cache_enabled: true,
                script_cache_shards: DEFAULT_SCRIPT_CACHE_SHARDS,
                script_cache_max_entries: DEFAULT_SCRIPT_CACHE_MAX_ENTRIES,
                script_cache_preload: None,
                script_cache_check_interval_ms: DEFAULT_SCRIPT_CACHE_CHECK_INTERVAL_MS,
                strict_preload: false,
            },
            observability: ObservabilityConfig {
                debug: false,
                error_format: DiagnosticOutputFormat::Text,
                debug_log: None,
                metrics_token: None,
                access_log: None,
                perf_trace: None,
                perf_trace_vm_counters: false,
                request_profile: None,
                request_profile_vm_counters: false,
                request_profile_trigger_header: true,
            },
            sessions_uploads: SessionsUploadsConfig {
                upload_temp_dir: std::env::temp_dir().join("phrust-uploads"),
                max_upload_files: DEFAULT_MAX_UPLOAD_FILES,
                max_upload_file_bytes: DEFAULT_MAX_UPLOAD_FILE_BYTES,
                max_multipart_parts: None,
                max_input_vars: DEFAULT_MAX_INPUT_VARS,
                file_uploads: true,
                session_save_path: std::env::temp_dir().join("phrust-sessions"),
                session_cookie_name: DEFAULT_SESSION_COOKIE_NAME.to_string(),
                session_serialize_handler: "php".to_string(),
                session_use_strict_mode: false,
                session_cookie_lifetime: 0,
                session_cookie_path: DEFAULT_SESSION_COOKIE_PATH.to_string(),
                session_cookie_domain: String::new(),
                session_cookie_secure: false,
                session_cookie_httponly: true,
                session_cookie_samesite: String::new(),
                session_cookie_partitioned: false,
                session_use_cookies: true,
                session_use_only_cookies: true,
                sessions_enabled: true,
                session_lock_timeout_ms: 5_000,
            },
            capabilities: CapabilityConfig {
                network_requests_enabled: false,
            },
            help: false,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn help_text() -> &'static str {
        "Usage: phrust-server --docroot <path> [options]\n\
\n\
Options:\n\
  --listen <addr>              TCP listen address (default: 127.0.0.1:8080)\n\
  --max-connections <n>        maximum active TCP/TLS/QUIC connections (default: 1024)\n\
  --config <path>              read simple TOML-style server config\n\
  --docroot <path>             document root (required unless --help)\n\
  --front-controller <path>    optional front controller, relative to docroot\n\
  --rewrite-prefix-query <p=q> rewrite matching request paths to /?q=<suffix>\n\
  --max-body-bytes <n>         transport hard limit (default: 33554432)\n\
  --post-max-bytes <n>         PHP post_max_size bytes (default: 8388608)\n\
  --request-body-memory-bytes <n> in-memory body threshold (default: 262144)\n\
  --request-body-temp-dir <path> request body spool directory\n\
  --enable-post-data-reading   enable automatic PHP POST parsing (default)\n\
  --disable-post-data-reading  leave POST bodies for php://input/request_parse_body\n\
  --upload-temp-dir <path>     upload temp directory (default: OS temp/phrust-uploads)\n\
  --max-upload-files <n>       maximum uploaded files per request (default: 20)\n\
  --max-upload-file-bytes <n>  maximum bytes per uploaded file (default: 2097152)\n\
  --max-multipart-parts <n|-1> maximum multipart parts (-1 uses PHP policy)\n\
  --max-input-vars <n>         maximum parsed input variables (default: 1000)\n\
  --enable-file-uploads        enable multipart file uploads (default)\n\
  --disable-file-uploads       ignore multipart file uploads\n\
  --session-save-path <path>   persistent files-session directory\n\
  --session-cookie-name <name> session cookie name (default: PHPSESSID)\n\
  --session-serialize-handler <name> php, php_binary, or php_serialize\n\
  --enable-session-strict-mode require incoming IDs to exist in the files store\n\
  --disable-session-strict-mode accept valid incoming IDs (default)\n\
  --session-cookie-lifetime <seconds> session cookie lifetime (default: 0)\n\
  --session-cookie-path <path> session cookie path (default: /)\n\
  --session-cookie-domain <domain> session cookie domain\n\
  --enable-session-cookie-secure / --disable-session-cookie-secure\n\
  --enable-session-cookie-httponly / --disable-session-cookie-httponly\n\
  --session-cookie-samesite <value> session cookie SameSite attribute\n\
  --enable-session-cookie-partitioned / --disable-session-cookie-partitioned\n\
  --enable-session-cookies / --disable-session-cookies\n\
  --enable-session-only-cookies / --disable-session-only-cookies\n\
  --session-lock-timeout-ms <n> maximum wait for a session file lock (default: 5000)\n\
  --disable-sessions           disable persistent web sessions\n\
  --max-in-flight <n>          maximum concurrent in-flight requests\n\
  --cpu-execution-limit <n>    maximum concurrent CPU-bound PHP executions (default: available CPUs)\n\
  --request-admission-timeout-ms <n> request admission wait (default: 500)\n\
  --cpu-queue-timeout-ms <n>   PHP CPU queue wait (default: 30000)\n\
  --request-header-timeout-ms <n> header read deadline (default: 10000)\n\
  --request-body-timeout-ms <n> total body read deadline (default: 30000)\n\
  --request-body-idle-timeout-ms <n> idle gap between body frames (default: 15000)\n\
  --response-write-idle-timeout-ms <n> stalled response write deadline (default: 30000)\n\
  --connection-idle-timeout-ms <n> idle keep-alive deadline (default: 75000)\n\
  --tls-handshake-timeout-ms <n> TLS/QUIC handshake deadline (default: 10000)\n\
  --graceful-shutdown-timeout-ms <n> drain deadline (default: 30000)\n\
  --max-request-header-bytes <n> decoded header-section limit (default: 65536)\n\
  --max-request-target-bytes <n> request-target limit (default: 16384)\n\
  --max-streams-per-connection <n> H2/H3 concurrent stream limit (default: 100)\n\
  --index <csv>                ordered directory indexes (default: index.php,index.html)\n\
  --php-extensions <csv>       executable PHP suffixes without dots (default: php)\n\
  --deployment-mode <mode>     dev uses live capability opens; immutable uses a startup asset index\n\
  --max-execution-ms <n>       PHP execution deadline in milliseconds (default: 30000)\n\
  --disable-execution-deadline disable cooperative PHP execution deadline\n\
  --engine-preset <name>       default optimizing or baseline diagnostic native runtime\n\
  --native-cache <mode>        off, read, write, or read-write PNA1 cache access\n\
  --native-cache-dir <path>    directory containing validated PNA1 artifacts\n\
  --perf-ablation <list>       comma-separated disables: inline-caches, include-o2, or all\n\
  --disable-metrics-endpoint   disable GET /__phrust/metrics\n\
  --metrics-token <token>      require Authorization: Bearer token for metrics\n\
  --tls-cert <path>            PEM certificate chain for HTTPS\n\
  --tls-key <path>             PEM private key for HTTPS\n\
  --enable-http3               enable HTTP/3 over QUIC using the TLS certificate\n\
  --http3-listen <addr>        UDP listen address for HTTP/3 (default: TCP listen address)\n\
  --access-log <path|->        write compact access logs to file or stdout\n\
  --perf-trace <path>          append per-PHP-request performance trace JSONL\n\
  --perf-trace-vm-counters     include heavy VM counters in perf trace rows\n\
  --request-profile <dir>      write JSON request profiles for opted-in PHP requests\n\
  --request-profile-vm-counters  collect heavy VM counters for profiled requests\n\
  --request-profile-trigger-header  profile only requests sending x-phrust-request-profile: 1 (default)\n\
  --enable-network-requests    allow PHP cURL requests to external hosts\n\
  --debug                      emit structured server debug events to stderr\n\
  --error-format <text|json>   render server diagnostics/debug events as text or JSON\n\
  --debug-log <path>           append server debug events to a file instead of stderr\n\
  --no-script-cache            disable process-local compiled script cache\n\
  --script-cache-shards <n>    compiled script cache shard count (default: 16)\n\
  --script-cache-max-entries <n> maximum compiled script cache entries (default: 4096)\n\
  --script-cache-preload <file> preload newline-delimited script paths at startup\n\
  --script-cache-check-interval-ms <n> skip stat checks for this many milliseconds (default: 2000; 0 validates every request)\n\
  --strict-preload             fail startup when preload entries cannot compile\n\
  --enable-cache-clear-endpoint enable loopback-only POST /__phrust/cache/clear\n\
  --help                       show this help\n"
    }

    pub fn validated_docroot(&self) -> Result<PathBuf, ConfigError> {
        let docroot = self.routing.docroot.canonicalize().map_err(|error| {
            ConfigError::new(format!(
                "docroot `{}` cannot be canonicalized: {error}",
                self.routing.docroot.display()
            ))
        })?;
        if !docroot.is_dir() {
            return Err(ConfigError::new(format!(
                "docroot `{}` is not a directory",
                docroot.display()
            )));
        }
        Ok(docroot)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.transport.max_connections > tokio::sync::Semaphore::MAX_PERMITS {
            return Err(ConfigError::new(format!(
                "max_connections must not exceed {}",
                tokio::sync::Semaphore::MAX_PERMITS
            )));
        }
        if self.transport.max_request_header_bytes > DEFAULT_MAX_REQUEST_HEADER_BYTES {
            return Err(ConfigError::new(format!(
                "max_request_header_bytes must not exceed the HTTP/1 parser buffer of {DEFAULT_MAX_REQUEST_HEADER_BYTES} bytes"
            )));
        }
        if self.transport.max_streams_per_connection > u32::MAX as usize {
            return Err(ConfigError::new(
                "max_streams_per_connection must fit in an HTTP/2 u32 field",
            ));
        }
        if self.limits.request_body_memory_bytes > self.limits.max_body_bytes {
            return Err(ConfigError::new(
                "request_body_memory_bytes must not exceed max_body_bytes",
            ));
        }
        if self.limits.post_max_bytes > self.limits.max_body_bytes {
            return Err(ConfigError::new(
                "post_max_bytes must not exceed max_body_bytes",
            ));
        }
        if self.routing.indexes.is_empty() {
            return Err(ConfigError::new("at least one directory index is required"));
        }
        if self.routing.php_extensions.is_empty() {
            return Err(ConfigError::new("at least one PHP extension is required"));
        }
        if let Some(path) = &self.routing.front_controller {
            validate_relative_path("front_controller", path)?;
        }
        validate_cookie_name(
            "session_cookie_name",
            &self.sessions_uploads.session_cookie_name,
        )?;
        validate_cookie_path(
            "session_cookie_path",
            &self.sessions_uploads.session_cookie_path,
        )?;
        if !matches!(
            self.sessions_uploads.session_serialize_handler.as_str(),
            "php" | "php_binary" | "php_serialize"
        ) {
            return Err(ConfigError::new(
                "session_serialize_handler must be php, php_binary, or php_serialize",
            ));
        }
        validate_cookie_attribute(
            "session_cookie_domain",
            &self.sessions_uploads.session_cookie_domain,
        )?;
        validate_cookie_attribute(
            "session_cookie_samesite",
            &self.sessions_uploads.session_cookie_samesite,
        )?;
        if self
            .sessions_uploads
            .session_save_path
            .to_string_lossy()
            .contains(';')
        {
            return Err(ConfigError::new(
                "session_save_path must be a plain directory; PHP depth/mode prefixes are not supported",
            ));
        }
        if self.transport.tls_cert.is_some() != self.transport.tls_key.is_some() {
            return Err(ConfigError::new(
                "TLS configuration requires both --tls-cert <path> and --tls-key <path>; provide both flags or neither",
            ));
        }
        if self.transport.http3_enabled && self.transport.tls_cert.is_none() {
            return Err(ConfigError::new(
                "HTTP/3 requires TLS; provide --tls-cert <path> and --tls-key <path> with --enable-http3",
            ));
        }
        Ok(())
    }
}

fn required_value(
    flag: &str,
    args: &mut impl Iterator<Item = String>,
) -> Result<String, ConfigError> {
    args.next().ok_or_else(|| {
        ConfigError::new(format!(
            "{flag} requires a value placeholder, for example {flag} <value>"
        ))
    })
}

fn config_path_from_args(args: &[String]) -> Result<Option<PathBuf>, ConfigError> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--config" {
            let Some(path) = iter.next() else {
                return Err(ConfigError::new(
                    "--config requires a value placeholder, for example --config <path>",
                ));
            };
            return Ok(Some(PathBuf::from(path)));
        }
    }
    Ok(None)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct FileConfig {
    values: HashMap<String, String>,
}

impl FileConfig {
    fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|error| {
            ConfigError::new(format!(
                "config `{}` cannot be read: {error}",
                path.display()
            ))
        })?;
        let mut values = HashMap::new();
        for (line_index, line) in contents.lines().enumerate() {
            let line = strip_config_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(ConfigError::new(format!(
                    "config `{}` line {} must use key = value",
                    path.display(),
                    line_index + 1
                )));
            };
            let key = normalize_config_key(key.trim());
            let value = parse_config_value(value.trim()).map_err(|message| {
                ConfigError::new(format!(
                    "config `{}` line {} {message}",
                    path.display(),
                    line_index + 1
                ))
            })?;
            values.insert(key, value);
        }
        Ok(Self { values })
    }

    fn string(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        self.string(key).map(PathBuf::from)
    }

    fn parse_listen(&self, key: &str) -> Result<Option<SocketAddr>, ConfigError> {
        self.values
            .get(key)
            .map(|value| parse_listen(value))
            .transpose()
    }

    fn positive_usize(&self, key: &str) -> Result<Option<usize>, ConfigError> {
        self.values
            .get(key)
            .map(|value| parse_positive_usize(key, value))
            .transpose()
    }

    fn positive_u64(&self, key: &str) -> Result<Option<u64>, ConfigError> {
        self.values
            .get(key)
            .map(|value| parse_positive_u64(key, value))
            .transpose()
    }

    fn nonnegative_u64(&self, key: &str) -> Result<Option<u64>, ConfigError> {
        self.values
            .get(key)
            .map(|value| parse_nonnegative_u64(key, value))
            .transpose()
    }

    fn bool(&self, key: &str) -> Result<Option<bool>, ConfigError> {
        self.values
            .get(key)
            .map(|value| match value.as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(ConfigError::new(format!(
                    "{key} must be true or false in config"
                ))),
            })
            .transpose()
    }

    fn request_rewrites(&self, key: &str) -> Result<Vec<RequestRewriteRule>, ConfigError> {
        let Some(value) = self.values.get(key) else {
            return Ok(Vec::new());
        };
        parse_request_rewrite_rules(key, value)
    }
}

fn normalize_config_key(key: &str) -> String {
    key.replace('-', "_")
}

fn strip_config_comment(line: &str) -> &str {
    let mut in_quote = false;
    for (index, byte) in line.bytes().enumerate() {
        match byte {
            b'"' => in_quote = !in_quote,
            b'#' if !in_quote => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_config_value(value: &str) -> Result<String, &'static str> {
    if let Some(value) = value.strip_prefix('"') {
        let Some(value) = value.strip_suffix('"') else {
            return Err("has an unterminated quoted value");
        };
        return Ok(value.replace("\\\"", "\"").replace("\\\\", "\\"));
    }
    if value.is_empty() {
        return Err("has an empty value");
    }
    Ok(value.to_string())
}

fn parse_listen(value: &str) -> Result<SocketAddr, ConfigError> {
    value.parse().map_err(|error| {
        ConfigError::new(format!(
            "invalid --listen `{value}`: {error}; expected host:port such as 127.0.0.1:8080"
        ))
    })
}

fn env_bool(env_value: &impl Fn(&str) -> Option<String>, name: &str) -> bool {
    env_value(name)
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
}

fn env_perf_trace_path(env_value: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let value = env_value("PHRUST_PERF_TRACE")?;
    let value = value.trim();
    if value.is_empty() || matches!(value, "0" | "false" | "FALSE" | "off") {
        None
    } else if matches!(value, "1" | "true" | "TRUE" | "yes" | "on") {
        Some(PathBuf::from("target/performance/server/perf-trace.jsonl"))
    } else {
        Some(PathBuf::from(value))
    }
}

fn env_request_profile_path(env_value: &impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let value = env_value("PHRUST_REQUEST_PROFILE")?;
    let value = value.trim();
    if value.is_empty() || matches!(value, "0" | "false" | "FALSE" | "off") {
        None
    } else if matches!(value, "1" | "true" | "TRUE" | "yes" | "on") {
        Some(PathBuf::from("target/performance/server/request-profile"))
    } else {
        Some(PathBuf::from(value))
    }
}

fn env_output_format(
    env_value: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Result<DiagnosticOutputFormat, ConfigError> {
    env_value(name)
        .map(|value| parse_output_format(&value))
        .transpose()
        .map(|value| value.unwrap_or(DiagnosticOutputFormat::Text))
}

fn parse_output_format(value: &str) -> Result<DiagnosticOutputFormat, ConfigError> {
    match value {
        "text" => Ok(DiagnosticOutputFormat::Text),
        "json" | "jsonl" => Ok(DiagnosticOutputFormat::Json),
        _ => Err(ConfigError::new(format!(
            "invalid error format `{value}`; expected text or json"
        ))),
    }
}

fn parse_engine_preset(flag: &str, value: &str) -> Result<EngineProfileName, ConfigError> {
    EngineProfileName::parse(value)
        .map_err(|error| ConfigError::new(format!("invalid {flag}: {error}")))
}

fn parse_native_cache(flag: &str, value: &str) -> Result<NativeCacheMode, ConfigError> {
    value
        .parse()
        .map_err(|error: String| ConfigError::new(format!("invalid {flag}: {error}")))
}

fn env_value_perf_ablation() -> Result<Option<ServerPerfAblation>, ConfigError> {
    std::env::var("PHRUST_PERF_ABLATION")
        .ok()
        .map(|value| parse_perf_ablation("PHRUST_PERF_ABLATION", &value))
        .transpose()
}

fn parse_deployment_mode(flag: &str, value: &str) -> Result<DeploymentRootMode, ConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dev" | "mutable" => Ok(DeploymentRootMode::DevMutable),
        "immutable" => Ok(DeploymentRootMode::ImmutableDeclared),
        _ => Err(ConfigError::new(format!(
            "invalid {flag} `{value}`; expected dev or immutable"
        ))),
    }
}

fn parse_perf_ablation(flag: &str, value: &str) -> Result<ServerPerfAblation, ConfigError> {
    let mut ablation = ServerPerfAblation::default();
    for raw_part in value.split(',') {
        let part = raw_part.trim();
        if part.is_empty() || matches!(part, "none" | "off" | "0") {
            continue;
        }
        match part.replace('_', "-").as_str() {
            "all" => {
                ablation.disable_inline_caches = true;
                ablation.disable_include_o2 = true;
            }
            "inline-caches" => ablation.disable_inline_caches = true,
            "include-o2" => ablation.disable_include_o2 = true,
            _ => {
                return Err(ConfigError::new(format!(
                    "invalid {flag} entry `{part}`; expected inline-caches, include-o2, all, or none"
                )));
            }
        }
    }
    Ok(ablation)
}

fn parse_positive_usize(flag: &str, value: &str) -> Result<usize, ConfigError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|error| ConfigError::new(format!("invalid {flag} `{value}`: {error}")))?;
    if parsed == 0 {
        return Err(ConfigError::new(format!(
            "{flag} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_max_multipart_parts(flag: &str, value: &str) -> Result<Option<usize>, ConfigError> {
    if value == "-1" {
        return Ok(None);
    }
    parse_positive_usize(flag, value).map(Some)
}

fn parse_positive_u64(flag: &str, value: &str) -> Result<u64, ConfigError> {
    let parsed = value
        .parse::<u64>()
        .map_err(|error| ConfigError::new(format!("invalid {flag} `{value}`: {error}")))?;
    if parsed == 0 {
        return Err(ConfigError::new(format!(
            "{flag} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_nonnegative_u64(flag: &str, value: &str) -> Result<u64, ConfigError> {
    value
        .parse::<u64>()
        .map_err(|error| ConfigError::new(format!("invalid {flag} `{value}`: {error}")))
}

fn parse_request_rewrite_rules(
    flag: &str,
    value: &str,
) -> Result<Vec<RequestRewriteRule>, ConfigError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|rule| !rule.is_empty())
        .map(|rule| parse_request_rewrite_rule(flag, rule))
        .collect()
}

fn parse_request_rewrite_rule(flag: &str, value: &str) -> Result<RequestRewriteRule, ConfigError> {
    let Some((path_prefix, query_parameter)) = value.split_once('=') else {
        return Err(ConfigError::new(format!(
            "{flag} must use /path-prefix=query_parameter"
        )));
    };
    let path_prefix = path_prefix.trim();
    let query_parameter = query_parameter.trim();
    validate_rewrite_path_prefix(flag, path_prefix)?;
    validate_query_parameter_name(flag, query_parameter)?;
    Ok(RequestRewriteRule {
        path_prefix: path_prefix.to_string(),
        query_parameter: query_parameter.to_string(),
    })
}

fn validate_rewrite_path_prefix(flag: &str, path_prefix: &str) -> Result<(), ConfigError> {
    if path_prefix.is_empty()
        || !path_prefix.starts_with('/')
        || path_prefix.contains('?')
        || path_prefix.contains('#')
        || path_prefix.contains('\0')
        || (path_prefix != "/" && path_prefix.ends_with('/'))
    {
        return Err(ConfigError::new(format!(
            "{flag} path prefix must start with /, must not end with /, and must not contain ?, #, or NUL"
        )));
    }
    Ok(())
}

fn validate_query_parameter_name(flag: &str, name: &str) -> Result<(), ConfigError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(ConfigError::new(format!(
            "{flag} query parameter must contain only ASCII letters, digits, and _"
        )));
    }
    Ok(())
}

fn parse_indexes(flag: &str, value: &str) -> Result<Vec<String>, ConfigError> {
    let indexes = value
        .split(',')
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if indexes.is_empty()
        || indexes.iter().any(|index| {
            index.is_empty()
                || matches!(index.as_str(), "." | "..")
                || index.contains('/')
                || index.contains('\\')
                || index.contains('\0')
        })
    {
        return Err(ConfigError::new(format!(
            "{flag} must be a comma-separated list of file names without paths or dot segments"
        )));
    }
    Ok(indexes)
}

fn parse_php_extensions(flag: &str, value: &str) -> Result<Vec<String>, ConfigError> {
    let mut extensions = Vec::new();
    for extension in value.split(',').map(str::trim) {
        if extension.is_empty()
            || extension.starts_with('.')
            || !extension
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(ConfigError::new(format!(
                "{flag} must contain comma-separated extensions without leading dots using only ASCII letters, digits, _ or -"
            )));
        }
        let normalized = extension.to_ascii_lowercase();
        if !extensions.contains(&normalized) {
            extensions.push(normalized);
        }
    }
    if extensions.is_empty() {
        return Err(ConfigError::new(format!("{flag} must not be empty")));
    }
    Ok(extensions)
}

fn validate_cookie_name(flag: &str, name: &str) -> Result<(), ConfigError> {
    if name.is_empty()
        || !name.bytes().all(
            |byte| matches!(byte, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e),
        )
    {
        return Err(ConfigError::new(format!(
            "{flag} must be a valid cookie name"
        )));
    }
    Ok(())
}

fn validate_cookie_path(flag: &str, path: &str) -> Result<(), ConfigError> {
    if path.is_empty() || path.contains(['\r', '\n', ';']) {
        return Err(ConfigError::new(format!(
            "{flag} must be a non-empty cookie path without response separators"
        )));
    }
    Ok(())
}

fn validate_cookie_attribute(flag: &str, value: &str) -> Result<(), ConfigError> {
    if value.contains(['\r', '\n', ';']) {
        return Err(ConfigError::new(format!(
            "{flag} must not contain cookie response separators"
        )));
    }
    Ok(())
}

fn validate_relative_path(flag: &str, path: &Path) -> Result<(), ConfigError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(ConfigError::new(format!(
            "{flag} must be a non-empty relative path inside docroot, not an absolute path"
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ConfigError::new(format!(
            "{flag} must stay inside docroot and must not contain `..`"
        )));
    }
    Ok(())
}

fn default_max_in_flight() -> usize {
    DEFAULT_MAX_IN_FLIGHT
}

fn default_cpu_execution_limit() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from)
}

#[cfg(test)]
mod tests {
    use super::ServerConfig;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn static_routing_defaults_and_csv_options_are_validated() {
        let defaults = ServerConfig::parse_from(["--docroot", "."]).expect("default config");
        assert_eq!(defaults.routing.indexes, ["index.php", "index.html"]);
        assert_eq!(defaults.routing.php_extensions, ["php"]);

        let configured = ServerConfig::parse_from([
            "--docroot",
            ".",
            "--index",
            "index.html,index.php",
            "--php-extensions",
            "PHP,phtml,php",
        ])
        .expect("configured static routing");
        assert_eq!(configured.routing.indexes, ["index.html", "index.php"]);
        assert_eq!(configured.routing.php_extensions, ["php", "phtml"]);
    }

    #[test]
    fn invalid_static_routing_values_fail_config_parsing() {
        for (flag, value) in [
            ("--index", ""),
            ("--index", "../index.html"),
            ("--index", "dir/index.html"),
            ("--index", "index.html,"),
            ("--php-extensions", ""),
            ("--php-extensions", ".php"),
            ("--php-extensions", "php/phtml"),
            ("--php-extensions", "php,p html"),
        ] {
            assert!(
                ServerConfig::parse_from(["--docroot", ".", flag, value]).is_err(),
                "{flag}={value:?}"
            );
        }
    }

    #[test]
    fn config_file_accepts_static_index_and_php_extension_csv() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "phrust-server-static-config-{}-{unique}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "docroot = \".\"\nindex = \"index.html,index.php\"\nphp_extensions = \"php,phtml\"\n",
        )
        .expect("write test config");
        let path_arg = path.to_string_lossy().into_owned();
        let config = ServerConfig::parse_from(["--config", path_arg.as_str()])
            .expect("parse static config file");
        std::fs::remove_file(path).expect("remove test config");

        assert_eq!(config.routing.indexes, ["index.html", "index.php"]);
        assert_eq!(config.routing.php_extensions, ["php", "phtml"]);
    }

    #[test]
    fn help_describes_static_index_extensions_and_deployment_mode_once() {
        let help = ServerConfig::help_text();
        assert_eq!(help.matches("--index <csv>").count(), 1);
        assert_eq!(help.matches("--php-extensions <csv>").count(), 1);
        assert_eq!(help.matches("--deployment-mode <mode>").count(), 1);
        assert!(help.contains("index.php,index.html"));
        assert!(help.contains("startup asset index"));
    }

    #[test]
    fn transport_hardening_defaults_and_timeout_migration_are_explicit() {
        let config = ServerConfig::parse_from(["--docroot", "."]).expect("default config");
        assert_eq!(config.transport.max_connections, 1_024);
        assert_eq!(config.limits.request_admission_timeout_ms, 500);
        assert_eq!(config.limits.cpu_queue_timeout_ms, 30_000);
        assert_eq!(config.transport.request_header_timeout_ms, 10_000);
        assert_eq!(config.limits.request_body_timeout_ms, 30_000);
        assert_eq!(config.limits.request_body_idle_timeout_ms, 15_000);
        assert_eq!(config.transport.response_write_idle_timeout_ms, 30_000);
        assert_eq!(config.transport.connection_idle_timeout_ms, 75_000);
        assert_eq!(config.transport.tls_handshake_timeout_ms, 10_000);
        assert_eq!(config.transport.graceful_shutdown_timeout_ms, 30_000);
        assert_eq!(config.transport.max_request_header_bytes, 65_536);
        assert_eq!(config.transport.max_request_target_bytes, 16_384);
        assert_eq!(config.transport.max_streams_per_connection, 100);

        let error = ServerConfig::parse_from(["--docroot", ".", "--request-timeout-ms", "1000"])
            .expect_err("retired timeout must fail");
        assert!(error.to_string().contains("--request-body-timeout-ms"));
        assert!(!ServerConfig::help_text().contains("--request-timeout-ms"));
    }

    #[test]
    fn retired_file_timeout_has_a_migration_error() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "phrust-server-retired-timeout-{}-{unique}.toml",
            std::process::id()
        ));
        std::fs::write(&path, "docroot = \".\"\nrequest_timeout_ms = 1\n")
            .expect("write test config");
        let path_arg = path.to_string_lossy().into_owned();
        let error = ServerConfig::parse_from(["--config", path_arg.as_str()])
            .expect_err("retired file setting must fail");
        std::fs::remove_file(path).expect("remove test config");
        assert!(error.to_string().contains("cpu_queue_timeout_ms"));
    }
}
