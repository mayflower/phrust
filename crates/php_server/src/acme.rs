use crate::{
    config::AcmeTlsConfig,
    metrics::ServerMetrics,
    server::ServerError,
    shutdown::{ShutdownCoordinator, ShutdownPhase},
    tls::tls_alpn_protocols,
};
use futures_util::StreamExt;
use rustls_acme::{
    AcmeConfig, AcmeState, EventError, EventOk, OrderError, ResolvesServerCertAcme, UseChallenge,
    caches::DirCache,
};
use rustls_pki_types::pem::PemObject;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore, ServerConfig, crypto::CryptoProvider, pki_types::CertificateDer,
};
use tracing::{debug, warn};

const ERROR_NONE: u8 = 0;
const ERROR_TRANSIENT_ORDER: u8 = 1;
const ERROR_CERT_PARSE: u8 = 2;
const ERROR_CACHE: u8 = 3;
const ERROR_STREAM_ENDED: u8 = 4;

#[derive(Debug, Default)]
pub(crate) struct AcmeStatus {
    certificate_available: AtomicBool,
    manager_running: AtomicBool,
    degraded: AtomicBool,
    successful_events_total: AtomicU64,
    error_events_total: AtomicU64,
    last_success_unix: AtomicU64,
    last_error_class: AtomicU8,
}

impl AcmeStatus {
    pub(crate) fn certificate_available(&self) -> bool {
        self.certificate_available.load(Ordering::Acquire)
    }

