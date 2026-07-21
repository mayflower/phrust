use crate::{
    access_log::AccessLogEntry, diagnostics::emit_server_debug_lazy,
    request_pipeline::PhpTransferCompletion, serve::write_access_log, state::AppState,
};
use hyper::StatusCode;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, atomic::Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::OwnedSemaphorePermit;
use tracing::warn;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransferOutcome {
    Completed,
    Aborted,
    Error,
}

impl TransferOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Aborted => "aborted",
            Self::Error => "error",
        }
    }
}

pub(crate) struct TransferContext {
    pub(crate) state: Arc<AppState>,
    pub(crate) request_id: String,
    pub(crate) started: Instant,
    pub(crate) method: String,
    pub(crate) request_target: String,
    pub(crate) route: &'static str,
    pub(crate) status: StatusCode,
    pub(crate) cache_hit: Option<bool>,
    pub(crate) permit: Option<OwnedSemaphorePermit>,
    pub(crate) php: Option<PhpTransferCompletion>,
    pub(crate) execution: Option<PhpExecutionCoordinator>,
}

#[derive(Clone)]
pub(crate) struct TransferLifecycle {
    inner: Arc<TransferLifecycleInner>,
}

struct TransferLifecycleInner {
    state: Mutex<TransferState>,
}

struct TransferState {
    context: Option<TransferContext>,
    transfer: Option<(TransferOutcome, u64)>,
    execution_complete: bool,
    finalized: bool,
    #[cfg(test)]
    observed: Option<(TransferOutcome, u64)>,
}

impl TransferLifecycle {
    pub(crate) fn new(mut context: TransferContext) -> Self {
        let execution = context.execution.take();
        let lifecycle = Self {
            inner: Arc::new(TransferLifecycleInner {
                state: Mutex::new(TransferState {
                    context: Some(context),
                    transfer: None,
                    execution_complete: execution.is_none(),
                    finalized: false,
                    #[cfg(test)]
                    observed: None,
                }),
            }),
        };
        if let Some(execution) = execution {
            execution.register(lifecycle.clone());
        }
        lifecycle
    }

    pub(crate) fn finish_transfer(&self, outcome: TransferOutcome, emitted_bytes: u64) {
        let context = {
            let mut state = self.inner.state.lock().expect("transfer state poisoned");
            if state.transfer.is_some() {
                return;
            }
            state.transfer = Some((outcome, emitted_bytes));
            #[cfg(test)]
            {
                state.observed = Some((outcome, emitted_bytes));
            }
            if !state.execution_complete || state.finalized {
                return;
            }
            state.finalized = true;
            state.context.take()
        };
        if let Some(context) = context {
            finalize(context, outcome, emitted_bytes);
        }
    }

    fn finish_execution(&self, completion: PhpTransferCompletion) {
        let finalization = {
            let mut state = self.inner.state.lock().expect("transfer state poisoned");
            if state.execution_complete {
                return;
            }
            state.execution_complete = true;
            if let Some(context) = state.context.as_mut() {
                context.php = Some(completion);
            }
            if state.transfer.is_none() || state.finalized {
                return;
            }
            state.finalized = true;
            let (outcome, emitted_bytes) = state.transfer.expect("transfer completion exists");
            state
                .context
                .take()
                .map(|context| (context, outcome, emitted_bytes))
        };
        if let Some((context, outcome, emitted_bytes)) = finalization {
            finalize(context, outcome, emitted_bytes);
        }
    }

