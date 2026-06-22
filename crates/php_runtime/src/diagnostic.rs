//! Runtime diagnostics shared by the VM and CLI.

use crate::builtins::RuntimeSourceSpan;

/// Runtime diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSeverity {
    /// PHP warning; execution may continue.
    Warning,
    /// PHP notice.
    Notice,
    /// PHP deprecation notice.
    Deprecation,
    /// Recoverable runtime error.
    RecoverableError,
    /// Fatal runtime error.
    FatalError,
    /// Explicit unsupported feature.
    UnsupportedFeature,
}

impl RuntimeSeverity {
    /// Stable JSON spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Notice => "notice",
            Self::Deprecation => "deprecation",
            Self::RecoverableError => "recoverable_error",
            Self::FatalError => "fatal_error",
            Self::UnsupportedFeature => "unsupported_feature",
        }
    }
}

/// Optional PHP-reference classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhpReferenceClassification {
    /// PHP warning.
    Warning,
    /// PHP `TypeError`.
    TypeError,
    /// PHP `DivisionByZeroError`.
    DivisionByZeroError,
    /// PHP `Error`.
    Error,
    /// PHP fatal error.
    FatalError,
    /// Unsupported/deferred behavior in this runtime.
    Unsupported,
}

impl PhpReferenceClassification {
    /// Stable JSON spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::TypeError => "type_error",
            Self::DivisionByZeroError => "division_by_zero_error",
            Self::Error => "error",
            Self::FatalError => "fatal_error",
            Self::Unsupported => "unsupported",
        }
    }
}

/// One deterministic runtime stack frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStackFrame {
    function: String,
}

impl RuntimeStackFrame {
    /// Creates a stack frame entry.
    #[must_use]
    pub fn new(function: impl Into<String>) -> Self {
        Self {
            function: function.into(),
        }
    }

    /// Function name.
    #[must_use]
    pub fn function(&self) -> &str {
        &self.function
    }
}

/// Structured runtime diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDiagnostic {
    id: String,
    severity: RuntimeSeverity,
    message: String,
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
    php_reference: Option<PhpReferenceClassification>,
}

impl RuntimeDiagnostic {
    /// Creates a diagnostic.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        severity: RuntimeSeverity,
        message: impl Into<String>,
        source_span: RuntimeSourceSpan,
        stack_trace: Vec<RuntimeStackFrame>,
        php_reference: Option<PhpReferenceClassification>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            message: message.into(),
            source_span,
            stack_trace,
            php_reference,
        }
    }

    /// Diagnostic ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Severity.
    #[must_use]
    pub const fn severity(&self) -> RuntimeSeverity {
        self.severity
    }

    /// Message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Source span.
    #[must_use]
    pub const fn source_span(&self) -> &RuntimeSourceSpan {
        &self.source_span
    }

    /// Stack trace frames.
    #[must_use]
    pub fn stack_trace(&self) -> &[RuntimeStackFrame] {
        &self.stack_trace
    }

    /// PHP-reference classification.
    #[must_use]
    pub const fn php_reference(&self) -> Option<PhpReferenceClassification> {
        self.php_reference
    }

    /// Stable compact JSON representation.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\"id\":\"");
        out.push_str(&escape_json(&self.id));
        out.push_str("\",\"severity\":\"");
        out.push_str(self.severity.as_str());
        out.push_str("\",\"message\":\"");
        out.push_str(&escape_json(&self.message));
        out.push_str("\",\"span\":{");
        out.push_str("\"file\":");
        match &self.source_span.file {
            Some(file) => {
                out.push('"');
                out.push_str(&escape_json(file));
                out.push('"');
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"start\":");
        out.push_str(&self.source_span.start.to_string());
        out.push_str(",\"end\":");
        out.push_str(&self.source_span.end.to_string());
        out.push_str("},\"stack\":[");
        for (index, frame) in self.stack_trace.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("{\"function\":\"");
            out.push_str(&escape_json(frame.function()));
            out.push_str("\"}");
        }
        out.push_str("],\"php_reference\":");
        match self.php_reference {
            Some(classification) => {
                out.push('"');
                out.push_str(classification.as_str());
                out.push('"');
            }
            None => out.push_str("null"),
        }
        out.push('}');
        out
    }
}

/// Runtime error wrapper kept separate from VM control flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeError {
    diagnostic: RuntimeDiagnostic,
}

impl RuntimeError {
    /// Creates a runtime error.
    #[must_use]
    pub const fn new(diagnostic: RuntimeDiagnostic) -> Self {
        Self { diagnostic }
    }

    /// Returns the diagnostic.
    #[must_use]
    pub const fn diagnostic(&self) -> &RuntimeDiagnostic {
        &self.diagnostic
    }
}

