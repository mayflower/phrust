//! Runtime fixture metadata and comparison result formats.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Runtime fixture expectation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFixtureExpectation {
    /// Reference and Rust outputs are expected to match.
    Pass,
    /// Fixture is an explicit known gap.
    KnownGap,
    /// Fixture is expected to fail on the Rust side.
    Fail,
    /// Fixture should be skipped.
    Skip,
}

impl RuntimeFixtureExpectation {
    /// Parses a metadata expectation value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pass" => Some(Self::Pass),
            "known_gap" => Some(Self::KnownGap),
            "fail" => Some(Self::Fail),
            "skip" => Some(Self::Skip),
            _ => None,
        }
    }
}

/// Runtime fixture category.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFixtureKind {
    /// Expected to execute successfully.
    Valid,
    /// Expected to produce a runtime error.
    Invalid,
    /// Explicit unsupported or deferred runtime feature.
    KnownGap,
    /// Any other runtime fixture group.
    Other,
}

/// Runtime fixture metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeFixture {
    /// Fixture path.
    pub path: PathBuf,
    /// Inferred fixture kind.
    pub kind: RuntimeFixtureKind,
    /// Expected comparison behavior.
    pub expect: RuntimeFixtureExpectation,
    /// Whether PHP reference execution is required for this fixture.
    pub php_ref_required: bool,
    /// Optional named normalization policy.
    pub normalize: Option<String>,
    /// Optional stable known-gap ID.
    pub known_gap_id: Option<String>,
    /// Script arguments passed after `--`.
    pub args: Vec<String>,
}

impl RuntimeFixture {
    /// Creates runtime fixture metadata.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        let kind = infer_runtime_kind(&path);
        let mut fixture = Self {
            path,
            kind,
            expect: match kind {
                RuntimeFixtureKind::KnownGap => RuntimeFixtureExpectation::KnownGap,
                RuntimeFixtureKind::Invalid => RuntimeFixtureExpectation::Fail,
                RuntimeFixtureKind::Valid | RuntimeFixtureKind::Other => {
                    RuntimeFixtureExpectation::Pass
                }
            },
            php_ref_required: false,
            normalize: None,
            known_gap_id: None,
            args: Vec::new(),
        };
        fixture.apply_comment_metadata();
        fixture
    }

    /// Returns a stable display path.
    #[must_use]
    pub fn display_path(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    fn apply_comment_metadata(&mut self) {
        let Ok(source) = std::fs::read_to_string(&self.path) else {
            return;
        };
        for line in source.lines().take(8) {
            let Some(metadata) = line
                .trim()
                .strip_prefix("// phase4-runtime:")
                .or_else(|| line.trim().strip_prefix("# phase4-runtime:"))
            else {
                continue;
            };
            for item in metadata.split_whitespace() {
                let Some((key, value)) = item.split_once('=') else {
                    continue;
                };
                let value = value.trim_matches('"');
                match key {
                    "expect" => {
                        if let Some(expect) = RuntimeFixtureExpectation::parse(value) {
                            self.expect = expect;
                        }
                    }
                    "php_ref_required" => {
                        self.php_ref_required = matches!(value, "true" | "1" | "yes");
                    }
                    "normalize" => self.normalize = Some(value.to_owned()),
                    "known_gap" => self.known_gap_id = Some(value.to_owned()),
                    "args" => {
                        self.args = value
                            .split(',')
                            .filter(|part| !part.is_empty())
                            .map(str::to_owned)
                            .collect();
                    }
                    _ => {}
                }
            }
        }
    }
}

/// A normalized side of a runtime comparison.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeSideResult {
    /// Exit code classification or numeric code string.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Normalized stderr or diagnostic output.
    pub stderr_normalized: String,
}

/// Runtime comparison status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeComparisonStatus {
    /// Reference and Rust runtime match.
    Pass,
    /// Reference and Rust runtime differ unexpectedly.
    Fail,
    /// Runtime comparison could not run in this environment.
    Skipped,
    /// Difference is explicitly documented as a known gap.
    KnownGap,
}

