//! Typed VM diagnostics used between VM subsystems.

use php_diagnostics::{
    DiagnosticEnvelope, DiagnosticLayer, DiagnosticLocation, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticSpan,
};
use php_runtime::api::RuntimeSourceSpan;
use std::collections::BTreeMap;
use std::fmt;

/// VM-local diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VmErrorSeverity {
    Warning,
    FatalError,
    Internal,
}

impl VmErrorSeverity {
    fn diagnostic_severity(self) -> DiagnosticSeverity {
        match self {
            Self::Warning => DiagnosticSeverity::Warning,
            Self::FatalError | Self::Internal => DiagnosticSeverity::FatalError,
        }
    }

    fn php_visible(self) -> bool {
        matches!(self, Self::Warning | Self::FatalError)
    }
}

/// Structured VM error with a stable code and machine-readable context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmError {
    code: &'static str,
    severity: VmErrorSeverity,
    phase: &'static str,
    span: Option<RuntimeSourceSpan>,
    context: BTreeMap<String, String>,
    message: String,
}

impl VmError {
    /// Creates a PHP-visible fatal VM error.
    #[must_use]
    pub fn fatal(code: &'static str, phase: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, VmErrorSeverity::FatalError, phase, message)
    }

    /// Creates an internal VM failure.
    #[must_use]
    pub fn internal(code: &'static str, phase: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, VmErrorSeverity::Internal, phase, message)
    }

    /// Creates a VM warning.
    #[must_use]
    pub fn warning(code: &'static str, phase: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, VmErrorSeverity::Warning, phase, message)
    }

    #[must_use]
    pub fn new(
        code: &'static str,
        severity: VmErrorSeverity,
        phase: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            phase,
            span: None,
            context: BTreeMap::new(),
            message: message.into(),
        }
    }

    /// Adds a source span to the error.
    #[must_use]
    pub fn with_span(mut self, span: RuntimeSourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    /// Adds one stable context value.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.context.insert(key.into(), value.to_string());
        self
    }

    /// Stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Internal phase that produced the diagnostic.
    #[must_use]
    pub const fn phase(&self) -> &'static str {
        self.phase
    }

    /// Human-readable message without the diagnostic code prefix.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Machine-readable context.
    #[must_use]
    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    /// Source span, when the VM boundary can attach one.
    #[must_use]
    pub const fn source_span(&self) -> Option<&RuntimeSourceSpan> {
        self.span.as_ref()
    }

    /// Renders the legacy runtime-error string used at the current VM boundary.
    #[must_use]
    pub fn render_message(&self) -> String {
        format!("{}: {}", self.code, self.message)
    }

    /// Converts this VM error to the shared diagnostics envelope.
    #[must_use]
    pub fn to_diagnostic_envelope(&self) -> DiagnosticEnvelope {
        let mut envelope = DiagnosticEnvelope::new(
            self.code,
            DiagnosticLayer::vm(),
            DiagnosticPhase::new(self.phase),
            self.severity.diagnostic_severity(),
            self.render_message(),
        )
        .with_context(self.context.clone());
        if let Some(span) = &self.span {
            envelope = envelope.with_location(DiagnosticLocation::new(
                span.file.clone(),
                None::<String>,
                Some(DiagnosticSpan::new(span.start as usize, span.end as usize)),
            ));
        }
        envelope.php_visible = self.severity.php_visible();
        envelope
    }
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render_message())
    }
}

impl std::error::Error for VmError {}

impl From<VmError> for String {
    fn from(error: VmError) -> Self {
        error.render_message()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_diagnostics::DiagnosticSeverity;

    #[test]
    fn vm_error_envelope_preserves_stable_code_status_context_and_span() {
        let span = RuntimeSourceSpan {
            file: Some("fixture.php".to_string()),
            start: 12,
            end: 20,
        };
        let error = VmError::fatal(
            "E_PHP_VM_INVALID_LOCAL_SLOT",
            "frame",
            "invalid local slot l42",
        )
        .with_span(span.clone())
        .with_context("slot", "l42")
        .with_context("operation", "set");

        assert_eq!(error.code(), "E_PHP_VM_INVALID_LOCAL_SLOT");
        assert_eq!(error.phase(), "frame");
        assert_eq!(error.message(), "invalid local slot l42");
        assert_eq!(error.source_span(), Some(&span));

        let envelope = error.to_diagnostic_envelope();
        assert_eq!(envelope.code, "E_PHP_VM_INVALID_LOCAL_SLOT");
        assert_eq!(envelope.severity, DiagnosticSeverity::FatalError);
        assert!(envelope.php_visible);
        assert_eq!(
            envelope.context.get("slot").map(String::as_str),
            Some("l42")
        );
        assert_eq!(
            envelope.context.get("operation").map(String::as_str),
            Some("set")
        );

        let location = envelope.location.expect("vm diagnostic location");
        assert_eq!(location.path.as_deref(), Some("fixture.php"));
        let span = location.span.expect("vm diagnostic span");
        assert_eq!(span.start, 12);
        assert_eq!(span.end, 20);
    }

    #[test]
    fn internal_vm_error_is_not_php_visible() {
        let envelope =
            VmError::internal("E_PHP_VM_FRAME_CORRUPT", "frame", "frame state is corrupt")
                .to_diagnostic_envelope();

        assert_eq!(envelope.code, "E_PHP_VM_FRAME_CORRUPT");
        assert_eq!(envelope.severity, DiagnosticSeverity::FatalError);
        assert!(!envelope.php_visible);
    }
}
