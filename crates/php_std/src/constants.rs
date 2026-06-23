//! PHP 8.5.7 core and platform constants for Phase 6.

use php_runtime::{PhpString, Value};

use crate::ConstantValue;

/// Target PHP version.
pub const PHP_VERSION: &str = "8.5.7";
/// Target PHP version ID.
pub const PHP_VERSION_ID: i64 = 80507;
/// Target PHP major version.
pub const PHP_MAJOR_VERSION: i64 = 8;
/// Target PHP minor version.
pub const PHP_MINOR_VERSION: i64 = 5;
/// Target PHP release version.
pub const PHP_RELEASE_VERSION: i64 = 7;

/// Directory separator for the current build target.
#[cfg(windows)]
pub const DIRECTORY_SEPARATOR: &str = "\\";
/// Directory separator for the current build target.
#[cfg(not(windows))]
pub const DIRECTORY_SEPARATOR: &str = "/";

/// Path separator for the current build target.
#[cfg(windows)]
pub const PATH_SEPARATOR: &str = ";";
/// Path separator for the current build target.
#[cfg(not(windows))]
pub const PATH_SEPARATOR: &str = ":";

/// PHP end-of-line constant for this CLI engine.
pub const PHP_EOL: &str = "\n";

/// PHP OS string for the current build target.
#[cfg(target_os = "macos")]
pub const PHP_OS: &str = "Darwin";
/// PHP OS string for the current build target.
#[cfg(target_os = "linux")]
pub const PHP_OS: &str = "Linux";
/// PHP OS string for the current build target.
#[cfg(target_os = "windows")]
pub const PHP_OS: &str = "WINNT";
/// PHP OS string for other targets.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub const PHP_OS: &str = std::env::consts::OS;

/// PHP OS family string for the current build target.
#[cfg(target_os = "macos")]
pub const PHP_OS_FAMILY: &str = "Darwin";
/// PHP OS family string for the current build target.
#[cfg(target_os = "linux")]
pub const PHP_OS_FAMILY: &str = "Linux";
/// PHP OS family string for the current build target.
#[cfg(target_os = "windows")]
pub const PHP_OS_FAMILY: &str = "Windows";
/// PHP OS family string for other targets.
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub const PHP_OS_FAMILY: &str = "Unknown";

/// PHP `E_ERROR`.
pub const E_ERROR: i64 = 1;
/// PHP `E_WARNING`.
pub const E_WARNING: i64 = 2;
/// PHP `E_PARSE`.
pub const E_PARSE: i64 = 4;
/// PHP `E_NOTICE`.
pub const E_NOTICE: i64 = 8;
/// PHP `E_CORE_ERROR`.
pub const E_CORE_ERROR: i64 = 16;
/// PHP `E_CORE_WARNING`.
pub const E_CORE_WARNING: i64 = 32;
/// PHP `E_COMPILE_ERROR`.
pub const E_COMPILE_ERROR: i64 = 64;
/// PHP `E_COMPILE_WARNING`.
pub const E_COMPILE_WARNING: i64 = 128;
/// PHP `E_USER_ERROR`.
pub const E_USER_ERROR: i64 = 256;
/// PHP `E_USER_WARNING`.
pub const E_USER_WARNING: i64 = 512;
/// PHP `E_USER_NOTICE`.
pub const E_USER_NOTICE: i64 = 1024;
/// PHP `E_STRICT`.
pub const E_STRICT: i64 = 2048;
/// PHP `E_RECOVERABLE_ERROR`.
pub const E_RECOVERABLE_ERROR: i64 = 4096;
/// PHP `E_DEPRECATED`.
pub const E_DEPRECATED: i64 = 8192;
/// PHP `E_USER_DEPRECATED`.
pub const E_USER_DEPRECATED: i64 = 16384;
/// PHP 8.x `E_ALL`.
pub const E_ALL: i64 = 32767;

/// Converts registry constant metadata into a runtime value.
#[must_use]
pub fn constant_to_value(value: ConstantValue) -> Value {
    match value {
        ConstantValue::Bool(value) => Value::Bool(value),
        ConstantValue::Int(value) => Value::Int(value),
        ConstantValue::String(value) => Value::String(PhpString::from(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExtensionRegistry;

    #[test]
    fn version_constants_match_phase0_target() {
        assert_eq!(PHP_VERSION, "8.5.7");
        assert_eq!(PHP_VERSION_ID, 80507);
        assert_eq!(PHP_MAJOR_VERSION, 8);
        assert_eq!(PHP_MINOR_VERSION, 5);
        assert_eq!(PHP_RELEASE_VERSION, 7);
    }

    #[test]
    fn core_constants_are_registered_with_values() {
        let registry = ExtensionRegistry::phase6_infrastructure();
        let version_id = registry
            .enabled_constant("PHP_VERSION_ID")
            .expect("PHP_VERSION_ID");
        assert_eq!(version_id.value(), Some(ConstantValue::Int(80507)));

        let separator = registry
            .enabled_constant("DIRECTORY_SEPARATOR")
            .expect("DIRECTORY_SEPARATOR");
        assert_eq!(
            constant_to_value(separator.value().expect("separator value")),
            Value::String(PhpString::from(DIRECTORY_SEPARATOR))
        );
    }

    #[test]
    fn json_constants_are_enabled_with_json_extension() {
        let mut registry = ExtensionRegistry::phase6_infrastructure();
        assert_eq!(
            registry
                .enabled_constant("JSON_ERROR_NONE")
                .and_then(crate::ConstantDescriptor::value),
            Some(ConstantValue::Int(0))
        );

        registry.disable_extension("json").expect("disable json");
        assert!(registry.enabled_constant("JSON_ERROR_NONE").is_none());
        registry.enable_extension("json").expect("re-enable json");
        assert_eq!(
            registry
                .enabled_constant("JSON_ERROR_NONE")
                .and_then(crate::ConstantDescriptor::value),
            Some(ConstantValue::Int(0))
        );
    }
}