    pub(crate) fn manager_running(&self) -> bool {
        self.manager_running.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn degraded(&self) -> bool {
        self.degraded.load(Ordering::Acquire)
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.manager_running() && self.certificate_available()
    }

    #[cfg(test)]
    pub(crate) fn successful_events_total(&self) -> u64 {
        self.successful_events_total.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn error_events_total(&self) -> u64 {
        self.error_events_total.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn last_success_unix(&self) -> u64 {
        self.last_success_unix.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn last_error_class(&self) -> u8 {
        self.last_error_class.load(Ordering::Acquire)
    }
}

pub(crate) struct AcmeTls {
    domains: Vec<String>,
    pub(crate) resolver: Arc<ResolvesServerCertAcme>,
    pub(crate) normal_config: Arc<ServerConfig>,
    pub(crate) challenge_config: Arc<ServerConfig>,
    pub(crate) status: Arc<AcmeStatus>,
}

impl std::fmt::Debug for AcmeTls {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AcmeTls")
            .field("domain_count", &self.domains.len())
            .field(
                "certificate_available",
                &self.status.certificate_available(),
            )
            .field("manager_running", &self.status.manager_running())
            .finish_non_exhaustive()
    }
}

impl AcmeTls {
    pub(crate) fn permits_challenge_sni(&self, server_name: Option<&str>) -> bool {
        server_name.is_some_and(|server_name| {
            self.domains
                .iter()
                .any(|domain| domain.eq_ignore_ascii_case(server_name))
        })
    }
}

pub(crate) struct AcmeManager {
    state: AcmeState<std::io::Error>,
    status: Arc<AcmeStatus>,
    cache_dir: PathBuf,
    initialized_at: Instant,
}

pub(crate) struct PreparedAcme {
    pub(crate) tls: Arc<AcmeTls>,
    pub(crate) manager: AcmeManager,
}

pub(crate) fn prepare_acme(
    config: &AcmeTlsConfig,
    provider: Arc<CryptoProvider>,
) -> Result<PreparedAcme, ServerError> {
    let base = if let Some(ca_path) = config.directory_ca_cert.as_deref() {
        let mut roots = RootCertStore::empty();
        let certs = CertificateDer::pem_file_iter(ca_path)
            .map_err(|error| {
                ServerError::Tls(format!(
                    "custom ACME CA `{}` cannot be parsed: {error}",
                    ca_path.display()
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                ServerError::Tls(format!(
                    "custom ACME CA `{}` cannot be parsed: {error}",
                    ca_path.display()
                ))
            })?;
        if certs.is_empty() {
            return Err(ServerError::Tls(format!(
                "custom ACME CA `{}` contains no certificates",
                ca_path.display()
            )));
        }
        for cert in certs {
            roots.add(cert).map_err(|error| {
                ServerError::Tls(format!(
                    "custom ACME CA `{}` is invalid: {error}",
                    ca_path.display()
                ))
            })?;
        }
        let client = ClientConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()
            .map_err(|error| ServerError::Tls(format!("ACME client TLS setup failed: {error}")))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        AcmeConfig::new_with_client_config(config.domains.iter(), Arc::new(client))
    } else {
        AcmeConfig::new_with_provider(config.domains.iter(), Arc::clone(&provider))
    };
    let state = base
        .contact([config.contact.as_str()])
        .directory(config.directory.url())
        .cache(DirCache::new(config.cache_dir.clone()))
        .challenge_type(UseChallenge::TlsAlpn01)
        .state();
    let resolver = state.resolver();
    let challenge_config = state.challenge_rustls_config_with_provider(Arc::clone(&provider));
    let cert_resolver: Arc<dyn tokio_rustls::rustls::server::ResolvesServerCert> = resolver.clone();
    let mut normal_config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| ServerError::Tls(format!("ACME server TLS setup failed: {error}")))?
        .with_no_client_auth()
        .with_cert_resolver(cert_resolver);
    normal_config.alpn_protocols = tls_alpn_protocols();
    let status = Arc::new(AcmeStatus::default());
    Ok(PreparedAcme {
        tls: Arc::new(AcmeTls {
            domains: config.domains.clone(),
            resolver,
            normal_config: Arc::new(normal_config),
            challenge_config,
            status: Arc::clone(&status),
        }),
        manager: AcmeManager {
            state,
            status,
            cache_dir: config.cache_dir.clone(),
            initialized_at: Instant::now(),
        },
    })
}

impl AcmeManager {
    pub(crate) async fn run(mut self, metrics: Arc<ServerMetrics>, shutdown: ShutdownCoordinator) {
        self.status.manager_running.store(true, Ordering::Release);
        metrics.acme_manager_running.store(1, Ordering::Release);
        metrics
            .acme_state_tasks_started_total
            .fetch_add(1, Ordering::Relaxed);
        metrics.acme_state_tasks_active.store(1, Ordering::Release);
        update_readiness(&self.status, &metrics, &shutdown);
        let mut shutdown_rx = shutdown.subscribe();
        loop {
            tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    let phase = *shutdown_rx.borrow_and_update();
                    if changed.is_err() || phase != ShutdownPhase::Running {
                        if phase == ShutdownPhase::Forced {
                            metrics.acme_shutdown_forced_total.fetch_add(1, Ordering::Relaxed);
                        } else {
                            metrics.acme_shutdown_completions_total.fetch_add(1, Ordering::Relaxed);
                        }
                        break;
                    }
                }
                event = self.state.next() => {
                    let Some(event) = event else {
                        metrics.acme_state_stream_ended_total.fetch_add(1, Ordering::Relaxed);
                        self.terminal(ERROR_STREAM_ENDED, &metrics, &shutdown);
                        break;
                    };
                    match event {
                        Ok(event) => {
                            if let Err(error) = self.success(event, &metrics, &shutdown) {
                                warn!(%error, "ACME cache permissions could not be secured");
                                metrics.acme_cache_errors_total.fetch_add(1, Ordering::Relaxed);
                                self.terminal(ERROR_CACHE, &metrics, &shutdown);
                                break;
                            }
                        }
                        Err(error) => {
                            if self.error(error, &metrics, &shutdown) {
                                break;
                            }
                        }
                    }
                }
            }
        }
        self.status.manager_running.store(false, Ordering::Release);
        metrics.acme_manager_running.store(0, Ordering::Release);
        metrics.acme_state_tasks_active.store(0, Ordering::Release);
        update_readiness(&self.status, &metrics, &shutdown);
    }

    fn success(
        &self,
        event: EventOk,
        metrics: &ServerMetrics,
        shutdown: &ShutdownCoordinator,
    ) -> Result<(), std::io::Error> {
        self.status
            .successful_events_total
            .fetch_add(1, Ordering::Relaxed);
        self.status.last_success_unix.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            Ordering::Release,
        );
        self.status
            .last_error_class
            .store(ERROR_NONE, Ordering::Release);
        metrics
            .acme_events_success_total
            .fetch_add(1, Ordering::Relaxed);
        match event {
            EventOk::DeployedCachedCert => {
                self.certificate_available(metrics);
                metrics
                    .acme_cache_loads_total
                    .fetch_add(1, Ordering::Relaxed);
                debug!("ACME cached certificate deployed");
            }
            EventOk::DeployedNewCert => {
                self.certificate_available(metrics);
                metrics
                    .acme_certificates_issued_total
                    .fetch_add(1, Ordering::Relaxed);
                debug!("ACME certificate deployed");
            }
            EventOk::CertCacheStore | EventOk::AccountCacheStore => {
                secure_cache_files(&self.cache_dir)?;
                debug!("ACME cache entry stored");
            }
        }
        update_readiness(&self.status, metrics, shutdown);
        Ok(())
    }

    fn certificate_available(&self, metrics: &ServerMetrics) {
        if !self
            .status
            .certificate_available
            .swap(true, Ordering::AcqRel)
        {
            let nanos = self
                .initialized_at
                .elapsed()
                .as_nanos()
                .min(u64::MAX as u128) as u64;
            metrics
                .acme_initialization_nanos_total
                .fetch_add(nanos, Ordering::Relaxed);
        }
        self.status.degraded.store(false, Ordering::Release);
        metrics
            .acme_certificate_available
            .store(1, Ordering::Release);
        metrics.acme_degraded.store(0, Ordering::Release);
    }

    fn error(
        &self,
        error: EventError<std::io::Error, std::io::Error>,
        metrics: &ServerMetrics,
        shutdown: &ShutdownCoordinator,
    ) -> bool {
        self.status
            .error_events_total
            .fetch_add(1, Ordering::Relaxed);
        metrics
            .acme_events_error_total
            .fetch_add(1, Ordering::Relaxed);
        match error {
            EventError::CertCacheLoad(error)
            | EventError::AccountCacheLoad(error)
            | EventError::CertCacheStore(error)
            | EventError::AccountCacheStore(error) => {
                metrics
                    .acme_cache_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                warn!(class = "cache_io", kind = ?error.kind(), "terminal ACME cache error");
                self.terminal(ERROR_CACHE, metrics, shutdown);
                true
            }
            EventError::CachedCertParse(_) => {
                metrics
                    .acme_cache_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                warn!(
                    class = "cached_certificate_parse",
                    "terminal ACME cache error"
                );
                self.terminal(ERROR_CACHE, metrics, shutdown);
                true
            }
            EventError::Order(error) => {
                let terminal = matches!(
                    error,
                    OrderError::Acme(rustls_acme::acme::AcmeError::KeyRejected(_))
                );
                if terminal {
                    warn!(class = "account_key", "terminal ACME account-key error");
                    self.terminal(ERROR_CACHE, metrics, shutdown);
                    return true;
                }
                self.transient(ERROR_TRANSIENT_ORDER, metrics, shutdown);
                metrics
                    .acme_renewal_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                warn!(class = "order", "transient ACME order error");
                false
            }
            EventError::NewCertParse(_) => {
                self.transient(ERROR_CERT_PARSE, metrics, shutdown);
                metrics
                    .acme_renewal_errors_total
                    .fetch_add(1, Ordering::Relaxed);
                warn!(
                    class = "new_certificate_parse",
                    "ACME certificate parse error"
                );
                false
            }
        }
    }

    fn transient(&self, class: u8, metrics: &ServerMetrics, shutdown: &ShutdownCoordinator) {
        self.status.degraded.store(true, Ordering::Release);
        self.status.last_error_class.store(class, Ordering::Release);
        metrics.acme_degraded.store(1, Ordering::Release);
        update_readiness(&self.status, metrics, shutdown);
    }

    fn terminal(&self, class: u8, metrics: &ServerMetrics, shutdown: &ShutdownCoordinator) {
        self.status.manager_running.store(false, Ordering::Release);
        self.status.degraded.store(true, Ordering::Release);
        self.status.last_error_class.store(class, Ordering::Release);
        metrics.acme_manager_running.store(0, Ordering::Release);
        metrics.acme_degraded.store(1, Ordering::Release);
        metrics
            .acme_terminal_failures_total
            .fetch_add(1, Ordering::Relaxed);
        metrics.acme_readiness.store(0, Ordering::Release);
        metrics.readiness_state.store(0, Ordering::Release);
        shutdown.begin_draining();
    }
}

fn update_readiness(status: &AcmeStatus, metrics: &ServerMetrics, shutdown: &ShutdownCoordinator) {
    let ready = shutdown.is_running() && status.is_ready();
    metrics
        .acme_readiness
        .store(u64::from(ready), Ordering::Release);
    metrics
        .readiness_state
        .store(u64::from(ready), Ordering::Release);
}

pub(crate) fn prepare_acme_cache_dir(path: &Path) -> Result<PathBuf, ServerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ServerError::Io(std::io::Error::other(format!(
                    "ACME cache `{}` must be a real directory, not a symlink",
                    path.display()
                ))));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path)?;
        }
        Err(error) => return Err(ServerError::Io(error)),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    let probe = tempfile::Builder::new()
        .prefix(".phrust-acme-probe-")
        .tempfile_in(path)?;
    drop(probe);
    secure_cache_files(path)?;
    path.canonicalize().map_err(ServerError::Io)
}

