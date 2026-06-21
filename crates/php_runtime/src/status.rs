//! Runtime execution status classification.

use std::fmt;

/// Final process-style status for an execution attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitStatus {
    /// Execution completed successfully.
    Success,
    /// Input failed to compile.
    CompileError,
    /// Runtime execution failed.
    RuntimeError,
    /// Execution encountered an unsupported feature.
    Unsupported,
    /// Fatal engine-level failure.
    Fatal,
}

/// Execution status with optional message payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionStatus {
    exit_status: ExitStatus,
    message: Option<String>,
}

impl ExecutionStatus {
    /// Successful execution.
    #[must_use]
    pub const fn success() -> Self {
        Self {
            exit_status: ExitStatus::Success,
            message: None,
        }
    }

    /// Compile error status.
    #[must_use]
    pub fn compile_error(message: impl Into<String>) -> Self {
        Self::with_message(ExitStatus::CompileError, message)
    }

    /// Runtime error status.
    #[must_use]
    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::with_message(ExitStatus::RuntimeError, message)
    }

    /// Unsupported feature status.
    #[must_use]
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::with_message(ExitStatus::Unsupported, message)
    }

    /// Fatal status.
    #[must_use]
    pub fn fatal(message: impl Into<String>) -> Self {
        Self::with_message(ExitStatus::Fatal, message)
    }

    /// Returns the status class.
    #[must_use]
    pub const fn exit_status(&self) -> ExitStatus {
        self.exit_status
    }

    /// Returns the optional message.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns true for successful execution.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self.exit_status, ExitStatus::Success)
    }

    fn with_message(exit_status: ExitStatus, message: impl Into<String>) -> Self {
        Self {
            exit_status,
            message: Some(message.into()),
        }
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Success => "success",
            Self::CompileError => "compile_error",
            Self::RuntimeError => "runtime_error",
            Self::Unsupported => "unsupported",
            Self::Fatal => "fatal",
        })
    }
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.message() {
            Some(message) => write!(f, "{}: {message}", self.exit_status),
            None => self.exit_status.fmt(f),
        }
    }
}
