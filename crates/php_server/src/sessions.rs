use super::state::AppState;
use crate::session_store::{
    SessionFileLease, SessionStoreError, generate_session_id_with_policy, valid_session_id,
};
use php_executor::PhpExecutionOutput;
use php_runtime::api::{
    PHP_SESSION_ACTIVE, RuntimeCancellationState, RuntimeHttpRequestContext, SessionAbortCallback,
    SessionDestroyCallback, SessionGcCallback, SessionIdGenerateCallback, SessionLoadCallback,
    SessionLoadResult, SessionRegenerateCallback, SessionState, SessionWriteCallback,
    encode_runtime_session_payload,
};
use std::{
    sync::{Arc, Mutex, atomic::Ordering},
    time::{Duration, Instant},
};

pub(crate) fn seed_session_state(
    request: &RuntimeHttpRequestContext,
    state: &AppState,
) -> Result<SessionState, String> {
    if !state.sessions.config.enabled {
        return Ok(SessionState::default());
    }
    state
        .services
        .metrics
        .session_seed_attempts
        .fetch_add(1, Ordering::Relaxed);
    let incoming_id = request
        .parsed_cookie
        .iter()
        .rev()
        .find(|(name, _)| name == &state.sessions.config.cookie_name)
        .map(|(_, value)| value.as_str())
        .filter(|value| valid_session_id(value))
        .unwrap_or("");
    Ok(SessionState::seeded_lazy(
        state.sessions.config.cookie_name.clone(),
        incoming_id.to_string(),
        None,
    ))
}

#[derive(Clone)]
pub(crate) struct SessionRequestCallbacks {
    pub(crate) loader: SessionLoadCallback,
    pub(crate) writer: SessionWriteCallback,
    pub(crate) aborter: SessionAbortCallback,
    pub(crate) destroyer: SessionDestroyCallback,
    pub(crate) regenerator: SessionRegenerateCallback,
    pub(crate) gc: SessionGcCallback,
    pub(crate) id_generator: SessionIdGenerateCallback,
    lease: Arc<Mutex<Option<SessionFileLease>>>,
    store: Arc<crate::session_store::SessionStore>,
    metrics: Arc<crate::metrics::ServerMetrics>,
    cancellation: RuntimeCancellationState,
}

