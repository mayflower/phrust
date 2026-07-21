use crate::{
    multipart::cleanup_uploaded_files, perf_trace::PerfTraceEvent, response::ResponseBody,
    sessions::finalize_session_state, state::AppState,
};
use hyper::Response;
use php_executor::PhpExecutionOutput;
use php_runtime::api::RuntimeUploadedFile;

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
    uploads: Vec<RuntimeUploadedFile>,
    armed: bool,
}

impl RequestCleanup {
    pub(crate) fn new(uploads: Vec<RuntimeUploadedFile>) -> Self {
        Self {
            uploads,
            armed: true,
        }
    }

    pub(crate) fn finalize_output(
        mut self,
        output: &mut PhpExecutionOutput,
        state: &AppState,
    ) -> Result<(), String> {
        output.upload_registry.cleanup_unmoved();
        self.armed = false;
        finalize_session_state(output, state)
    }
}

impl Drop for RequestCleanup {
    fn drop(&mut self) {
        if self.armed {
            cleanup_uploaded_files(&self.uploads);
        }
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "phrust-request-cleanup-{}-{nonce}",
            std::process::id()
        ));
        std::fs::write(&path, b"upload").expect("write upload fixture");
        {
            let _cleanup = RequestCleanup::new(vec![RuntimeUploadedFile {
                field_name: "file".to_string(),
                client_filename: "fixture.txt".to_string(),
                content_type: "text/plain".to_string(),
                temp_path: path.to_string_lossy().into_owned(),
                error: 0,
                size: 6,
            }]);
        }
        assert!(!path.exists());
    }
}
