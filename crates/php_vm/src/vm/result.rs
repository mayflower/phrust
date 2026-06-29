use crate::counters::VmCounters;
use crate::tiering::TieringStats;
use php_diagnostics::{
    DiagnosticEnvelope, DiagnosticLayer, DiagnosticPhase, DiagnosticSeverity, DiagnosticSuggestion,
};
use php_runtime::{
    ExecutionStatus, OutputBuffer, ReferenceCell, RuntimeDiagnostic, RuntimeHttpResponseState,
    SessionState, UploadRegistry, Value,
};
use std::collections::BTreeMap;

/// Execution result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmResult {
    /// Final execution status.
    pub status: ExecutionStatus,
    /// Captured stdout bytes.
    pub output: OutputBuffer,
    /// Structured runtime diagnostics emitted during execution.
    pub diagnostics: Vec<RuntimeDiagnostic>,
    /// Request-local HTTP response state accumulated by web-response builtins.
    pub http_response: RuntimeHttpResponseState,
    /// Request-local upload registry state after PHP code has executed.
    pub upload_registry: UploadRegistry,
    /// Request-local session state after PHP code has executed.
    pub session: SessionState,
    /// Return value when execution returned successfully.
    pub return_value: Option<Value>,
    /// Process exit code when PHP `exit`/`die` terminated the script.
    pub process_exit_code: Option<i32>,
    pub(super) yielded: Option<super::GeneratorYield>,
    pub(super) fiber_suspension: Option<super::FiberSuspension>,
    pub(super) return_ref: Option<ReferenceCell>,
    /// Deterministic trace events captured when `VmOptions::trace` is enabled.
    pub trace: Vec<String>,
    /// Optional performance VM/runtime counters.
    pub counters: Option<VmCounters>,
    /// Optional performance tiering stats.
    pub tiering_stats: Option<TieringStats>,
}

/// VM control-flow signal, kept separate from runtime diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmControlFlow {
    /// Function return.
    Return(Option<Value>),
    /// Future exception throw signal.
    Throw(Value),
    /// Loop break signal.
    Break,
    /// Loop continue signal.
    Continue,
}

/// Structured VM max-step diagnostic context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmStepLimitDiagnostic {
    /// Configured maximum VM steps.
    pub max_steps: u64,
    /// Current function ID, when available.
    pub function_id: Option<u32>,
    /// Current block ID, when available.
    pub block_id: Option<u32>,
    /// Current instruction ID, when available.
    pub instruction_id: Option<u32>,
    /// Current opcode, when available.
    pub opcode: Option<String>,
}

impl VmStepLimitDiagnostic {
    /// Converts this step-limit failure to the shared diagnostic envelope.
    #[must_use]
    pub fn to_diagnostic_envelope(&self) -> DiagnosticEnvelope {
        let mut context = BTreeMap::new();
        context.insert("max_steps".to_string(), self.max_steps.to_string());
        if let Some(function_id) = self.function_id {
            context.insert("function_id".to_string(), function_id.to_string());
        }
        if let Some(block_id) = self.block_id {
            context.insert("block_id".to_string(), block_id.to_string());
        }
        if let Some(instruction_id) = self.instruction_id {
            context.insert("instruction_id".to_string(), instruction_id.to_string());
        }
        if let Some(opcode) = &self.opcode {
            context.insert("opcode".to_string(), opcode.clone());
        }

        let mut envelope = DiagnosticEnvelope::new(
            "E_PHP_VM_STEP_LIMIT",
            DiagnosticLayer::vm(),
            DiagnosticPhase::new("execute"),
            DiagnosticSeverity::FatalError,
            "VM step limit exceeded",
        )
        .with_context(context);
        envelope.suggestion = Some(DiagnosticSuggestion::new(
            "enable debug mode or reduce the reproducer around the reported instruction",
        ));
        envelope.php_visible = false;
        envelope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_step_limit_has_shared_envelope_context() {
        let diagnostic = VmStepLimitDiagnostic {
            max_steps: 100,
            function_id: Some(1),
            block_id: Some(2),
            instruction_id: Some(3),
            opcode: Some("jump".to_string()),
        };

        let envelope = diagnostic.to_diagnostic_envelope();
        let json: serde_json::Value =
            serde_json::from_str(&envelope.compact_json().expect("json")).expect("parse json");

        assert_eq!(json["code"], "E_PHP_VM_STEP_LIMIT");
        assert_eq!(json["layer"], "vm");
        assert_eq!(json["phase"], "execute");
        assert_eq!(json["context"]["max_steps"], "100");
        assert_eq!(json["context"]["function_id"], "1");
        assert_eq!(json["context"]["block_id"], "2");
        assert_eq!(json["context"]["instruction_id"], "3");
        assert_eq!(json["context"]["opcode"], "jump");
    }
}