impl SessionRequestCallbacks {
    pub(crate) fn new(state: &AppState, cancellation: RuntimeCancellationState) -> Self {
        let metrics = Arc::clone(&state.services.metrics);
        let store = Arc::clone(&state.sessions.session_store);
        let lease = Arc::new(Mutex::new(None::<SessionFileLease>));
        let load_metrics = Arc::clone(&metrics);
        let load_store = Arc::clone(&store);
        let load_lease = Arc::clone(&lease);
        let load_cancellation = cancellation.clone();
        let loader = SessionLoadCallback::new_with_policy(move |id, strict_mode| {
            load_metrics
                .session_lazy_loads
                .fetch_add(1, Ordering::Relaxed);
            load_metrics
                .session_store_loads
                .fetch_add(1, Ordering::Relaxed);
            let started = Instant::now();
            let mut slot = load_lease
                .lock()
                .map_err(|_| store_error("lease lock poisoned"))?;
            if slot
                .as_ref()
                .is_none_or(|current| current.id() != id || current.finalized())
            {
                if let Some(mut previous) = slot.take() {
                    let _ = load_store.abort(&mut previous);
                }
                load_metrics
                    .session_lock_waits_total
                    .fetch_add(1, Ordering::Relaxed);
                let acquired = if strict_mode {
                    load_store.acquire_existing_cancellable(id, &load_cancellation)
                } else {
                    load_store
                        .acquire_cancellable(id, &load_cancellation)
                        .map(Some)
                }
                .map_err(|error| {
                    record_lock_error(&load_metrics, &error);
                    store_error(&format!("failed to load session: {error}"))
                })?;
                let Some(acquired) = acquired else {
                    return Ok(SessionLoadResult::default());
                };
                load_metrics
                    .session_lock_wait_nanos_total
                    .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
                *slot = Some(acquired);
            }
            let current = slot.as_ref().expect("lease installed");
            load_metrics
                .session_file_reads_total
                .fetch_add(1, Ordering::Relaxed);
            Ok(SessionLoadResult {
                payload: current.payload().to_vec(),
                existed: current.existed(),
            })
        });

        let write_metrics = Arc::clone(&metrics);
        let write_store = Arc::clone(&store);
        let write_lease = Arc::clone(&lease);
        let write_cancellation = cancellation.clone();
        let writer = SessionWriteCallback::new_with_policy(move |id, payload, lazy_write| {
            let mut slot = write_lease
                .lock()
                .map_err(|_| store_error("lease lock poisoned"))?;
            ensure_lease(
                &write_store,
                &write_metrics,
                &mut slot,
                id,
                &write_cancellation,
            )?;
            let wrote = write_store
                .commit(slot.as_mut().expect("lease installed"), payload, lazy_write)
                .map_err(|error| store_error(&format!("failed to save session: {error}")))?;
            if wrote {
                write_metrics
                    .session_store_writes
                    .fetch_add(1, Ordering::Relaxed);
                write_metrics
                    .session_file_writes_total
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                write_metrics
                    .session_lazy_touches_total
                    .fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        });

        let abort_metrics = Arc::clone(&metrics);
        let abort_store = Arc::clone(&store);
        let abort_lease = Arc::clone(&lease);
        let aborter = SessionAbortCallback::new(move |id| {
            let mut slot = abort_lease
                .lock()
                .map_err(|_| store_error("lease lock poisoned"))?;
            if let Some(current) = slot.as_mut().filter(|current| current.id() == id) {
                abort_store
                    .abort(current)
                    .map_err(|error| store_error(&format!("failed to unlock session: {error}")))?;
            }
            abort_metrics
                .session_aborts_total
                .fetch_add(1, Ordering::Relaxed);
            Ok(())
        });

        let destroy_metrics = Arc::clone(&metrics);
        let destroy_store = Arc::clone(&store);
        let destroy_lease = Arc::clone(&lease);
        let destroy_cancellation = cancellation.clone();
        let destroyer = SessionDestroyCallback::new(move |id| {
            let mut slot = destroy_lease
                .lock()
                .map_err(|_| store_error("lease lock poisoned"))?;
            ensure_lease(
                &destroy_store,
                &destroy_metrics,
                &mut slot,
                id,
                &destroy_cancellation,
            )?;
            destroy_store
                .destroy(slot.as_mut().expect("lease installed"))
                .map_err(|error| store_error(&format!("failed to delete session: {error}")))?;
            destroy_metrics
                .session_store_deletes
                .fetch_add(1, Ordering::Relaxed);
            Ok(())
        });

        let regenerate_metrics = Arc::clone(&metrics);
        let regenerate_store = Arc::clone(&store);
        let regenerate_lease = Arc::clone(&lease);
        let regenerate_cancellation = cancellation.clone();
        let regenerator = SessionRegenerateCallback::new_with_policy(
            move |old_id, new_id, payload, delete_old| {
                let mut slot = regenerate_lease
                    .lock()
                    .map_err(|_| store_error("lease lock poisoned"))?;
                ensure_lease(
                    &regenerate_store,
                    &regenerate_metrics,
                    &mut slot,
                    old_id,
                    &regenerate_cancellation,
                )?;
                let new_lease = regenerate_store
                    .regenerate(
                        slot.as_mut().expect("lease installed"),
                        new_id,
                        payload,
                        delete_old,
                    )
                    .map_err(|error| {
                        store_error(&format!("failed to regenerate session: {error}"))
                    })?;
                *slot = Some(new_lease);
                regenerate_metrics
                    .session_regenerations_total
                    .fetch_add(1, Ordering::Relaxed);
                regenerate_metrics
                    .session_file_writes_total
                    .fetch_add(1, Ordering::Relaxed);
                Ok(())
            },
        );

        let gc_metrics = Arc::clone(&metrics);
        let gc_store = Arc::clone(&store);
        let gc = SessionGcCallback::new(move |max_lifetime_seconds| {
            gc_metrics
                .session_gc_runs_total
                .fetch_add(1, Ordering::Relaxed);
            let deleted = gc_store
                .gc(Duration::from_secs(max_lifetime_seconds))
                .map_err(|error| store_error(&format!("session GC failed: {error}")))?;
            gc_metrics
                .session_gc_deleted_total
                .fetch_add(deleted as u64, Ordering::Relaxed);
            Ok(deleted)
        });

        let id_metrics = Arc::clone(&metrics);
        let id_store = Arc::clone(&store);
        let id_generator =
            SessionIdGenerateCallback::new_with_policy(move |length, bits, prefix| {
                id_metrics
                    .session_id_generations
                    .fetch_add(1, Ordering::Relaxed);
                for _ in 0..32 {
                    let id =
                        generate_session_id_with_policy(length, bits, prefix).map_err(|error| {
                            store_error(&format!("failed to generate session id: {error}"))
                        })?;
                    if !id_store.exists(&id).map_err(|error| {
                        store_error(&format!("failed to check session id: {error}"))
                    })? {
                        return Ok(id);
                    }
                }
                Err(store_error(
                    "could not generate a collision-free session id",
                ))
            });

        Self {
            loader,
            writer,
            aborter,
            destroyer,
            regenerator,
            gc,
            id_generator,
            lease,
            store,
            metrics,
            cancellation,
        }
    }

    pub(crate) fn finalize(&self, output: &mut PhpExecutionOutput) -> Result<(), String> {
        self.metrics
            .session_finalizations
            .fetch_add(1, Ordering::Relaxed);
        if output.session.status() != PHP_SESSION_ACTIVE || output.session.id().is_empty() {
            self.metrics
                .session_finalize_skipped_inactive
                .fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        let id = output.session.id().to_owned();
        let data = output.session.data();
        let payload = encode_runtime_session_payload(output.session.serialize_handler(), &data, -1)
            .map_err(|error| store_error(&format!("failed to encode session: {error}")))?;
        let mut slot = self
            .lease
            .lock()
            .map_err(|_| store_error("lease lock poisoned"))?;
        ensure_lease(
            &self.store,
            &self.metrics,
            &mut slot,
            &id,
            &self.cancellation,
        )?;
        let wrote = self
            .store
            .commit(
                slot.as_mut().expect("lease installed"),
                &payload,
                output.session.lazy_write(),
            )
            .map_err(|error| store_error(&format!("failed to save session: {error}")))?;
        if wrote {
            self.metrics
                .session_store_writes
                .fetch_add(1, Ordering::Relaxed);
            self.metrics
                .session_file_writes_total
                .fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics
                .session_lazy_touches_total
                .fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }
}

fn ensure_lease(
    store: &crate::session_store::SessionStore,
    metrics: &crate::metrics::ServerMetrics,
    slot: &mut Option<SessionFileLease>,
    id: &str,
    cancellation: &RuntimeCancellationState,
) -> Result<(), String> {
    if slot
        .as_ref()
        .is_some_and(|current| current.id() == id && !current.finalized())
    {
        return Ok(());
    }
    if let Some(mut previous) = slot.take() {
        let _ = store.abort(&mut previous);
    }
    metrics
        .session_lock_waits_total
        .fetch_add(1, Ordering::Relaxed);
    let started = Instant::now();
    let acquired = store
        .acquire_cancellable(id, cancellation)
        .map_err(|error| {
            record_lock_error(metrics, &error);
            store_error(&format!("failed to open session: {error}"))
        })?;
    metrics
        .session_lock_wait_nanos_total
        .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    *slot = Some(acquired);
    Ok(())
}

fn record_lock_error(metrics: &crate::metrics::ServerMetrics, error: &SessionStoreError) {
    if matches!(error, SessionStoreError::LockTimeout) {
        metrics
            .session_lock_timeouts_total
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn store_error(message: &str) -> String {
    format!("E_PHP_SESSION_STORE_UNAVAILABLE: {message}")
}
