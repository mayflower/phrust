//! Runtime reference oracle for executing PHP files with the pinned PHP CLI.

use crate::normalize_output::normalize_runtime_stderr;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of trying to execute the PHP runtime reference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RuntimeReferenceRun {
    /// `REFERENCE_PHP` was not configured and no runtime comparison was made.
    Skipped { reason: String },
    /// PHP executed and produced process output.
    Completed(RuntimeReferenceOutput),
    /// `REFERENCE_PHP` was explicitly configured but unusable, or execution
    /// failed before PHP could run.
    Error { reason: String },
}

impl RuntimeReferenceRun {
    /// Returns true when the oracle could not run because it was not configured.
    #[must_use]
    pub const fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

/// Normalized process output from the PHP runtime reference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeReferenceOutput {
    /// PHP file that was executed.
    pub file: String,
    /// PHP binary used for execution.
    pub php_binary: String,
    /// Process exit code, if the process exited normally.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured raw stderr.
    pub stderr: String,
    /// Captured stderr after deterministic normalization.
    pub stderr_normalized: String,
    /// True when a future timeout-capable runner terminates the process.
    pub timeout: bool,
}

/// Resolves `REFERENCE_PHP` from the process environment.
#[must_use]
pub fn reference_php_from_env() -> Option<PathBuf> {
    env::var_os("REFERENCE_PHP").map(PathBuf::from)
}

/// Executes a PHP file with `REFERENCE_PHP`.
///
/// Missing `REFERENCE_PHP` is a clean skip. An explicitly configured but
/// unusable binary is an error so CI cannot silently compare against nothing.
#[must_use]
pub fn run_reference_php_file(file: impl AsRef<Path>) -> RuntimeReferenceRun {
    let file = file.as_ref();
    let Some(php_bin) = reference_php_from_env() else {
        return RuntimeReferenceRun::Skipped {
            reason: "REFERENCE_PHP is not set".to_owned(),
        };
    };

    if !php_bin.is_file() {
        return RuntimeReferenceRun::Error {
            reason: format!("REFERENCE_PHP is not a file: {}", php_bin.display()),
        };
    }

    let output = Command::new(&php_bin)
        .arg(file)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("NO_COLOR", "1")
        .env("PHP_INI_SCAN_DIR", "")
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return RuntimeReferenceRun::Error {
                reason: format!("failed to execute {}: {error}", php_bin.display()),
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stderr_normalized = normalize_runtime_stderr(&stderr, file, Some(&php_bin));

    RuntimeReferenceRun::Completed(RuntimeReferenceOutput {
        file: file.display().to_string(),
        php_binary: php_bin.display().to_string(),
        exit_code: output.status.code(),
        stdout,
        stderr,
        stderr_normalized,
        timeout: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{RuntimeReferenceRun, run_reference_php_file};
    use std::path::Path;

    #[test]
    fn runtime_reference_smoke_executes_or_skips() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture = manifest_dir
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .join("fixtures/runtime/valid/hello.php");
        let result = run_reference_php_file(fixture);
        match result {
            RuntimeReferenceRun::Skipped { reason } => {
                assert!(reason.contains("REFERENCE_PHP"));
            }
            RuntimeReferenceRun::Completed(output) => {
                assert_eq!(output.exit_code, Some(0));
                assert_eq!(output.stdout, "hello phase4\n");
                assert!(!output.timeout);
            }
            RuntimeReferenceRun::Error { reason } => panic!("{reason}"),
        }
    }
}
