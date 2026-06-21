//! Lowering context shared by Phase 3 collector passes.

use crate::diagnostics::{DiagnosticReporter, SemanticDiagnostic};

/// Mutable context for semantic lowering and collection passes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoweringContext {
    reporter: DiagnosticReporter,
}

impl LoweringContext {
    /// Creates an empty lowering context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns mutable diagnostic reporter.
    #[must_use]
    pub fn reporter_mut(&mut self) -> &mut DiagnosticReporter {
        &mut self.reporter
    }

    /// Finishes lowering and returns diagnostics.
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<SemanticDiagnostic> {
        self.reporter.into_diagnostics()
    }
}
