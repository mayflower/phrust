use serde::{Deserialize, Serialize};

/// Normalized PHP semantic-frontend reference result.
///
/// Phase 3 uses `php -l` as the compile frontend acceptance oracle. The script
/// intentionally does not execute PHP source files.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhpFrontendResult {
    /// Source file path.
    pub file: String,
    /// True when PHP accepts the file at compile/lint time.
    pub ok: bool,
    /// Process exit code from `php -l`.
    pub exit_code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// PHP version string.
    pub php_version: String,
    /// Reference mode.
    pub mode: String,
    /// Oracle phase name.
    pub oracle: String,
    /// Normalized acceptance classification.
    pub classification: String,
    /// True when the reference process exceeded the harness timeout.
    #[serde(default)]
    pub timeout: bool,
}

impl PhpFrontendResult {
    /// Parses normalized PHP frontend JSON.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

/// Normalized Rust semantic frontend result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RustFrontendResult {
    /// Engine marker.
    pub engine: String,
    /// Target PHP version.
    pub target_php_version: String,
    /// True when parser and semantic frontend diagnostics contain no errors.
    pub ok: bool,
    /// Minimal module summary.
    pub module: serde_json::Value,
    /// Number of parser diagnostics.
    pub parser_diagnostics: usize,
    /// Semantic diagnostics as normalized JSON values.
    pub semantic_diagnostics: Vec<serde_json::Value>,
}

impl RustFrontendResult {
    /// Parses normalized Rust frontend JSON.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::{PhpFrontendResult, RustFrontendResult};

    #[test]
    fn parses_php_frontend_result_json() {
        let json = r#"{
            "file": "fixtures/semantic/valid/minimal.php",
            "ok": true,
            "exit_code": 0,
            "stdout": "No syntax errors detected",
            "stderr": "",
            "php_version": "8.5.7",
            "mode": "lint_compile_frontend",
            "oracle": "php-lint",
            "classification": "accepted"
        }"#;

        let result = PhpFrontendResult::from_json(json).expect("valid frontend oracle JSON");
        assert!(result.ok);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.classification, "accepted");
        assert_eq!(result.mode, "lint_compile_frontend");
        assert_eq!(result.oracle, "php-lint");
    }

    #[test]
    fn parses_rust_frontend_result_json() {
        let json = r#"{
            "engine": "phrust-frontend",
            "target_php_version": "8.5",
            "ok": true,
            "module": {"root_kind": "SOURCE_FILE", "source_bytes": 14},
            "parser_diagnostics": 0,
            "semantic_diagnostics": []
        }"#;

        let result = RustFrontendResult::from_json(json).expect("valid Rust frontend JSON");
        assert!(result.ok);
        assert_eq!(result.target_php_version, "8.5");
    }
}
