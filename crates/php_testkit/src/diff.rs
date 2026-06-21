use serde::{Deserialize, Serialize};

/// Normalized parser-side result for acceptance comparison.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RustParseResult {
    /// Source file path.
    pub file: String,
    /// True when the Rust parser accepts the file.
    pub ok: bool,
    /// Number of parser diagnostics.
    pub diagnostics: usize,
}

/// Acceptance comparison result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ParserAcceptanceDiff {
    /// Source file path.
    pub file: String,
    /// PHP reference acceptance.
    pub reference_ok: bool,
    /// Rust parser acceptance.
    pub rust_ok: bool,
}

impl ParserAcceptanceDiff {
    /// Returns true when both sides agree.
    #[must_use]
    pub const fn matches(&self) -> bool {
        self.reference_ok == self.rust_ok
    }
}

/// Acceptance comparison result for the semantic frontend.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticAcceptanceDiff {
    /// Source file path.
    pub file: String,
    /// PHP reference frontend acceptance.
    pub reference_ok: bool,
    /// Rust semantic frontend acceptance.
    pub rust_ok: bool,
}

impl SemanticAcceptanceDiff {
    /// Returns true when both sides agree.
    #[must_use]
    pub const fn matches(&self) -> bool {
        self.reference_ok == self.rust_ok
    }
}

/// Phase 3 semantic frontend comparison status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SemanticDiffStatus {
    /// Reference and Rust both accept.
    MatchAccepted,
    /// Reference and Rust both reject.
    MatchRejected,
    /// Rust accepts a reference reject.
    RustAcceptsReferenceRejects,
    /// Rust rejects a reference accept.
    RustRejectsReferenceAccepts,
    /// No reference PHP binary was available.
    ReferenceUnavailable,
    /// Difference is explicit and documented.
    KnownGap,
    /// Comparison was skipped for an environmental reason.
    Skipped,
}

/// Normalized semantic frontend diff row.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticDiff {
    /// Fixture path.
    pub fixture_path: String,
    /// PHP reference frontend acceptance, when available.
    pub reference_ok: Option<bool>,
    /// Rust semantic frontend acceptance.
    pub rust_ok: bool,
    /// Normalized comparison status.
    pub status: SemanticDiffStatus,
    /// Reference classification such as accepted, rejected, or timeout.
    pub reference_classification: Option<String>,
    /// Semantic diagnostic IDs emitted by the Rust frontend.
    pub rust_diagnostic_ids: Vec<String>,
    /// Additional deterministic notes.
    pub notes: Vec<String>,
}

impl SemanticDiff {
    /// Builds a diff row from acceptance booleans.
    #[must_use]
    pub fn from_acceptance(
        fixture_path: impl Into<String>,
        reference_ok: Option<bool>,
        rust_ok: bool,
        reference_classification: Option<String>,
        rust_diagnostic_ids: Vec<String>,
        notes: Vec<String>,
        known_gap: bool,
    ) -> Self {
        let status = match (reference_ok, known_gap) {
            (None, _) => SemanticDiffStatus::ReferenceUnavailable,
            (Some(_), true) => SemanticDiffStatus::KnownGap,
            (Some(true), false) if rust_ok => SemanticDiffStatus::MatchAccepted,
            (Some(false), false) if !rust_ok => SemanticDiffStatus::MatchRejected,
            (Some(false), false) => SemanticDiffStatus::RustAcceptsReferenceRejects,
            (Some(true), false) => SemanticDiffStatus::RustRejectsReferenceAccepts,
        };
        Self {
            fixture_path: fixture_path.into(),
            reference_ok,
            rust_ok,
            status,
            reference_classification,
            rust_diagnostic_ids,
            notes,
        }
    }

    /// Returns true when this row does not represent an unexpected mismatch.
    #[must_use]
    pub const fn is_expected(&self) -> bool {
        matches!(
            self.status,
            SemanticDiffStatus::MatchAccepted
                | SemanticDiffStatus::MatchRejected
                | SemanticDiffStatus::ReferenceUnavailable
                | SemanticDiffStatus::KnownGap
                | SemanticDiffStatus::Skipped
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ParserAcceptanceDiff, SemanticAcceptanceDiff, SemanticDiff, SemanticDiffStatus};

    #[test]
    fn reports_acceptance_match() {
        let diff = ParserAcceptanceDiff {
            file: "fixture.php".to_owned(),
            reference_ok: true,
            rust_ok: true,
        };
        assert!(diff.matches());
    }

    #[test]
    fn reports_semantic_acceptance_match() {
        let diff = SemanticAcceptanceDiff {
            file: "fixture.php".to_owned(),
            reference_ok: false,
            rust_ok: false,
        };
        assert!(diff.matches());
    }

    #[test]
    fn reports_semantic_diff_statuses() {
        let matched = SemanticDiff::from_acceptance(
            "fixtures/semantic/valid/minimal.php",
            Some(true),
            true,
            Some("accepted".to_owned()),
            vec![],
            vec![],
            false,
        );
        assert_eq!(matched.status, SemanticDiffStatus::MatchAccepted);
        assert!(matched.is_expected());

        let mismatch = SemanticDiff::from_acceptance(
            "fixtures/semantic/invalid/missing-semicolon.php",
            Some(false),
            true,
            Some("rejected".to_owned()),
            vec![],
            vec![],
            false,
        );
        assert_eq!(
            mismatch.status,
            SemanticDiffStatus::RustAcceptsReferenceRejects
        );
        assert!(!mismatch.is_expected());

        let unavailable = SemanticDiff::from_acceptance(
            "fixtures/semantic/valid/minimal.php",
            None,
            true,
            None,
            vec![],
            vec!["no PHP reference".to_owned()],
            false,
        );
        assert_eq!(unavailable.status, SemanticDiffStatus::ReferenceUnavailable);
        assert!(unavailable.is_expected());
    }
}
