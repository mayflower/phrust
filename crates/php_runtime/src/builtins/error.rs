//! Builtin error reporting.

/// Runtime error reported by a builtin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltinError {
    diagnostic_id: &'static str,
    message: String,
}

impl BuiltinError {
    /// Creates a builtin error with a stable diagnostic ID.
    #[must_use]
    pub fn new(diagnostic_id: &'static str, message: impl Into<String>) -> Self {
        Self {
            diagnostic_id,
            message: message.into(),
        }
    }

    /// Stable diagnostic ID.
    #[must_use]
    pub const fn diagnostic_id(&self) -> &'static str {
        self.diagnostic_id
    }

    /// Human-readable message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Combines ID and message for VM runtime errors.
    #[must_use]
    pub fn display_message(&self) -> String {
        format!("{}: {}", self.diagnostic_id, self.message)
    }
}