/// Undefined variable warning helper.
#[must_use]
pub fn undefined_variable_warning(
    name: impl Into<String>,
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    let name = name.into();
    RuntimeDiagnostic::new(
        "E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING",
        RuntimeSeverity::Warning,
        format!("undefined variable ${name}"),
        source_span,
        stack_trace,
        Some(PhpReferenceClassification::Warning),
    )
}

/// TypeError MVP helper.
#[must_use]
pub fn type_error_mvp(
    message: impl Into<String>,
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic::new(
        "E_PHP_RUNTIME_TYPE_ERROR",
        RuntimeSeverity::FatalError,
        message,
        source_span,
        stack_trace,
        Some(PhpReferenceClassification::TypeError),
    )
}

/// DivisionByZero MVP helper.
#[must_use]
pub fn division_by_zero_mvp(
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic::new(
        "E_PHP_RUNTIME_DIVISION_BY_ZERO",
        RuntimeSeverity::FatalError,
        "division by zero",
        source_span,
        stack_trace,
        Some(PhpReferenceClassification::DivisionByZeroError),
    )
}

/// Undefined function helper.
#[must_use]
pub fn undefined_function(
    name: impl AsRef<str>,
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic::new(
        "E_PHP_RUNTIME_UNDEFINED_FUNCTION",
        RuntimeSeverity::FatalError,
        format!("undefined function {}", name.as_ref()),
        source_span,
        stack_trace,
        Some(PhpReferenceClassification::Error),
    )
}

/// Unsupported feature helper.
#[must_use]
pub fn unsupported_feature(
    id: impl Into<String>,
    message: impl Into<String>,
    source_span: RuntimeSourceSpan,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    RuntimeDiagnostic::new(
        id,
        RuntimeSeverity::UnsupportedFeature,
        message,
        source_span,
        stack_trace,
        Some(PhpReferenceClassification::Unsupported),
    )
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeDiagnostic, RuntimeSeverity, RuntimeStackFrame, division_by_zero_mvp,
        undefined_function, undefined_variable_warning,
    };
    use crate::RuntimeSourceSpan;

    #[test]
    fn errors_runtime_diagnostics_are_stable_json() {
        let diagnostic = undefined_function(
            "missing",
            RuntimeSourceSpan {
                file: Some("fixture.php".to_owned()),
                start: 1,
                end: 8,
            },
            vec![RuntimeStackFrame::new("main")],
        );

        assert_eq!(diagnostic.id(), "E_PHP_RUNTIME_UNDEFINED_FUNCTION");
        assert_eq!(diagnostic.severity(), RuntimeSeverity::FatalError);
        assert_eq!(
            diagnostic.to_json(),
            "{\"id\":\"E_PHP_RUNTIME_UNDEFINED_FUNCTION\",\"severity\":\"fatal_error\",\"message\":\"undefined function missing\",\"span\":{\"file\":\"fixture.php\",\"start\":1,\"end\":8},\"stack\":[{\"function\":\"main\"}],\"php_reference\":\"error\"}"
        );
    }

    #[test]
    fn errors_helpers_cover_warning_and_division_by_zero() {
        let warning =
            undefined_variable_warning("missing", RuntimeSourceSpan::default(), Vec::new());
        assert_eq!(warning.id(), "E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING");
        assert_eq!(warning.severity(), RuntimeSeverity::Warning);

        let division = division_by_zero_mvp(RuntimeSourceSpan::default(), Vec::new());
        assert_eq!(division.id(), "E_PHP_RUNTIME_DIVISION_BY_ZERO");
        assert_eq!(division.message(), "division by zero");
    }

    #[test]
    fn exception_diagnostics_preserve_severity_and_source_mapping() {
        let span = RuntimeSourceSpan {
            file: Some("fixtures/phase5/errors/type-error-uncaught.php".to_owned()),
            start: 12,
            end: 21,
        };
        let warning = undefined_variable_warning("missing", span.clone(), Vec::new());
        let fatal = RuntimeDiagnostic::new(
            "E_PHP_VM_UNCAUGHT_EXCEPTION",
            RuntimeSeverity::FatalError,
            "Uncaught TypeError: bad".to_owned(),
            span.clone(),
            vec![RuntimeStackFrame::new("main")],
            None,
        );

        assert_eq!(warning.severity(), RuntimeSeverity::Warning);
        assert_eq!(fatal.severity(), RuntimeSeverity::FatalError);
        let json = fatal.to_json();
        assert!(json.contains("type-error-uncaught.php"));
        assert!(json.contains("\"start\":12"));
        assert!(json.contains("\"end\":21"));
    }
}
