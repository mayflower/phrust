use crate::{
    perf_trace::PerfTraceEvent, response::ResponseBody, sessions::SessionRequestCallbacks,
};
use hyper::Response;
use php_executor::PhpExecutionOutput;
use std::sync::{Arc, Weak, atomic::Ordering};

#[derive(Debug, Default)]
pub(crate) struct RequestUploadSet {
    temp_paths: Vec<tempfile::TempPath>,
    bytes: u64,
    metrics: Option<Weak<crate::metrics::ServerMetrics>>,
}

impl RequestUploadSet {
    pub(crate) fn with_metrics(metrics: &Arc<crate::metrics::ServerMetrics>) -> Self {
        Self {
            temp_paths: Vec::new(),
            bytes: 0,
            metrics: Some(Arc::downgrade(metrics)),
        }
    }

    pub(crate) fn push(&mut self, path: tempfile::TempPath) {
        self.temp_paths.push(path);
        if let Some(metrics) = self.metrics.as_ref().and_then(Weak::upgrade) {
            metrics
                .upload_tempfiles_active
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn add_bytes(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_add(bytes);
        if let Some(metrics) = self.metrics.as_ref().and_then(Weak::upgrade) {
            metrics
                .upload_tempfile_bytes_active
                .fetch_add(bytes, Ordering::Relaxed);
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.temp_paths.len()
    }
}

impl Drop for RequestUploadSet {
    fn drop(&mut self) {
        if let Some(metrics) = self.metrics.as_ref().and_then(Weak::upgrade) {
            metrics
                .upload_tempfiles_active
                .fetch_sub(self.temp_paths.len() as u64, Ordering::Relaxed);
            metrics
                .upload_tempfile_bytes_active
                .fetch_sub(self.bytes, Ordering::Relaxed);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RequestStage {
    RouteTargetSelection,
    BodyAndMultipart,
    SessionLoad,
    ExecutorAcquisition,
    Execution,
    SessionAndUploadCleanup,
}

impl RequestStage {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::RouteTargetSelection => "routing",
            Self::BodyAndMultipart => "body_multipart",
            Self::SessionLoad => "session_seed",
            Self::ExecutorAcquisition => "executor_acquisition",
            Self::Execution => "php_vm_execution",
            Self::SessionAndUploadCleanup => "session_finalize",
        }
    }
}

/// Owns request-local cleanup until execution transfers it to the runtime
/// output or exits through an error path.
pub(crate) struct RequestCleanup {
    _uploads: Arc<RequestUploadSet>,
    sessions: Option<SessionRequestCallbacks>,
    armed: bool,
}

impl RequestCleanup {
    pub(crate) fn new(
        uploads: Arc<RequestUploadSet>,
        sessions: Option<SessionRequestCallbacks>,
    ) -> Self {
        Self {
            _uploads: uploads,
            sessions,
            armed: true,
        }
    }

    pub(crate) fn finalize_output(
        mut self,
        output: &mut PhpExecutionOutput,
        _state: &crate::state::AppState,
    ) -> Result<(), String> {
        output.upload_registry.cleanup_unmoved();
        self.armed = false;
        self.sessions
            .as_ref()
            .map_or(Ok(()), |sessions| sessions.finalize(output))
    }
}

impl Drop for RequestCleanup {
    fn drop(&mut self) {
        let _ = self.armed;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PhpTransferCompletion {
    pub(crate) trace: Option<PerfTraceEvent>,
    pub(crate) failure_stage: Option<RequestStage>,
}

/// PHP execution data carried with the response until the shared transfer
/// lifecycle completes. No request-final observability is emitted here.
pub(crate) struct RequestOutcome {
    response: Response<ResponseBody>,
    cache_hit: Option<bool>,
    failure_stage: Option<RequestStage>,
}

impl RequestOutcome {
    pub(crate) fn success(response: Response<ResponseBody>, cache_hit: Option<bool>) -> Self {
        Self {
            response,
            cache_hit,
            failure_stage: None,
        }
    }

    pub(crate) fn failure(
        response: Response<ResponseBody>,
        cache_hit: Option<bool>,
        stage: RequestStage,
    ) -> Self {
        Self {
            response,
            cache_hit,
            failure_stage: Some(stage),
        }
    }

    pub(crate) fn into_response(
        mut self,
        trace: Option<PerfTraceEvent>,
    ) -> (Response<ResponseBody>, Option<bool>) {
        self.response
            .extensions_mut()
            .insert(PhpTransferCompletion {
                trace,
                failure_stage: self.failure_stage,
            });
        (self.response, self.cache_hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_stage_names_are_stable_and_distinct() {
        let stages = [
            RequestStage::RouteTargetSelection,
            RequestStage::BodyAndMultipart,
            RequestStage::SessionLoad,
            RequestStage::ExecutorAcquisition,
            RequestStage::Execution,
            RequestStage::SessionAndUploadCleanup,
        ];
        let mut names = stages.map(RequestStage::name).to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), stages.len());
    }

    #[test]
    fn dropped_request_cleanup_removes_uploaded_temp_files() {
        let named = tempfile::Builder::new()
            .prefix("phrust-request-cleanup-")
            .tempfile_in(std::env::temp_dir())
            .expect("create upload fixture");
        std::fs::write(named.path(), b"upload").expect("write upload fixture");
        let path = named.path().to_path_buf();
        let mut uploads = RequestUploadSet::default();
        uploads.push(named.into_temp_path());
        {
            let _cleanup = RequestCleanup::new(Arc::new(uploads), None);
        }
        assert!(!path.exists());
    }
}