    #[cfg(test)]
    pub(crate) fn test() -> Self {
        Self {
            inner: Arc::new(TransferLifecycleInner {
                state: Mutex::new(TransferState {
                    context: None,
                    transfer: None,
                    execution_complete: true,
                    finalized: false,
                    observed: None,
                }),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn observed(&self) -> Option<(TransferOutcome, u64)> {
        self.inner
            .state
            .lock()
            .expect("transfer state poisoned")
            .observed
    }
}

#[derive(Clone)]
pub(crate) struct PhpExecutionCoordinator {
    state: Arc<Mutex<PhpExecutionCoordinatorState>>,
}

struct PhpExecutionCoordinatorState {
    lifecycle: Option<TransferLifecycle>,
    completion: Option<PhpTransferCompletion>,
}

impl PhpExecutionCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PhpExecutionCoordinatorState {
                lifecycle: None,
                completion: None,
            })),
        }
    }

    fn register(&self, lifecycle: TransferLifecycle) {
        let completion = {
            let mut state = self.state.lock().expect("PHP completion state poisoned");
            if let Some(completion) = state.completion.take() {
                Some(completion)
            } else {
                state.lifecycle = Some(lifecycle.clone());
                None
            }
        };
        if let Some(completion) = completion {
            lifecycle.finish_execution(completion);
        }
    }

    pub(crate) fn complete(&self, completion: PhpTransferCompletion) {
        let lifecycle = {
            let mut state = self.state.lock().expect("PHP completion state poisoned");
            if let Some(lifecycle) = state.lifecycle.take() {
                Some(lifecycle)
            } else {
                state.completion = Some(completion.clone());
                None
            }
        };
        if let Some(lifecycle) = lifecycle {
            lifecycle.finish_execution(completion);
        }
    }
}

fn finalize(mut context: TransferContext, outcome: TransferOutcome, emitted_bytes: u64) {
    let duration = context.started.elapsed();
    let metrics = &context.state.services.metrics;
    metrics.record_response(context.status);
    metrics
        .response_output_bytes
        .fetch_add(emitted_bytes, Ordering::Relaxed);
    match outcome {
        TransferOutcome::Completed => &metrics.transfers_completed,
        TransferOutcome::Aborted => &metrics.transfers_aborted,
        TransferOutcome::Error => &metrics.transfer_errors,
    }
    .fetch_add(1, Ordering::Relaxed);
    if context.route == "static" {
        metrics
            .static_streamed_bytes
            .fetch_add(emitted_bytes, Ordering::Relaxed);
    }
    if outcome == TransferOutcome::Aborted
        && matches!(context.route, "php" | "front-controller" | "builtin-router")
    {
        metrics
            .client_disconnect_cancellations
            .fetch_add(1, Ordering::Relaxed);
    }

    emit_server_debug_lazy(
        &context.state,
        Some(&context.request_id),
        "D_PHRUST_SERVER_RESPONSE",
        "response",
        "server response transfer finished",
        || {
            BTreeMap::from([
                ("status".to_string(), context.status.as_u16().to_string()),
                ("emitted_bytes".to_string(), emitted_bytes.to_string()),
                ("outcome".to_string(), outcome.as_str().to_string()),
                ("route".to_string(), context.route.to_string()),
                ("duration_ms".to_string(), duration.as_millis().to_string()),
            ])
        },
    );
    write_access_log(
        &context.state,
        AccessLogEntry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            method: &context.method,
            path: &context.request_target,
            status: context.status,
            emitted_bytes,
            duration,
            route: context.route,
            cache_hit: context.cache_hit,
            outcome: outcome.as_str(),
        },
    );

    if let Some(mut php) = context.php.take()
        && let Some(mut trace) = php.trace.take()
    {
        trace.status = context.status.as_u16();
        trace.cache_hit = context.cache_hit;
        trace.failure_phase = php.failure_stage.map(|stage| stage.name());
        trace.response_bytes = emitted_bytes;
        if let Some(writer) = &context.state.observability.perf_trace
            && let Err(error) = writer.write(&trace)
        {
            warn!(%error, path=%writer.path().display(), "perf trace write failed");
        }
        if trace.profile_requested
            && let Some(writer) = &context.state.observability.request_profile
            && let Err(error) = writer.write(&trace, trace.profile_counters.as_ref())
        {
            warn!(%error, dir=%writer.dir().display(), "request profile write failed");
        }
    }

    // The in-flight permit is intentionally owned by the context through all
    // finalization work and is released only here.
    drop(context.permit.take());
}