/// JSON shape for Phase 4 runtime comparison results.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeComparisonResult {
    /// Fixture path.
    pub file: String,
    /// PHP reference side when available.
    pub reference: Option<RuntimeSideResult>,
    /// Rust runtime side when available.
    pub rust: Option<RuntimeSideResult>,
    /// Comparison status.
    pub status: RuntimeComparisonStatus,
    /// Stable runtime diagnostic IDs emitted by the Rust runtime.
    pub diagnostic_ids: Vec<String>,
    /// Known-gap ID when `status` is `KnownGap`.
    pub known_gap_id: Option<String>,
    /// Human-readable failure or skip details.
    pub message: Option<String>,
}

impl RuntimeComparisonResult {
    /// Serializes this result to stable pretty JSON.
    pub fn to_pretty_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

fn infer_runtime_kind(path: &Path) -> RuntimeFixtureKind {
    let path = path.to_string_lossy();
    if path.contains("/known_gaps/") {
        RuntimeFixtureKind::KnownGap
    } else if path.contains("/invalid/") {
        RuntimeFixtureKind::Invalid
    } else if path.contains("/valid/") {
        RuntimeFixtureKind::Valid
    } else {
        RuntimeFixtureKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeComparisonResult, RuntimeComparisonStatus, RuntimeFixture,
        RuntimeFixtureExpectation, RuntimeFixtureKind, RuntimeSideResult,
    };
    use std::path::PathBuf;

    #[test]
    fn runtime_fixture_kind_is_inferred_from_path() {
        assert_eq!(
            RuntimeFixture::new(PathBuf::from("fixtures/runtime/valid/hello.php")).kind,
            RuntimeFixtureKind::Valid
        );
        assert_eq!(
            RuntimeFixture::new(PathBuf::from("fixtures/runtime/valid/hello.php")).expect,
            RuntimeFixtureExpectation::Pass
        );
        assert_eq!(
            RuntimeFixture::new(PathBuf::from("fixtures/runtime/invalid/runtime-error.php")).kind,
            RuntimeFixtureKind::Invalid
        );
        assert_eq!(
            RuntimeFixture::new(PathBuf::from(
                "fixtures/runtime/known_gaps/generators/yield.php"
            ))
            .kind,
            RuntimeFixtureKind::KnownGap
        );
    }

    #[test]
    fn runtime_fixture_comment_metadata_is_parsed() {
        let dir =
            std::env::temp_dir().join(format!("phrust-runtime-fixture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("fixture.php");
        std::fs::write(
            &path,
            "<?php\n// phase4-runtime: expect=known_gap php_ref_required=true normalize=path_lines known_gap=E_TEST args=one,two\n",
        )
        .expect("fixture write");

        let fixture = RuntimeFixture::new(path.clone());

        assert_eq!(fixture.expect, RuntimeFixtureExpectation::KnownGap);
        assert!(fixture.php_ref_required);
        assert_eq!(fixture.normalize.as_deref(), Some("path_lines"));
        assert_eq!(fixture.known_gap_id.as_deref(), Some("E_TEST"));
        assert_eq!(fixture.args, ["one", "two"]);
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn runtime_comparison_result_has_expected_json_shape() {
        let result = RuntimeComparisonResult {
            file: "fixtures/runtime/valid/hello.php".to_owned(),
            reference: Some(RuntimeSideResult {
                exit_code: Some(0),
                stdout: "hello phase4\n".to_owned(),
                stderr_normalized: String::new(),
            }),
            rust: None,
            status: RuntimeComparisonStatus::Skipped,
            diagnostic_ids: vec![],
            known_gap_id: None,
            message: None,
        };
        let json = result.to_pretty_json().expect("runtime JSON");
        assert!(json.contains("\"reference\""));
        assert!(json.contains("\"rust\""));
        assert!(json.contains("\"status\""));
        assert!(json.contains("\"diagnostic_ids\""));
        assert!(json.contains("\"known_gap_id\""));
    }
}