fn secure_cache_files(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(std::io::Error::other(format!(
                "ACME cache entry `{}` must be a regular file",
                entry.path().display()
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AcmeDirectory, AcmeTlsConfig};

    fn test_manager() -> (tempfile::TempDir, AcmeManager, Arc<AcmeStatus>) {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        prepare_acme_cache_dir(&cache).unwrap();
        let prepared = prepare_acme(
            &AcmeTlsConfig {
                domains: vec!["example.org".to_string()],
                contact: "mailto:a@example.org".to_string(),
                cache_dir: cache,
                directory: AcmeDirectory::Staging,
                directory_ca_cert: None,
            },
            Arc::new(tokio_rustls::rustls::crypto::ring::default_provider()),
        )
        .unwrap();
        let status = Arc::clone(&prepared.tls.status);
        (root, prepared.manager, status)
    }

    #[cfg(unix)]
    #[test]
    fn creates_private_cache_directory_and_rejects_symlink() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let canonical = prepare_acme_cache_dir(&cache).unwrap();
        assert_eq!(canonical, cache.canonicalize().unwrap());
        assert_eq!(
            fs::metadata(&cache).unwrap().permissions().mode() & 0o777,
            0o700
        );

        let link = root.path().join("cache-link");
        symlink(&cache, &link).unwrap();
        assert!(prepare_acme_cache_dir(&link).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cache_files_are_forced_to_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        prepare_acme_cache_dir(&cache).unwrap();
        let entry = cache.join("cached_account_test");
        fs::write(&entry, b"sensitive").unwrap();
        fs::set_permissions(&entry, fs::Permissions::from_mode(0o644)).unwrap();
        secure_cache_files(&cache).unwrap();
        assert_eq!(
            fs::metadata(entry).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn empty_status_is_not_ready() {
        let status = AcmeStatus::default();
        assert!(!status.is_ready());
        assert!(!status.certificate_available());
        assert!(!status.manager_running());
        assert!(!status.degraded());
        assert_eq!(status.successful_events_total(), 0);
        assert_eq!(status.error_events_total(), 0);
        assert_eq!(status.last_success_unix(), 0);
        assert_eq!(status.last_error_class(), ERROR_NONE);
    }

    #[test]
    fn normal_and_challenge_alpn_are_disjoint_and_sni_is_allowlisted() {
        let (_root, manager, _status) = test_manager();
        let tls = prepare_acme(
            &AcmeTlsConfig {
                domains: vec!["example.org".to_string()],
                contact: "mailto:a@example.org".to_string(),
                cache_dir: manager.cache_dir.clone(),
                directory: AcmeDirectory::Staging,
                directory_ca_cert: None,
            },
            Arc::new(tokio_rustls::rustls::crypto::ring::default_provider()),
        )
        .unwrap()
        .tls;
        assert_eq!(
            tls.normal_config.alpn_protocols,
            [b"h2".to_vec(), b"http/1.1".to_vec()]
        );
        assert_eq!(
            tls.challenge_config.alpn_protocols,
            [b"acme-tls/1".to_vec()]
        );
        assert!(tls.permits_challenge_sni(Some("example.org")));
        assert!(tls.permits_challenge_sni(Some("EXAMPLE.ORG")));
        assert!(!tls.permits_challenge_sni(Some("unknown.example")));
        assert!(!tls.permits_challenge_sni(None));
    }

    #[test]
    fn certificate_event_sets_readiness_and_transient_error_keeps_it() {
        let (_root, manager, status) = test_manager();
        let metrics = ServerMetrics::default();
        let shutdown = ShutdownCoordinator::new();
        status.manager_running.store(true, Ordering::Release);
        manager
            .success(EventOk::DeployedCachedCert, &metrics, &shutdown)
            .unwrap();
        assert!(status.is_ready());
        assert_eq!(metrics.readiness_state.load(Ordering::Acquire), 1);
        assert_eq!(metrics.acme_cache_loads_total.load(Ordering::Acquire), 1);

        manager.transient(ERROR_TRANSIENT_ORDER, &metrics, &shutdown);
        assert!(status.is_ready());
        assert!(status.degraded());
        assert_eq!(metrics.readiness_state.load(Ordering::Acquire), 1);
    }

    #[test]
    fn transient_error_without_certificate_stays_not_ready() {
        let (_root, manager, status) = test_manager();
        let metrics = ServerMetrics::default();
        let shutdown = ShutdownCoordinator::new();
        status.manager_running.store(true, Ordering::Release);
        manager.transient(ERROR_TRANSIENT_ORDER, &metrics, &shutdown);
        assert!(!status.is_ready());
        assert!(status.degraded());
        assert_eq!(metrics.readiness_state.load(Ordering::Acquire), 0);
    }

    #[test]
    fn terminal_state_end_disables_readiness_and_begins_drain() {
        let (_root, manager, status) = test_manager();
        let metrics = ServerMetrics::default();
        let shutdown = ShutdownCoordinator::new();
        status.manager_running.store(true, Ordering::Release);
        manager.certificate_available(&metrics);
        manager.terminal(ERROR_STREAM_ENDED, &metrics, &shutdown);
        assert!(!status.is_ready());
        assert!(!shutdown.is_running());
        assert_eq!(metrics.readiness_state.load(Ordering::Acquire), 0);
    }

    #[test]
    fn low_level_state_does_not_take_listener_ownership() {
        let source = include_str!("acme.rs");
        let forbidden = [
            [".", "incoming("].concat(),
            [".", "tokio_incoming("].concat(),
            ["Arc<", "Mutex<AcmeState"].concat(),
        ];
        for needle in forbidden {
            assert!(!source.contains(&needle), "forbidden ACME API: {needle}");
        }
        let serve_source = include_str!("serve.rs");
        assert_eq!(
            serve_source
                .matches("manager.run(metrics, shutdown)")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn shutdown_joins_the_single_manager_task() {
        let (_root, manager, status) = test_manager();
        let metrics = Arc::new(ServerMetrics::default());
        let shutdown = ShutdownCoordinator::new();
        let mut tasks = tokio::task::JoinSet::new();
        let task_metrics = Arc::clone(&metrics);
        let task_shutdown = shutdown.clone();
        tasks.spawn(async move { manager.run(task_metrics, task_shutdown).await });
        tokio::task::yield_now().await;
        shutdown.force();
        tokio::time::timeout(std::time::Duration::from_secs(1), tasks.join_next())
            .await
            .expect("manager task must honor shutdown")
            .expect("manager task must remain owned")
            .expect("manager task must not panic");
        assert!(!status.manager_running());
        assert_eq!(metrics.acme_state_tasks_active.load(Ordering::Acquire), 0);
        assert_eq!(
            metrics.acme_shutdown_forced_total.load(Ordering::Acquire),
            1
        );
    }
}
