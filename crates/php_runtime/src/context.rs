//! Deterministic runtime configuration for CLI fixture execution.

use crate::{ArrayKey, PhpArray, PhpString, Value};
use std::path::PathBuf;

/// Minimal ini-like runtime options carried by the VM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeIniOptions {
    /// Placeholder for PHP's `error_reporting` bitmask.
    pub error_reporting: ErrorReporting,
    /// Placeholder for display_errors-style behavior.
    pub display_errors: bool,
}

impl Default for RuntimeIniOptions {
    fn default() -> Self {
        Self {
            error_reporting: ErrorReporting::default(),
            display_errors: true,
        }
    }
}

/// Minimal error_reporting placeholder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorReporting {
    /// Stored mask. The Phase 4 VM does not interpret it yet.
    pub mask: i64,
}

impl Default for ErrorReporting {
    fn default() -> Self {
        Self { mask: -1 }
    }
}

/// Per-file or per-function strict_types metadata placeholder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrictTypesInfo {
    /// Stable file or function key.
    pub subject: String,
    /// Whether strict_types is enabled for the subject.
    pub enabled: bool,
}

/// Owned deterministic runtime context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeContext {
    /// Current working directory for future relative-path/runtime behavior.
    pub cwd: PathBuf,
    /// PHP CLI argv vector. Element 0 is the script path when configured.
    pub argv: Vec<String>,
    /// Controlled environment entries. Host env is never imported implicitly.
    pub env: Vec<(String, String)>,
    /// Minimal include path placeholder.
    pub include_path: Vec<PathBuf>,
    /// Minimal ini-like options.
    pub ini: RuntimeIniOptions,
    /// Strict-types metadata collected by future frontend integration.
    pub strict_types: Vec<StrictTypesInfo>,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self {
            cwd: PathBuf::from("."),
            argv: Vec::new(),
            env: Vec::new(),
            include_path: vec![PathBuf::from(".")],
            ini: RuntimeIniOptions::default(),
            strict_types: Vec::new(),
        }
    }
}

impl RuntimeContext {
    /// Creates a context for deterministic CLI fixture execution.
    #[must_use]
    pub fn controlled_cli(script_path: impl Into<String>, script_args: Vec<String>) -> Self {
        let mut argv = vec![script_path.into()];
        argv.extend(script_args);
        Self {
            argv,
            ..Self::default()
        }
    }

    /// Sets a deterministic current working directory.
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Sets a deterministic include path.
    #[must_use]
    pub fn with_include_path(mut self, include_path: Vec<PathBuf>) -> Self {
        self.include_path = include_path;
        self
    }

    /// Sets controlled environment entries in stable key order.
    #[must_use]
    pub fn with_env(mut self, mut env: Vec<(String, String)>) -> Self {
        env.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        self.env = env;
        self
    }

    /// Returns the `$argc` value derived from configured argv.
    #[must_use]
    pub fn argc(&self) -> i64 {
        self.argv.len() as i64
    }

    /// Returns a controlled global/superglobal value by local name.
    #[must_use]
    pub fn global_value(&self, name: &str) -> Option<Value> {
        match name {
            "argc" => Some(Value::Int(self.argc())),
            "argv" => Some(self.argv_array()),
            "_SERVER" => Some(Value::Array(self.server_array())),
            "_ENV" => Some(Value::Array(self.env_array())),
            "_GET" | "_POST" | "_COOKIE" | "_FILES" | "_REQUEST" | "GLOBALS" => {
                Some(Value::Array(PhpArray::new()))
            }
            _ => None,
        }
    }

    fn argv_array(&self) -> Value {
        Value::packed_array(
            self.argv
                .iter()
                .map(|value| Value::string(value.as_bytes().to_vec()))
                .collect(),
        )
    }

    fn server_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        array.insert(string_key("argc"), Value::Int(self.argc()));
        array.insert(string_key("argv"), self.argv_array());
        array
    }

    fn env_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for (key, value) in &self.env {
            array.insert(string_key(key), Value::string(value.as_bytes().to_vec()));
        }
        array
    }
}

fn string_key(value: &str) -> ArrayKey {
    ArrayKey::String(PhpString::from_test_str(value))
}

#[cfg(test)]
mod tests {
    use super::{RuntimeContext, StrictTypesInfo};
    use crate::{ArrayKey, PhpString, Value};

    #[test]
    fn context_defaults_are_deterministic() {
        let context = RuntimeContext::default();

        assert_eq!(context.cwd.to_string_lossy(), ".");
        assert!(context.argv.is_empty());
        assert!(context.env.is_empty());
        assert_eq!(context.include_path.len(), 1);
        assert_eq!(context.ini.error_reporting.mask, -1);
        assert!(context.ini.display_errors);
        assert!(context.strict_types.is_empty());
    }

    #[test]
    fn context_cli_argv_and_server_are_controlled() {
        let context = RuntimeContext::controlled_cli(
            "fixtures/runtime/valid/superglobals/argv.php",
            vec!["alpha".to_string(), "beta".to_string()],
        );

        assert_eq!(context.argc(), 3);
        assert_eq!(context.global_value("argc"), Some(Value::Int(3)));
        let Some(Value::Array(server)) = context.global_value("_SERVER") else {
            panic!("expected server array");
        };
        assert_eq!(
            server.get(&ArrayKey::String(PhpString::from_test_str("argc"))),
            Some(&Value::Int(3))
        );
        assert!(matches!(
            server.get(&ArrayKey::String(PhpString::from_test_str("argv"))),
            Some(Value::Array(_))
        ));
    }

    #[test]
    fn context_env_is_sorted_and_host_independent() {
        let context = RuntimeContext::default().with_env(vec![
            ("ZED".to_string(), "last".to_string()),
            ("ALPHA".to_string(), "first".to_string()),
        ]);

        assert_eq!(context.env[0].0, "ALPHA");
        assert_eq!(context.env[1].0, "ZED");
        assert!(context.global_value("_ENV").is_some());
        assert_eq!(
            RuntimeContext::default().env,
            Vec::<(String, String)>::new()
        );
    }

    #[test]
    fn context_strict_types_placeholder_is_explicit_metadata() {
        let context = RuntimeContext {
            strict_types: vec![StrictTypesInfo {
                subject: "fixture.php".to_string(),
                enabled: true,
            }],
            ..RuntimeContext::default()
        };

        assert_eq!(context.strict_types[0].subject, "fixture.php");
        assert!(context.strict_types[0].enabled);
    }
}
