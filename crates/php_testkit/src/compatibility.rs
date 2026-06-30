//! Shared compatibility mismatch taxonomy and reporting helpers.

use serde::{Deserialize, Serialize};

/// Stable mismatch buckets shared by runtime and PHPT differential tooling.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum MismatchCategory {
    /// The PHP reference could not parse or accept the fixture as expected.
    ReferenceParseMismatch,
    /// Phrust rejected source that the reference accepted.
    PhrustParseMismatch,
    /// Frontend lowering or compile-time validation diverged.
    CompileMismatch,
    /// The program reaches an explicitly unsupported feature.
    UnsupportedFeature,
    /// Reference and Phrust process exit status differs.
    RuntimeExitMismatch,
    /// Reference and Phrust stdout differs.
    StdoutMismatch,
    /// Reference and Phrust stderr differs without a stable diagnostic ID.
    StderrMismatch,
    /// Stable diagnostic IDs or diagnostic text differ.
    DiagnosticMismatch,
    /// A run timed out or hit a deterministic nontermination guard.
    TimeoutOrNontermination,
    /// The harness itself failed before producing comparable output.
    HarnessError,
    /// A failure matched an authoritative known-gap entry.
    ExpectedKnownGap,
    /// A known-gap fixture now matches the reference.
    UnexpectedPass,
}

impl MismatchCategory {
    /// Parses a stable report label.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ReferenceParseMismatch" => Some(Self::ReferenceParseMismatch),
            "PhrustParseMismatch" => Some(Self::PhrustParseMismatch),
            "CompileMismatch" => Some(Self::CompileMismatch),
            "UnsupportedFeature" => Some(Self::UnsupportedFeature),
            "RuntimeExitMismatch" => Some(Self::RuntimeExitMismatch),
            "StdoutMismatch" => Some(Self::StdoutMismatch),
            "StderrMismatch" => Some(Self::StderrMismatch),
            "DiagnosticMismatch" => Some(Self::DiagnosticMismatch),
            "TimeoutOrNontermination" => Some(Self::TimeoutOrNontermination),
            "HarnessError" => Some(Self::HarnessError),
            "ExpectedKnownGap" => Some(Self::ExpectedKnownGap),
            "UnexpectedPass" => Some(Self::UnexpectedPass),
            _ => None,
        }
    }

    /// Returns the stable report label for this category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReferenceParseMismatch => "ReferenceParseMismatch",
            Self::PhrustParseMismatch => "PhrustParseMismatch",
            Self::CompileMismatch => "CompileMismatch",
            Self::UnsupportedFeature => "UnsupportedFeature",
            Self::RuntimeExitMismatch => "RuntimeExitMismatch",
            Self::StdoutMismatch => "StdoutMismatch",
            Self::StderrMismatch => "StderrMismatch",
            Self::DiagnosticMismatch => "DiagnosticMismatch",
            Self::TimeoutOrNontermination => "TimeoutOrNontermination",
            Self::HarnessError => "HarnessError",
            Self::ExpectedKnownGap => "ExpectedKnownGap",
            Self::UnexpectedPass => "UnexpectedPass",
        }
    }
}

/// Returns the first 1-based line where two strings differ.
#[must_use]
pub fn first_differing_line(left: &str, right: &str) -> Option<usize> {
    let mut line = 1;
    let mut left_lines = left.lines();
    let mut right_lines = right.lines();
    loop {
        match (left_lines.next(), right_lines.next()) {
            (Some(left), Some(right)) if left == right => {
                line += 1;
            }
            (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) => return Some(line),
            (None, None) => return None,
        }
    }
}

/// Produces a short single-line output summary suitable for reports.
#[must_use]
pub fn summarize_output(value: &str) -> String {
    const LIMIT: usize = 160;
    let line_count = value.lines().count();
    let byte_count = value.len();
    let mut preview = value.replace('\n', "\\n");
    if preview.len() > LIMIT {
        preview.truncate(LIMIT);
        preview.push_str("...");
    }
    format!("{line_count} line(s), {byte_count} byte(s): {preview:?}")
}

/// Classifies a PHPT result detail into the shared taxonomy.
#[must_use]
pub fn classify_phpt_detail(outcome: &str, detail: &str) -> Option<MismatchCategory> {
    if outcome == "PASS" || outcome == "SKIP" {
        return None;
    }
    let detail = detail.to_ascii_lowercase();
    if outcome == "BORK" {
        return Some(MismatchCategory::HarnessError);
    }
    if detail.contains("phpt_timeout") || detail.contains("timeout") {
        Some(MismatchCategory::TimeoutOrNontermination)
    } else if detail.contains("parse") || detail.contains("syntax") {
        Some(MismatchCategory::PhrustParseMismatch)
    } else if detail.contains("compile") || detail.contains("lower") {
        Some(MismatchCategory::CompileMismatch)
    } else if detail.contains("unsupported") || detail.contains("not implemented") {
        Some(MismatchCategory::UnsupportedFeature)
    } else if detail.contains("target exited") {
        Some(MismatchCategory::RuntimeExitMismatch)
    } else if detail.contains("stderr") || detail.contains("diagnostic") {
        Some(MismatchCategory::DiagnosticMismatch)
    } else if detail.contains("expected") || detail.contains("actual") {
        Some(MismatchCategory::StdoutMismatch)
    } else {
        Some(MismatchCategory::HarnessError)
    }
}

#[cfg(test)]
mod tests {
    use super::{MismatchCategory, classify_phpt_detail, first_differing_line, summarize_output};

    #[test]
    fn first_differing_line_is_one_based() {
        assert_eq!(first_differing_line("a\nb\n", "a\nc\n"), Some(2));
        assert_eq!(first_differing_line("a\n", "a\nb\n"), Some(2));
        assert_eq!(first_differing_line("a\n", "a\n"), None);
    }

    #[test]
    fn phpt_details_map_to_shared_categories() {
        assert_eq!(classify_phpt_detail("PASS", ""), None);
        assert_eq!(
            classify_phpt_detail("FAIL", "target exited with status 255"),
            Some(MismatchCategory::RuntimeExitMismatch)
        );
        assert_eq!(
            classify_phpt_detail("FAIL", "expected output did not match actual output"),
            Some(MismatchCategory::StdoutMismatch)
        );
        assert_eq!(
            classify_phpt_detail("BORK", "unsupported section --X--"),
            Some(MismatchCategory::HarnessError)
        );
    }

    #[test]
    fn output_summary_is_bounded() {
        let summary = summarize_output(&"x".repeat(256));
        assert!(summary.contains("256 byte"));
        assert!(summary.len() < 220);
    }
}
