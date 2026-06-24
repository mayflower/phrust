//! Deterministic internal builtin registry for the runtime VM.

use crate::{
    ArrayKey, CallableValue, ClassEntry, ClassFlags, FilesystemCapabilities, NumericValue,
    ObjectRef, OutputBuffer, PcreCache, PhpArray, PhpString, ResourceTable, RuntimeDiagnostic,
    RuntimeSeverity, StreamWrapperRegistry, UnserializeOptions, Value, compare, datetime, pcre,
    serialize as serialize_value, to_bool, to_float, to_int, to_number, to_string,
    unserialize as unserialize_value, value::FloatValue,
};
use crate::{equal, identical};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use md5::{Digest, Md5};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use sha1::Sha1;
use std::collections::BTreeSet;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const JSON_ERROR_NONE: i64 = 0;
const JSON_ERROR_DEPTH: i64 = 1;
const JSON_ERROR_SYNTAX: i64 = 4;
const JSON_ERROR_UTF8: i64 = 5;
const JSON_OBJECT_AS_ARRAY: i64 = 1;
const JSON_PRETTY_PRINT: i64 = 128;
const JSON_UNESCAPED_SLASHES: i64 = 64;
const JSON_UNESCAPED_UNICODE: i64 = 256;
const JSON_PRESERVE_ZERO_FRACTION: i64 = 1024;
const JSON_THROW_ON_ERROR: i64 = 4_194_304;

/// Source location passed to internal builtins.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSourceSpan {
    /// Optional source file path.
    pub file: Option<String>,
    /// Start byte offset.
    pub start: u32,
    /// End byte offset.
    pub end: u32,
}

/// Mutable runtime services available to internal builtins.
pub struct BuiltinContext<'a> {
    output: &'a mut OutputBuffer,
    cwd: PathBuf,
    include_path: Vec<PathBuf>,
    default_timezone: String,
    filesystem: FilesystemCapabilities,
    resources: Option<&'a mut ResourceTable>,
    pcre_cache: PcreCache,
    preg_last_error: i64,
    preg_last_error_msg: String,
    json_last_error: i64,
    json_last_error_msg: String,
    diagnostics: Vec<RuntimeDiagnostic>,
}

impl<'a> BuiltinContext<'a> {
    /// Creates a runtime context backed by the VM output buffer.
    #[must_use]
    pub fn new(output: &'a mut OutputBuffer) -> Self {
        Self {
            output,
            cwd: PathBuf::from("."),
            include_path: vec![PathBuf::from(".")],
            default_timezone: datetime::DEFAULT_TIMEZONE.to_string(),
            filesystem: FilesystemCapabilities::none(),
            resources: None,
            pcre_cache: PcreCache::default(),
            preg_last_error: pcre::PREG_NO_ERROR,
            preg_last_error_msg: pcre::preg_error_message(pcre::PREG_NO_ERROR).to_string(),
            json_last_error: JSON_ERROR_NONE,
            json_last_error_msg: json_error_message(JSON_ERROR_NONE).to_string(),
            diagnostics: Vec::new(),
        }
    }

    /// Creates a runtime context with deterministic host capability policy.
    #[must_use]
    pub fn with_runtime(
        output: &'a mut OutputBuffer,
        cwd: impl Into<PathBuf>,
        filesystem: FilesystemCapabilities,
        resources: Option<&'a mut ResourceTable>,
    ) -> Self {
        Self {
            output,
            cwd: cwd.into(),
            include_path: vec![PathBuf::from(".")],
            default_timezone: datetime::DEFAULT_TIMEZONE.to_string(),
            filesystem,
            resources,
            pcre_cache: PcreCache::default(),
            preg_last_error: pcre::PREG_NO_ERROR,
            preg_last_error_msg: pcre::preg_error_message(pcre::PREG_NO_ERROR).to_string(),
            json_last_error: JSON_ERROR_NONE,
            json_last_error_msg: json_error_message(JSON_ERROR_NONE).to_string(),
            diagnostics: Vec::new(),
        }
    }

    /// Returns the output buffer.
    pub fn output(&mut self) -> &mut OutputBuffer {
        self.output
    }

    /// Emits a PHP display_errors-style warning into stdout and records a
    /// structured diagnostic for VM/report consumers.
    pub fn php_warning(
        &mut self,
        id: impl Into<String>,
        message: impl Into<String>,
        source_span: RuntimeSourceSpan,
    ) {
        let message = message.into();
        let line = source_span.start.to_string();
        let file = source_span.file.as_deref().unwrap_or("<unknown>");
        self.output.write_slices(&[
            b"\nWarning: ",
            message.as_bytes(),
            b" in ",
            file.as_bytes(),
            b" on line ",
            line.as_bytes(),
            b"\n",
        ]);
        self.diagnostics.push(RuntimeDiagnostic::new(
            id,
            RuntimeSeverity::Warning,
            message,
            source_span,
            Vec::new(),
            None,
        ));
    }

    /// Drains structured diagnostics emitted by builtins.
    pub fn take_diagnostics(&mut self) -> Vec<RuntimeDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Current working directory for path and filesystem builtins.
    #[must_use]
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Updates the request-local current working directory for filesystem builtins.
    pub fn set_cwd(&mut self, cwd: impl Into<PathBuf>) {
        self.cwd = cwd.into();
    }

    /// Include path entries used by stream include-path resolution.
    #[must_use]
    pub fn include_path(&self) -> &[PathBuf] {
        &self.include_path
    }

    /// Sets request-local include path entries.
    pub fn set_include_path(&mut self, include_path: Vec<PathBuf>) {
        self.include_path = include_path;
    }

    /// Current request-local default timezone.
    #[must_use]
    pub fn default_timezone(&self) -> &str {
        &self.default_timezone
    }

    /// Updates the request-local default timezone.
    pub fn set_default_timezone(&mut self, identifier: impl Into<String>) {
        self.default_timezone = identifier.into();
    }

    /// Filesystem capabilities for path and filesystem builtins.
    #[must_use]
    pub const fn filesystem_capabilities(&self) -> &FilesystemCapabilities {
        &self.filesystem
    }

    /// Request-local resource table for stream builtins.
    pub fn resources(&mut self) -> Option<&mut ResourceTable> {
        self.resources.as_deref_mut()
    }

    /// Request-local PCRE pattern cache.
    pub fn pcre_cache(&mut self) -> &mut PcreCache {
        &mut self.pcre_cache
    }

    /// Updates request-local PCRE last-error state.
    pub fn set_preg_last_error(&mut self, code: i64, message: impl Into<String>) {
        self.preg_last_error = code;
        self.preg_last_error_msg = message.into();
    }

    /// Clears request-local PCRE last-error state.
    pub fn clear_preg_last_error(&mut self) {
        self.set_preg_last_error(
            pcre::PREG_NO_ERROR,
            pcre::preg_error_message(pcre::PREG_NO_ERROR),
        );
    }

    /// Returns request-local PCRE last-error code and message.
    #[must_use]
    pub fn preg_last_error(&self) -> (i64, &str) {
        (self.preg_last_error, &self.preg_last_error_msg)
    }

    /// Updates request-local JSON last-error state.
    pub fn set_json_last_error(&mut self, code: i64) {
        self.json_last_error = code;
        self.json_last_error_msg = json_error_message(code).to_string();
    }

    /// Returns request-local JSON last-error code and message.
    #[must_use]
    pub fn json_last_error(&self) -> (i64, &str) {
        (self.json_last_error, &self.json_last_error_msg)
    }
}

/// Result returned by an internal builtin.
pub type BuiltinResult = Result<Value, BuiltinError>;

/// Internal builtin function signature.
pub type InternalFunction =
    fn(&mut BuiltinContext<'_>, Vec<Value>, RuntimeSourceSpan) -> BuiltinResult;

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

/// Registered builtin entry.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinEntry {
    name: &'static str,
    function: InternalFunction,
    compatibility: BuiltinCompatibility,
}

impl BuiltinEntry {
    /// Builtin name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Internal function pointer.
    #[must_use]
    pub const fn function(self) -> InternalFunction {
        self.function
    }

    /// Compatibility classification.
    #[must_use]
    pub const fn compatibility(self) -> BuiltinCompatibility {
        self.compatibility
    }
}

/// Whether a builtin is PHP-compatible or only for local fixtures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinCompatibility {
    /// PHP-compatible MVP builtin.
    Php,
    /// Internal test helper, not exposed as a PHP standard builtin.
    InternalTestHelper,
}

/// Deterministic builtin registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// Creates a builtin registry view.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns entries in stable sorted order.
    #[must_use]
    pub const fn entries(self) -> &'static [BuiltinEntry] {
        BUILTINS
    }

    /// Looks up a builtin by normalized name.
    #[must_use]
    pub fn get(self, name: &str) -> Option<BuiltinEntry> {
        BUILTINS.iter().copied().find(|entry| entry.name == name)
    }

    /// Returns true when a normalized name is registered.
    #[must_use]
    pub fn contains(self, name: &str) -> bool {
        self.get(name).is_some()
    }
}

const BUILTINS: &[BuiltinEntry] = &[
    BuiltinEntry {
        name: "abs",
        function: builtin_abs,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_all",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_any",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_chunk",
        function: builtin_array_chunk,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_column",
        function: builtin_array_column,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_filter",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_find",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_find_key",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_flip",
        function: builtin_array_flip,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_is_list",
        function: builtin_array_is_list,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_key_exists",
        function: builtin_array_key_exists,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_key_first",
        function: builtin_array_key_first,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_key_last",
        function: builtin_array_key_last,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_keys",
        function: builtin_array_keys,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_map",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_merge",
        function: builtin_array_merge,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_merge_recursive",
        function: builtin_array_merge_recursive,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_pad",
        function: builtin_array_pad,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_pop",
        function: builtin_array_pop,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_push",
        function: builtin_array_push,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_reduce",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_replace",
        function: builtin_array_replace,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_reverse",
        function: builtin_array_reverse,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_search",
        function: builtin_array_search,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_shift",
        function: builtin_array_shift,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_slice",
        function: builtin_array_slice,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_splice",
        function: builtin_array_splice,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_unshift",
        function: builtin_array_unshift,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_values",
        function: builtin_array_values,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "array_walk",
        function: builtin_array_callback_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "arsort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "asort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "base64_decode",
        function: builtin_base64_decode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "base64_encode",
        function: builtin_base64_encode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "basename",
        function: builtin_basename,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "bin2hex",
        function: builtin_bin2hex,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "boolval",
        function: builtin_boolval,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "call_user_func",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "call_user_func_array",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ceil",
        function: builtin_ceil,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "chdir",
        function: builtin_chdir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "chr",
        function: builtin_chr,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "class_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "clearstatcache",
        function: builtin_clearstatcache,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "closedir",
        function: builtin_closedir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "constant",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "copy",
        function: builtin_copy,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "count",
        function: builtin_count,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "crc32",
        function: builtin_crc32,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "date",
        function: builtin_date,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "date_default_timezone_get",
        function: builtin_date_default_timezone_get,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "date_default_timezone_set",
        function: builtin_date_default_timezone_set,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "defined",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "dirname",
        function: builtin_dirname,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "enum_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "error_reporting",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "exec",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "explode",
        function: builtin_explode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "extension_loaded",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fclose",
        function: builtin_fclose,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "feof",
        function: builtin_feof,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fflush",
        function: builtin_fflush,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fgetc",
        function: builtin_fgetc,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fgets",
        function: builtin_fgets,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "file_exists",
        function: builtin_file_exists,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "file_get_contents",
        function: builtin_file_get_contents,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "file_put_contents",
        function: builtin_file_put_contents,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "filemtime",
        function: builtin_filemtime,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "filesize",
        function: builtin_filesize,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "filetype",
        function: builtin_filetype,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "floatval",
        function: builtin_floatval,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "floor",
        function: builtin_floor,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "flush",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fmod",
        function: builtin_fmod,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fopen",
        function: builtin_fopen,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "forward_static_call",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fprintf",
        function: builtin_fprintf,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fread",
        function: builtin_fread,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fseek",
        function: builtin_fseek,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ftell",
        function: builtin_ftell,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "func_get_arg",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "func_get_args",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "func_num_args",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "function_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "fwrite",
        function: builtin_fwrite,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_cfg_var",
        function: builtin_config_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_class",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_class_methods",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_class_vars",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_current_user",
        function: builtin_environment_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_debug_type",
        function: builtin_get_debug_type,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_declared_classes",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_declared_interfaces",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_declared_traits",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_loaded_extensions",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_mangled_object_vars",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_object_vars",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_parent_class",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_resource_id",
        function: builtin_get_resource_id,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "get_resource_type",
        function: builtin_get_resource_type,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "getcwd",
        function: builtin_getcwd,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "getenv",
        function: builtin_environment_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "gettype",
        function: builtin_gettype,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "glob",
        function: builtin_glob,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "hash",
        function: builtin_hash,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "hash_hmac",
        function: builtin_hash_hmac,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "hex2bin",
        function: builtin_hex2bin,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "htmlentities",
        function: builtin_htmlentities,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "htmlspecialchars",
        function: builtin_htmlspecialchars,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "htmlspecialchars_decode",
        function: builtin_htmlspecialchars_decode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "http_build_query",
        function: builtin_http_build_query,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "implode",
        function: builtin_implode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "in_array",
        function: builtin_in_array,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ini_get",
        function: builtin_config_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ini_get_all",
        function: builtin_config_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ini_set",
        function: builtin_config_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "intdiv",
        function: builtin_intdiv,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "interface_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "intval",
        function: builtin_intval,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_array",
        function: builtin_is_array,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_bool",
        function: builtin_is_bool,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_countable",
        function: builtin_is_countable,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_dir",
        function: builtin_is_dir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_file",
        function: builtin_is_file,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_finite",
        function: builtin_is_finite,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_float",
        function: builtin_is_float,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_infinite",
        function: builtin_is_infinite,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_int",
        function: builtin_is_int,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_iterable",
        function: builtin_is_iterable,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_link",
        function: builtin_is_link,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_nan",
        function: builtin_is_nan,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_null",
        function: builtin_is_null,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_object",
        function: builtin_is_object,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_readable",
        function: builtin_is_readable,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_resource",
        function: builtin_is_resource,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_scalar",
        function: builtin_is_scalar,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_string",
        function: builtin_is_string,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_subclass_of",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "is_writable",
        function: builtin_is_writable,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "json_decode",
        function: builtin_json_decode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "json_encode",
        function: builtin_json_encode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "json_last_error",
        function: builtin_json_last_error,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "json_last_error_msg",
        function: builtin_json_last_error_msg,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "json_validate",
        function: builtin_json_validate,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "krsort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ksort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "lcfirst",
        function: builtin_lcfirst,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "lstat",
        function: builtin_lstat,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ltrim",
        function: builtin_ltrim,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "max",
        function: builtin_max,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "md5",
        function: builtin_md5,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "method_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "min",
        function: builtin_min,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "mkdir",
        function: builtin_mkdir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "natcasesort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "natsort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "number_format",
        function: builtin_number_format,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_end_clean",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_end_flush",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_get_clean",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_get_contents",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_get_length",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_get_level",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ob_start",
        function: builtin_output_buffering_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "opendir",
        function: builtin_opendir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ord",
        function: builtin_ord,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "passthru",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "pathinfo",
        function: builtin_pathinfo,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "pclose",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "php_sapi_name",
        function: builtin_environment_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "php_uname",
        function: builtin_environment_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "popen",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "pow",
        function: builtin_pow,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_grep",
        function: builtin_preg_grep,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_last_error",
        function: builtin_preg_last_error,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_last_error_msg",
        function: builtin_preg_last_error_msg,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_match",
        function: builtin_preg_match,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_match_all",
        function: builtin_preg_match_all,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_quote",
        function: builtin_preg_quote,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_replace",
        function: builtin_preg_replace,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_replace_callback",
        function: builtin_preg_replace_callback,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "preg_split",
        function: builtin_preg_split,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "print",
        function: builtin_print,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "print_r",
        function: builtin_print_r,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "printf",
        function: builtin_printf,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "proc_close",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "proc_get_status",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "proc_open",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "property_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "putenv",
        function: builtin_environment_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "random_bytes",
        function: builtin_random_bytes,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "random_int",
        function: builtin_random_int,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rawurldecode",
        function: builtin_rawurldecode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rawurlencode",
        function: builtin_rawurlencode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "readdir",
        function: builtin_readdir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "readfile",
        function: builtin_readfile,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "realpath",
        function: builtin_realpath,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rename",
        function: builtin_rename,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "restore_error_handler",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "restore_exception_handler",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rewind",
        function: builtin_rewind,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rewinddir",
        function: builtin_rewinddir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rmdir",
        function: builtin_rmdir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "round",
        function: builtin_round,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rsort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "rtrim",
        function: builtin_rtrim,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "scandir",
        function: builtin_scandir,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "serialize",
        function: builtin_serialize,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "set_error_handler",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "set_exception_handler",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "sha1",
        function: builtin_sha1,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "shell_exec",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "sizeof",
        function: builtin_count,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "sort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_autoload_call",
        function: builtin_spl_autoload_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_autoload_functions",
        function: builtin_spl_autoload_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_autoload_register",
        function: builtin_spl_autoload_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_autoload_unregister",
        function: builtin_spl_autoload_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_object_hash",
        function: builtin_spl_object_hash,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "spl_object_id",
        function: builtin_spl_object_id,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "sprintf",
        function: builtin_sprintf,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "sqrt",
        function: builtin_sqrt,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stat",
        function: builtin_stat,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_contains",
        function: builtin_str_contains,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_ends_with",
        function: builtin_str_ends_with,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_pad",
        function: builtin_str_pad,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_repeat",
        function: builtin_str_repeat,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_replace",
        function: builtin_str_replace,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "str_starts_with",
        function: builtin_str_starts_with,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strcasecmp",
        function: builtin_strcasecmp,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strcmp",
        function: builtin_strcmp,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_context_create",
        function: builtin_stream_context_create,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_context_get_options",
        function: builtin_stream_context_get_options,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_context_set_option",
        function: builtin_stream_context_set_option,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_copy_to_stream",
        function: builtin_stream_copy_to_stream,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_get_contents",
        function: builtin_stream_get_contents,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_get_meta_data",
        function: builtin_stream_get_meta_data,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_get_wrappers",
        function: builtin_stream_get_wrappers,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_is_local",
        function: builtin_stream_is_local,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_isatty",
        function: builtin_stream_isatty,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stream_resolve_include_path",
        function: builtin_stream_resolve_include_path,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "stripos",
        function: builtin_stripos,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strlen",
        function: builtin_strlen,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strncasecmp",
        function: builtin_strncasecmp,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strncmp",
        function: builtin_strncmp,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strpos",
        function: builtin_strpos,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strrev",
        function: builtin_strrev,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strrpos",
        function: builtin_strrpos,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strtolower",
        function: builtin_strtolower,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strtotime",
        function: builtin_strtotime,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strtoupper",
        function: builtin_strtoupper,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strtr",
        function: builtin_strtr,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "strval",
        function: builtin_strval,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "substr",
        function: builtin_substr,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "system",
        function: builtin_process_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "tempnam",
        function: builtin_tempnam,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "time",
        function: builtin_time,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "timezone_identifiers_list",
        function: builtin_timezone_identifiers_list,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "tmpfile",
        function: builtin_tmpfile,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "token_get_all",
        function: builtin_token_get_all,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "token_name",
        function: builtin_token_name,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "touch",
        function: builtin_touch,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "trait_exists",
        function: builtin_symbol_introspection_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "trigger_error",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "trim",
        function: builtin_trim,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "uasort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ucfirst",
        function: builtin_ucfirst,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "ucwords",
        function: builtin_ucwords,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "uksort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "unlink",
        function: builtin_unlink,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "unserialize",
        function: builtin_unserialize,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "urldecode",
        function: builtin_urldecode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "urlencode",
        function: builtin_urlencode,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "user_error",
        function: builtin_error_handling_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "usort",
        function: builtin_array_sort_requires_vm,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "var_dump",
        function: builtin_var_dump,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "var_export",
        function: builtin_var_export,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "version_compare",
        function: builtin_version_compare,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "vprintf",
        function: builtin_vprintf,
        compatibility: BuiltinCompatibility::Php,
    },
    BuiltinEntry {
        name: "vsprintf",
        function: builtin_vsprintf,
        compatibility: BuiltinCompatibility::Php,
    },
];

fn expect_arity(name: &str, args: &[Value], expected: usize) -> Result<(), BuiltinError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(arity_error(
        name,
        &format!("exactly {expected} argument(s)"),
    ))
}

fn arity_error(name: &str, expected: &str) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_ARITY",
        format!("builtin {name} expects {expected}"),
    )
}

fn builtin_strlen(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strlen", &args, 1)?;
    let value = string_arg("strlen", &args[0])?;
    Ok(Value::Int(value.len() as i64))
}

fn builtin_strtoupper(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strtoupper", &args, 1)?;
    match args.into_iter().next().expect("checked arity") {
        Value::String(value) => {
            let upper = value
                .as_bytes()
                .iter()
                .map(u8::to_ascii_uppercase)
                .collect::<Vec<_>>();
            Ok(Value::string(upper))
        }
        other => Err(type_error("strtoupper", "string", &other)),
    }
}

fn builtin_trim(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    trim_builtin("trim", args, true, true)
}

fn builtin_ltrim(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    trim_builtin("ltrim", args, true, false)
}

fn builtin_rtrim(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    trim_builtin("rtrim", args, false, true)
}

fn builtin_explode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin explode expects two or three argument(s)",
        ));
    }
    let separator = string_arg("explode", &args[0])?;
    if separator.is_empty() {
        return Err(value_error("explode", "separator cannot be empty"));
    }
    let string = string_arg("explode", &args[1])?;
    let limit = args
        .get(2)
        .map(|value| int_arg("explode", value))
        .transpose()?;
    let mut parts = split_bytes(string.as_bytes(), separator.as_bytes());
    match limit {
        Some(0) => parts.truncate(1),
        Some(limit) if limit > 0 => {
            parts = split_bytes_limited(string.as_bytes(), separator.as_bytes(), limit as usize)
        }
        Some(limit) if limit < 0 => {
            let drop = limit.unsigned_abs() as usize;
            if drop >= parts.len() {
                parts.clear();
            } else {
                parts.truncate(parts.len() - drop);
            }
        }
        _ => {}
    }
    Ok(Value::Array(crate::PhpArray::from_packed(
        parts.into_iter().map(Value::string).collect(),
    )))
}

fn builtin_implode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin implode expects one or two argument(s)",
        ));
    }
    let (separator, array) = if args.len() == 1 || matches!(deref_value(&args[0]), Value::Array(_))
    {
        (
            crate::PhpString::from_bytes(Vec::new()),
            array_arg("implode", &args[0])?,
        )
    } else {
        (
            string_arg("implode", &args[0])?,
            array_arg("implode", &args[1])?,
        )
    };
    let mut output = Vec::new();
    for (index, value) in array.iter().enumerate() {
        if index > 0 {
            output.extend_from_slice(separator.as_bytes());
        }
        output.extend_from_slice(value.as_bytes());
    }
    Ok(Value::string(output))
}

fn builtin_count(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(arity_error("count", "one or two argument(s)"));
    }
    let mode = args
        .get(1)
        .map(|value| int_arg("count", value))
        .transpose()?
        .unwrap_or(0);
    let count = match deref_value(&args[0]) {
        Value::Array(array) if mode == 1 => count_recursive(&array),
        Value::Array(array) => array.len(),
        Value::Object(object) => {
            match (
                object.get_property("__entries"),
                object.get_property("__storage"),
            ) {
                (Some(Value::Array(entries)), _) => entries.len(),
                (_, Some(Value::Array(entries))) => entries.len(),
                _ => return Err(type_error("count", "array or Countable", &args[0])),
            }
        }
        _ => return Err(type_error("count", "array or Countable", &args[0])),
    };
    Ok(Value::Int(count as i64))
}

fn builtin_array_key_exists(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_key_exists", &args, 2)?;
    let key = array_key_arg("array_key_exists", &args[0])?;
    let Value::Array(array) = deref_value(&args[1]) else {
        return Err(type_error("array_key_exists", "array", &args[1]));
    };
    Ok(Value::Bool(array.get(&key).is_some()))
}

fn builtin_array_keys(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=3).contains(&args.len()) {
        return Err(arity_error("array_keys", "one to three argument(s)"));
    }
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("array_keys", "array", &args[0]));
    };
    let strict = args
        .get(2)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("array_keys", message))?
        .unwrap_or(false);
    let mut keys = Vec::new();
    for (key, value) in array.iter() {
        if let Some(filter) = args.get(1)
            && !array_value_matches("array_keys", value, filter, strict)?
        {
            continue;
        }
        keys.push(array_key_to_value(key));
    }
    Ok(Value::packed_array(keys))
}

fn builtin_array_values(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_values", &args, 1)?;
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("array_values", "array", &args[0]));
    };
    Ok(Value::packed_array(
        array.iter().map(|(_, value)| value.clone()).collect(),
    ))
}

fn builtin_array_is_list(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_is_list", &args, 1)?;
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("array_is_list", "array", &args[0]));
    };
    Ok(Value::Bool(array.packed_elements().is_some()))
}

fn builtin_array_key_first(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_key_first", &args, 1)?;
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("array_key_first", "array", &args[0]));
    };
    Ok(array
        .iter()
        .next()
        .map_or(Value::Null, |(key, _)| array_key_to_value(key)))
}

fn builtin_array_key_last(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_key_last", &args, 1)?;
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("array_key_last", "array", &args[0]));
    };
    Ok(array
        .iter()
        .last()
        .map_or(Value::Null, |(key, _)| array_key_to_value(key)))
}

fn builtin_in_array(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("in_array", "two or three argument(s)"));
    }
    let Value::Array(array) = deref_value(&args[1]) else {
        return Err(type_error("in_array", "array", &args[1]));
    };
    let strict = args
        .get(2)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("in_array", message))?
        .unwrap_or(false);
    for (_, value) in array.iter() {
        if array_value_matches("in_array", &args[0], value, strict)? {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_array_search(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("array_search", "two or three argument(s)"));
    }
    let Value::Array(array) = deref_value(&args[1]) else {
        return Err(type_error("array_search", "array", &args[1]));
    };
    let strict = args
        .get(2)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("array_search", message))?
        .unwrap_or(false);
    for (key, value) in array.iter() {
        if array_value_matches("array_search", &args[0], value, strict)? {
            return Ok(array_key_to_value(key));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_array_column(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("array_column", "two or three argument(s)"));
    }
    let Value::Array(rows) = deref_value(&args[0]) else {
        return Err(type_error("array_column", "array", &args[0]));
    };
    let column_key = if matches!(deref_value(&args[1]), Value::Null) {
        None
    } else {
        Some(array_key_arg("array_column", &args[1])?)
    };
    let index_key = args
        .get(2)
        .filter(|value| !matches!(deref_value(value), Value::Null))
        .map(|value| array_key_arg("array_column", value))
        .transpose()?;
    let mut output = crate::PhpArray::new();
    for (_, row) in rows.iter() {
        let Value::Array(row) = deref_value(row) else {
            continue;
        };
        let Some(value) = column_key
            .as_ref()
            .map_or(Some(Value::Array(row.clone())), |key| row.get(key).cloned())
        else {
            continue;
        };
        if let Some(index_key) = &index_key
            && let Some(index_value) = row.get(index_key)
            && let Some(output_key) = ArrayKey::from_value_mvp(index_value)
        {
            output.insert(output_key, value);
            continue;
        }
        output.append(value);
    }
    Ok(Value::Array(output))
}

fn builtin_array_push(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error("array_push", "one or more argument(s)"));
    }
    let cell = array_reference_cell("array_push", &args[0])?;
    let mut array = array_from_reference_cell("array_push", &cell)?;
    for value in args.iter().skip(1) {
        array.append(value.clone());
    }
    let len = array.len() as i64;
    cell.set(Value::Array(array));
    Ok(Value::Int(len))
}

fn builtin_array_pop(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_pop", &args, 1)?;
    let cell = array_reference_cell("array_pop", &args[0])?;
    let array = array_from_reference_cell("array_pop", &cell)?;
    let mut entries = array_entries(&array);
    let value = entries.pop().map_or(Value::Null, |(_, value)| value);
    cell.set(Value::Array(array_from_entries_preserve(entries)));
    Ok(value)
}

fn builtin_array_shift(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_shift", &args, 1)?;
    let cell = array_reference_cell("array_shift", &args[0])?;
    let array = array_from_reference_cell("array_shift", &cell)?;
    let mut entries = array_entries(&array);
    let value = if entries.is_empty() {
        Value::Null
    } else {
        entries.remove(0).1
    };
    cell.set(Value::Array(array_from_entries_reindex_ints(entries)));
    Ok(value)
}

fn builtin_array_unshift(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error("array_unshift", "one or more argument(s)"));
    }
    let cell = array_reference_cell("array_unshift", &args[0])?;
    let array = array_from_reference_cell("array_unshift", &cell)?;
    let mut output = crate::PhpArray::new();
    for value in args.iter().skip(1) {
        output.append(value.clone());
    }
    for (key, value) in array.iter() {
        match key {
            ArrayKey::Int(_) => {
                output.append(value.clone());
            }
            ArrayKey::String(key) => {
                output.insert(ArrayKey::String(key.clone()), value.clone());
            }
        }
    }
    let len = output.len() as i64;
    cell.set(Value::Array(output));
    Ok(Value::Int(len))
}

fn builtin_array_slice(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=4).contains(&args.len()) {
        return Err(arity_error("array_slice", "two to four argument(s)"));
    }
    let array = array_value_arg("array_slice", &args[0])?;
    let offset = int_arg("array_slice", &args[1])?;
    let length = args
        .get(2)
        .filter(|value| !matches!(deref_value(value), Value::Null))
        .map(|value| int_arg("array_slice", value))
        .transpose()?;
    let preserve_keys = args
        .get(3)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("array_slice", message))?
        .unwrap_or(false);
    let entries = slice_entries(array_entries(&array), offset, length);
    Ok(Value::Array(array_from_entries_for_slice(
        entries,
        preserve_keys,
    )))
}

fn builtin_array_splice(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=4).contains(&args.len()) {
        return Err(arity_error("array_splice", "two to four argument(s)"));
    }
    let cell = array_reference_cell("array_splice", &args[0])?;
    let array = array_from_reference_cell("array_splice", &cell)?;
    let entries = array_entries(&array);
    let offset = int_arg("array_splice", &args[1])?;
    let start = normalize_slice_start(entries.len(), offset);
    let delete_len = args
        .get(2)
        .filter(|value| !matches!(deref_value(value), Value::Null))
        .map(|value| splice_length(entries.len(), start, int_arg("array_splice", value)?))
        .transpose()?
        .unwrap_or(entries.len().saturating_sub(start));
    let replacement = args
        .get(3)
        .map(|value| splice_replacement_values("array_splice", value))
        .transpose()?
        .unwrap_or_default();

    let removed = entries[start..start + delete_len].to_vec();
    let mut result_values = Vec::new();
    result_values.extend(entries[..start].iter().map(|(_, value)| value.clone()));
    result_values.extend(replacement);
    result_values.extend(
        entries[start + delete_len..]
            .iter()
            .map(|(_, value)| value.clone()),
    );
    cell.set(Value::packed_array(result_values));
    Ok(Value::Array(array_from_entries_reindex_ints(removed)))
}

fn builtin_array_merge(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    let mut output = crate::PhpArray::new();
    for arg in &args {
        let array = array_value_arg("array_merge", arg)?;
        for (key, value) in array.iter() {
            match key {
                ArrayKey::Int(_) => {
                    output.append(value.clone());
                }
                ArrayKey::String(key) => {
                    output.insert(ArrayKey::String(key.clone()), value.clone());
                }
            }
        }
    }
    Ok(Value::Array(output))
}

fn builtin_array_merge_recursive(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    let mut output = crate::PhpArray::new();
    for arg in &args {
        let array = array_value_arg("array_merge_recursive", arg)?;
        merge_recursive_into(&mut output, &array);
    }
    Ok(Value::Array(output))
}

fn builtin_array_replace(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error("array_replace", "one or more argument(s)"));
    }
    let mut output = array_value_arg("array_replace", &args[0])?;
    for arg in args.iter().skip(1) {
        let array = array_value_arg("array_replace", arg)?;
        for (key, value) in array.iter() {
            output.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Array(output))
}

fn builtin_array_reverse(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(arity_error("array_reverse", "one or two argument(s)"));
    }
    let array = array_value_arg("array_reverse", &args[0])?;
    let preserve_keys = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("array_reverse", message))?
        .unwrap_or(false);
    let mut entries = array_entries(&array);
    entries.reverse();
    Ok(Value::Array(array_from_entries_for_slice(
        entries,
        preserve_keys,
    )))
}

fn builtin_array_pad(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_pad", &args, 3)?;
    let array = array_value_arg("array_pad", &args[0])?;
    let target = int_arg("array_pad", &args[1])?;
    let pad_value = args[2].clone();
    let mut values = array
        .iter()
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    let target_len = target.unsigned_abs() as usize;
    if target_len > values.len() {
        let pad_count = target_len - values.len();
        if target < 0 {
            let mut padded = vec![pad_value; pad_count];
            padded.extend(values);
            values = padded;
        } else {
            values.extend(std::iter::repeat_n(pad_value, pad_count));
        }
    }
    Ok(Value::packed_array(values))
}

fn builtin_array_chunk(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("array_chunk", "two or three argument(s)"));
    }
    let array = array_value_arg("array_chunk", &args[0])?;
    let length = int_arg("array_chunk", &args[1])?;
    if length <= 0 {
        return Err(value_error(
            "array_chunk",
            "length must be greater than or equal to 1",
        ));
    }
    let preserve_keys = args
        .get(2)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("array_chunk", message))?
        .unwrap_or(false);
    let entries = array_entries(&array);
    let mut chunks = Vec::new();
    for chunk in entries.chunks(length as usize) {
        let chunk_entries = chunk.to_vec();
        let chunk_array = if preserve_keys {
            array_from_entries_preserve(chunk_entries)
        } else {
            array_from_entries_for_slice(chunk_entries, false)
        };
        chunks.push(Value::Array(chunk_array));
    }
    Ok(Value::packed_array(chunks))
}

fn builtin_array_flip(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("array_flip", &args, 1)?;
    let array = array_value_arg("array_flip", &args[0])?;
    let mut output = crate::PhpArray::new();
    for (key, value) in array.iter() {
        let Some(output_key) = ArrayKey::from_value_mvp(value) else {
            context.php_warning(
                "E_PHP_RUNTIME_ARRAY_FLIP_ENTRY_SKIPPED",
                "array_flip(): Can only flip string and integer values, entry skipped",
                span.clone(),
            );
            continue;
        };
        output.insert(output_key, array_key_to_value(key));
    }
    Ok(Value::Array(output))
}

fn builtin_array_callback_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_CALLABLE_CONTEXT_REQUIRED",
        "array callback builtins require VM callable dispatch",
    ))
}

fn builtin_array_sort_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_CALLABLE_CONTEXT_REQUIRED",
        "array sort builtins require VM reference and callable dispatch",
    ))
}

fn builtin_symbol_introspection_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_SYMBOL_CONTEXT_REQUIRED",
        "symbol introspection builtins require VM symbol tables and autoload state",
    ))
}

fn builtin_config_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_CONFIG_CONTEXT_REQUIRED",
        "configuration builtins require VM request-local INI state",
    ))
}

fn builtin_error_handling_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_ERROR_CONTEXT_REQUIRED",
        "error handling builtins require VM handler stacks and request-local INI state",
    ))
}

fn builtin_output_buffering_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_OUTPUT_BUFFER_CONTEXT_REQUIRED",
        "output buffering builtins require VM output buffer stack state",
    ))
}

fn builtin_spl_autoload_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_SPL_AUTOLOAD_CONTEXT_REQUIRED",
        "SPL autoload builtins require VM autoload stack state",
    ))
}

fn builtin_environment_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_ENVIRONMENT_CONTEXT_REQUIRED",
        "environment builtins require VM request context state",
    ))
}

fn builtin_process_requires_vm(
    _context: &mut BuiltinContext<'_>,
    _args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_PROCESS_CONTEXT_REQUIRED",
        "process builtins require VM process capability policy",
    ))
}

fn builtin_abs(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("abs", &args, 1)?;
    Ok(
        match to_number(&args[0]).map_err(|message| conversion_error("abs", message))? {
            NumericValue::Int(value) => value
                .checked_abs()
                .map(Value::Int)
                .unwrap_or_else(|| Value::float((value as f64).abs())),
            NumericValue::Float(value) => Value::float(value.abs()),
        },
    )
}

fn builtin_min(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    min_max_builtin("min", args, false)
}

fn builtin_max(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    min_max_builtin("max", args, true)
}

fn builtin_round(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=3).contains(&args.len()) {
        return Err(arity_error("round", "one to three argument(s)"));
    }
    let value = numeric_f64_arg("round", &args[0])?;
    let precision = args
        .get(1)
        .map(|value| int_arg("round", value))
        .transpose()?
        .unwrap_or(0);
    let factor = 10_f64.powi(precision as i32);
    Ok(Value::float((value * factor).round() / factor))
}

fn builtin_floor(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("floor", &args, 1)?;
    Ok(Value::float(numeric_f64_arg("floor", &args[0])?.floor()))
}

fn builtin_ceil(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("ceil", &args, 1)?;
    Ok(Value::float(numeric_f64_arg("ceil", &args[0])?.ceil()))
}

fn builtin_sqrt(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("sqrt", &args, 1)?;
    Ok(Value::float(numeric_f64_arg("sqrt", &args[0])?.sqrt()))
}

fn builtin_pow(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("pow", &args, 2)?;
    if let (Ok(NumericValue::Int(base)), Ok(NumericValue::Int(exponent))) =
        (to_number(&args[0]), to_number(&args[1]))
        && let Ok(exponent) = u32::try_from(exponent)
        && let Some(value) = base.checked_pow(exponent)
    {
        return Ok(Value::Int(value));
    }
    Ok(Value::float(
        numeric_f64_arg("pow", &args[0])?.powf(numeric_f64_arg("pow", &args[1])?),
    ))
}

fn builtin_intdiv(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("intdiv", &args, 2)?;
    let dividend = int_arg("intdiv", &args[0])?;
    let divisor = int_arg("intdiv", &args[1])?;
    if divisor == 0 {
        return Err(value_error("intdiv", "division by zero"));
    }
    if dividend == i64::MIN && divisor == -1 {
        return Err(value_error("intdiv", "division overflows"));
    }
    Ok(Value::Int(dividend / divisor))
}

fn builtin_fmod(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fmod", &args, 2)?;
    let dividend = numeric_f64_arg("fmod", &args[0])?;
    let divisor = numeric_f64_arg("fmod", &args[1])?;
    if divisor == 0.0 {
        return Err(value_error("fmod", "division by zero"));
    }
    Ok(Value::float(dividend % divisor))
}

fn builtin_is_finite(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_finite", &args, 1)?;
    Ok(Value::Bool(
        numeric_f64_arg("is_finite", &args[0])?.is_finite(),
    ))
}

fn builtin_is_infinite(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_infinite", &args, 1)?;
    Ok(Value::Bool(
        numeric_f64_arg("is_infinite", &args[0])?.is_infinite(),
    ))
}

fn builtin_is_nan(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_nan", &args, 1)?;
    Ok(Value::Bool(numeric_f64_arg("is_nan", &args[0])?.is_nan()))
}

fn builtin_number_format(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=4).contains(&args.len()) {
        return Err(arity_error("number_format", "one to four argument(s)"));
    }
    let value = numeric_f64_arg("number_format", &args[0])?;
    let decimals = args
        .get(1)
        .map(|value| int_arg("number_format", value))
        .transpose()?
        .unwrap_or(0)
        .max(0) as usize;
    let decimal_separator = args
        .get(2)
        .map(|value| string_arg("number_format", value))
        .transpose()?
        .unwrap_or_else(|| crate::PhpString::from_test_str("."));
    let thousands_separator = args
        .get(3)
        .map(|value| string_arg("number_format", value))
        .transpose()?
        .unwrap_or_else(|| crate::PhpString::from_test_str(","));
    let rounded = format!("{:.*}", decimals, value.abs());
    let (integer, fraction) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let mut grouped = group_decimal_integer(integer, &thousands_separator.to_string_lossy());
    if decimals > 0 {
        grouped.push_str(&decimal_separator.to_string_lossy());
        grouped.push_str(fraction);
    }
    if value.is_sign_negative() && grouped != "0" {
        grouped.insert(0, '-');
    }
    Ok(Value::string(grouped))
}

fn builtin_str_replace(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(3..=4).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin str_replace expects three or four argument(s)",
        ));
    }
    let search = string_list_arg("str_replace", &args[0])?;
    let replace = string_list_arg("str_replace", &args[1])?;
    let mut count = 0_i64;
    let result = replace_subject(&args[2], &search, &replace, &mut count)?;
    if let Some(Value::Reference(cell)) = args.get(3) {
        cell.set(Value::Int(count));
    }
    Ok(result)
}

fn builtin_strtr(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() == 2 {
        let mut subject = string_arg("strtr", &args[0])?.into_bytes();
        let Value::Array(map) = deref_value(&args[1]) else {
            return Err(type_error("strtr", "array", &args[1]));
        };
        let mut replacements = map
            .iter()
            .map(|(key, value)| {
                let key = match key {
                    ArrayKey::Int(index) => index.to_string().into_bytes(),
                    ArrayKey::String(key) => key.as_bytes().to_vec(),
                };
                Ok((key, string_arg("strtr", value)?.into_bytes()))
            })
            .collect::<Result<Vec<_>, BuiltinError>>()?;
        replacements.sort_by_key(|(key, _)| std::cmp::Reverse(key.len()));
        subject = replace_map(&subject, &replacements);
        return Ok(Value::string(subject));
    }
    expect_arity("strtr", &args, 3)?;
    let mut subject = string_arg("strtr", &args[0])?.into_bytes();
    let from = string_arg("strtr", &args[1])?;
    let to = string_arg("strtr", &args[2])?;
    for byte in &mut subject {
        if let Some(index) = from.as_bytes().iter().position(|from| from == byte)
            && let Some(replacement) = to.as_bytes().get(index)
        {
            *byte = *replacement;
        }
    }
    Ok(Value::string(subject))
}

fn builtin_strtolower(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strtolower", &args, 1)?;
    Ok(Value::string(
        string_arg("strtolower", &args[0])?
            .as_bytes()
            .iter()
            .map(u8::to_ascii_lowercase)
            .collect::<Vec<_>>(),
    ))
}

fn builtin_ucfirst(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("ucfirst", &args, 1)?;
    Ok(Value::string(change_first_ascii(
        string_arg("ucfirst", &args[0])?,
        true,
    )))
}

fn builtin_lcfirst(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("lcfirst", &args, 1)?;
    Ok(Value::string(change_first_ascii(
        string_arg("lcfirst", &args[0])?,
        false,
    )))
}

fn builtin_ucwords(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin ucwords expects one or two argument(s)",
        ));
    }
    let mut bytes = string_arg("ucwords", &args[0])?.into_bytes();
    let delimiters = args
        .get(1)
        .map(|value| string_arg("ucwords", value))
        .transpose()?;
    let delimiters = delimiters
        .as_ref()
        .map_or(b" \t\r\n\x0c\x0b".as_slice(), crate::PhpString::as_bytes);
    let mut at_word_start = true;
    for byte in &mut bytes {
        if delimiters.contains(byte) {
            at_word_start = true;
        } else if at_word_start {
            *byte = byte.to_ascii_uppercase();
            at_word_start = false;
        }
    }
    Ok(Value::string(bytes))
}

fn builtin_str_repeat(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("str_repeat", &args, 2)?;
    let string = string_arg("str_repeat", &args[0])?;
    let count = int_arg("str_repeat", &args[1])?;
    if count < 0 {
        return Err(value_error(
            "str_repeat",
            "count must be greater than or equal to 0",
        ));
    }
    Ok(Value::string(string.as_bytes().repeat(count as usize)))
}

fn builtin_str_pad(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=4).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin str_pad expects two to four argument(s)",
        ));
    }
    let input = string_arg("str_pad", &args[0])?;
    let length = int_arg("str_pad", &args[1])?;
    if length < 0 {
        return Err(value_error(
            "str_pad",
            "length must be greater than or equal to 0",
        ));
    }
    let pad = args
        .get(2)
        .map(|value| string_arg("str_pad", value))
        .transpose()?
        .unwrap_or_else(|| crate::PhpString::from_test_str(" "));
    if pad.is_empty() {
        return Err(value_error("str_pad", "pad string cannot be empty"));
    }
    let pad_type = args
        .get(3)
        .map(|value| int_arg("str_pad", value))
        .transpose()?
        .unwrap_or(1);
    let target = length as usize;
    if input.len() >= target {
        return Ok(Value::String(input));
    }
    let needed = target - input.len();
    let (left, right) = match pad_type {
        0 => (needed, 0),
        2 => (needed / 2, needed - (needed / 2)),
        _ => (0, needed),
    };
    let mut output = repeat_pad(pad.as_bytes(), left);
    output.extend_from_slice(input.as_bytes());
    output.extend_from_slice(&repeat_pad(pad.as_bytes(), right));
    Ok(Value::string(output))
}

fn builtin_strrev(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strrev", &args, 1)?;
    let mut bytes = string_arg("strrev", &args[0])?.into_bytes();
    bytes.reverse();
    Ok(Value::string(bytes))
}

fn builtin_bin2hex(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("bin2hex", &args, 1)?;
    Ok(Value::string(hex_encode(
        string_arg("bin2hex", &args[0])?.as_bytes(),
    )))
}

fn builtin_hex2bin(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("hex2bin", &args, 1)?;
    let input = string_arg("hex2bin", &args[0])?;
    if !input.as_bytes().len().is_multiple_of(2) {
        context.php_warning(
            "E_PHP_RUNTIME_HEX2BIN_ODD_LENGTH",
            "hex2bin(): Hexadecimal input string must have an even length",
            span,
        );
        return Ok(Value::Bool(false));
    }
    if input
        .as_bytes()
        .iter()
        .any(|byte| hex_nibble(*byte).is_none())
    {
        context.php_warning(
            "E_PHP_RUNTIME_HEX2BIN_INVALID_HEX",
            "hex2bin(): Input string must be hexadecimal string",
            span,
        );
        return Ok(Value::Bool(false));
    }
    hex_decode(input.as_bytes()).map_or(Ok(Value::Bool(false)), |bytes| Ok(Value::string(bytes)))
}

fn builtin_ord(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("ord", &args, 1)?;
    let input = string_arg("ord", &args[0])?;
    input
        .as_bytes()
        .first()
        .copied()
        .map(|byte| Value::Int(i64::from(byte)))
        .ok_or_else(|| value_error("ord", "string must not be empty"))
}

fn builtin_chr(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("chr", &args, 1)?;
    let value = int_arg("chr", &args[0])?.rem_euclid(256) as u8;
    Ok(Value::string(vec![value]))
}

fn builtin_md5(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin md5 expects one or two argument(s)",
        ));
    }
    let input = string_arg("md5", &args[0])?;
    let raw = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("md5", message))?
        .unwrap_or(false);
    let digest = Md5::digest(input.as_bytes());
    Ok(if raw {
        Value::string(digest.to_vec())
    } else {
        Value::string(hex_encode(&digest))
    })
}

fn builtin_sha1(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin sha1 expects one or two argument(s)",
        ));
    }
    let input = string_arg("sha1", &args[0])?;
    let raw = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("sha1", message))?
        .unwrap_or(false);
    let digest = Sha1::digest(input.as_bytes());
    Ok(if raw {
        Value::string(digest.to_vec())
    } else {
        Value::string(hex_encode(&digest))
    })
}

fn builtin_crc32(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("crc32", &args, 1)?;
    let input = string_arg("crc32", &args[0])?;
    Ok(Value::Int(i64::from(crc32fast::hash(input.as_bytes()))))
}

fn builtin_hash(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("hash", "two or three argument(s)"));
    }
    let algorithm = string_arg("hash", &args[0])?.to_string_lossy();
    let input = string_arg("hash", &args[1])?;
    let binary = args
        .get(2)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("hash", message))?
        .unwrap_or(false);
    let digest = hash_digest_bytes("hash", &algorithm, input.as_bytes())?;
    Ok(if binary {
        Value::string(digest)
    } else {
        Value::string(hex_encode(&digest))
    })
}

fn builtin_hash_hmac(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(3..=4).contains(&args.len()) {
        return Err(arity_error("hash_hmac", "three or four argument(s)"));
    }
    let algorithm = string_arg("hash_hmac", &args[0])?.to_string_lossy();
    let input = string_arg("hash_hmac", &args[1])?;
    let key = string_arg("hash_hmac", &args[2])?;
    let binary = args
        .get(3)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("hash_hmac", message))?
        .unwrap_or(false);
    let digest = hmac_digest_bytes("hash_hmac", &algorithm, key.as_bytes(), input.as_bytes())?;
    Ok(if binary {
        Value::string(digest)
    } else {
        Value::string(hex_encode(&digest))
    })
}

fn builtin_random_bytes(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("random_bytes", &args, 1)?;
    let length = int_arg("random_bytes", &args[0])?;
    if length < 1 {
        return Err(value_error("random_bytes", "length must be greater than 0"));
    }
    let mut bytes = vec![0; length as usize];
    getrandom::getrandom(&mut bytes).map_err(|error| {
        BuiltinError::new(
            "E_PHP_RUNTIME_RANDOM_FAILURE",
            format!("random_bytes(): failed to read random bytes: {error}"),
        )
    })?;
    Ok(Value::string(bytes))
}

fn builtin_random_int(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("random_int", &args, 2)?;
    let min = int_arg("random_int", &args[0])?;
    let max = int_arg("random_int", &args[1])?;
    if max < min {
        return Err(value_error(
            "random_int",
            "max must be greater than or equal to min",
        ));
    }
    let range = (i128::from(max) - i128::from(min) + 1) as u128;
    let zone = u128::MAX - (u128::MAX % range);
    loop {
        let mut bytes = [0; 16];
        getrandom::getrandom(&mut bytes).map_err(|error| {
            BuiltinError::new(
                "E_PHP_RUNTIME_RANDOM_FAILURE",
                format!("random_int(): failed to read random bytes: {error}"),
            )
        })?;
        let sample = u128::from_le_bytes(bytes);
        if sample < zone {
            let offset = (sample % range) as i128;
            return Ok(Value::Int((i128::from(min) + offset) as i64));
        }
    }
}

fn builtin_base64_encode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("base64_encode", &args, 1)?;
    Ok(Value::string(
        BASE64_STANDARD
            .encode(string_arg("base64_encode", &args[0])?.as_bytes())
            .into_bytes(),
    ))
}

fn builtin_base64_decode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin base64_decode expects one or two argument(s)",
        ));
    }
    let input = string_arg("base64_decode", &args[0])?;
    let strict = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("base64_decode", message))?
        .unwrap_or(false);
    let source = if strict {
        input.as_bytes().to_vec()
    } else {
        input
            .as_bytes()
            .iter()
            .copied()
            .filter(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
            .collect()
    };
    match BASE64_STANDARD.decode(source) {
        Ok(bytes) => Ok(Value::string(bytes)),
        Err(_) => Ok(Value::Bool(false)),
    }
}

fn builtin_htmlspecialchars(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=4).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin htmlspecialchars expects one to four argument(s)",
        ));
    }
    Ok(Value::string(html_escape(
        string_arg("htmlspecialchars", &args[0])?.as_bytes(),
    )))
}

fn builtin_htmlentities(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    builtin_htmlspecialchars(context, args, span)
}

fn builtin_htmlspecialchars_decode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin htmlspecialchars_decode expects one or two argument(s)",
        ));
    }
    Ok(Value::string(html_decode(
        &string_arg("htmlspecialchars_decode", &args[0])?.to_string_lossy(),
    )))
}

fn builtin_urlencode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("urlencode", &args, 1)?;
    Ok(Value::string(url_encode(
        string_arg("urlencode", &args[0])?.as_bytes(),
        false,
    )))
}

fn builtin_rawurlencode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("rawurlencode", &args, 1)?;
    Ok(Value::string(url_encode(
        string_arg("rawurlencode", &args[0])?.as_bytes(),
        true,
    )))
}

fn builtin_urldecode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("urldecode", &args, 1)?;
    Ok(Value::string(url_decode(
        string_arg("urldecode", &args[0])?.as_bytes(),
        false,
    )))
}

fn builtin_rawurldecode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("rawurldecode", &args, 1)?;
    Ok(Value::string(url_decode(
        string_arg("rawurldecode", &args[0])?.as_bytes(),
        true,
    )))
}

fn builtin_http_build_query(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=4).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin http_build_query expects one to four argument(s)",
        ));
    }
    let Value::Array(array) = deref_value(&args[0]) else {
        return Err(type_error("http_build_query", "array", &args[0]));
    };
    let mut pairs = Vec::new();
    build_query_pairs(None, &Value::Array(array), &mut pairs)?;
    Ok(Value::string(pairs.join("&").into_bytes()))
}

fn builtin_substr(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin substr expects two or three argument(s)",
        ));
    }
    let string = string_arg("substr", &args[0])?;
    let offset = int_arg("substr", &args[1])?;
    let length = args
        .get(2)
        .map(|value| int_arg("substr", value))
        .transpose()?;
    let bytes = string.as_bytes();
    let start = normalize_offset(bytes.len(), offset);
    let end = match length {
        None => bytes.len(),
        Some(length) if length >= 0 => start.saturating_add(length as usize).min(bytes.len()),
        Some(length) => bytes.len().saturating_sub(length.unsigned_abs() as usize),
    };
    if start >= bytes.len() || end < start {
        return Ok(Value::string(Vec::new()));
    }
    Ok(Value::string(bytes[start..end].to_vec()))
}

fn builtin_strpos(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    string_position("strpos", args, false, false)
}

fn builtin_stripos(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    string_position("stripos", args, true, false)
}

fn builtin_strrpos(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    string_position("strrpos", args, false, true)
}

fn builtin_str_contains(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("str_contains", &args, 2)?;
    let haystack = string_arg("str_contains", &args[0])?;
    let needle = string_arg("str_contains", &args[1])?;
    Ok(Value::Bool(
        find_bytes(haystack.as_bytes(), needle.as_bytes()).is_some(),
    ))
}

fn builtin_str_starts_with(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("str_starts_with", &args, 2)?;
    let haystack = string_arg("str_starts_with", &args[0])?;
    let needle = string_arg("str_starts_with", &args[1])?;
    Ok(Value::Bool(
        haystack.as_bytes().starts_with(needle.as_bytes()),
    ))
}

fn builtin_str_ends_with(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("str_ends_with", &args, 2)?;
    let haystack = string_arg("str_ends_with", &args[0])?;
    let needle = string_arg("str_ends_with", &args[1])?;
    Ok(Value::Bool(
        haystack.as_bytes().ends_with(needle.as_bytes()),
    ))
}

fn builtin_strcmp(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strcmp", &args, 2)?;
    compare_strings("strcmp", &args, false, None)
}

fn builtin_strncmp(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strncmp", &args, 3)?;
    let length = int_arg("strncmp", &args[2])?;
    if length < 0 {
        return Err(value_error(
            "strncmp",
            "length must be greater than or equal to 0",
        ));
    }
    compare_strings("strncmp", &args, false, Some(length as usize))
}

fn builtin_strcasecmp(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strcasecmp", &args, 2)?;
    compare_strings("strcasecmp", &args, true, None)
}

fn builtin_strncasecmp(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strncasecmp", &args, 3)?;
    let length = int_arg("strncasecmp", &args[2])?;
    if length < 0 {
        return Err(value_error(
            "strncasecmp",
            "length must be greater than or equal to 0",
        ));
    }
    compare_strings("strncasecmp", &args, true, Some(length as usize))
}

fn builtin_version_compare(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("version_compare", "2 or 3 argument(s)"));
    }

    let left = string_arg("version_compare", &args[0])?.to_string_lossy();
    let right = string_arg("version_compare", &args[1])?.to_string_lossy();
    let comparison = compare_versions(&left, &right);
    if let Some(operator) = args.get(2) {
        let operator = string_arg("version_compare", operator)?.to_string_lossy();
        return Ok(Value::Bool(version_operator_matches(
            &operator, comparison,
        )?));
    }
    Ok(Value::Int(comparison))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VersionPart {
    Number(i64),
    Label(i8),
}

fn compare_versions(left: &str, right: &str) -> i64 {
    let left = version_parts(left);
    let right = version_parts(right);
    let len = left.len().max(right.len());
    for index in 0..len {
        let ordering = compare_version_part(left.get(index), right.get(index));
        if ordering != 0 {
            return ordering;
        }
    }
    0
}

fn version_parts(version: &str) -> Vec<VersionPart> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for ch in version.chars() {
        if ch.is_ascii_alphanumeric() {
            let is_digit = ch.is_ascii_digit();
            if current_is_digit.is_some_and(|was_digit| was_digit != is_digit) {
                push_version_part(&mut parts, &current);
                current.clear();
            }
            current.push(ch);
            current_is_digit = Some(is_digit);
        } else if matches!(ch, '.' | '-' | '_' | '+') {
            if !current.is_empty() {
                push_version_part(&mut parts, &current);
                current.clear();
            }
            current_is_digit = None;
        } else if !current.is_empty() {
            push_version_part(&mut parts, &current);
            current.clear();
            current_is_digit = None;
        }
    }

    if !current.is_empty() {
        push_version_part(&mut parts, &current);
    }

    while matches!(parts.last(), Some(VersionPart::Number(0))) {
        parts.pop();
    }
    parts
}

fn push_version_part(parts: &mut Vec<VersionPart>, part: &str) {
    if part.as_bytes().iter().all(u8::is_ascii_digit) {
        parts.push(VersionPart::Number(part.parse::<i64>().unwrap_or(i64::MAX)));
    } else {
        parts.push(VersionPart::Label(version_label_rank(part)));
    }
}

fn version_label_rank(label: &str) -> i8 {
    match label.to_ascii_lowercase().as_str() {
        "dev" => -6,
        "alpha" | "a" => -5,
        "beta" | "b" => -4,
        "rc" => -3,
        "pl" | "p" => 1,
        _ => -2,
    }
}

fn compare_version_part(left: Option<&VersionPart>, right: Option<&VersionPart>) -> i64 {
    match (left, right) {
        (None, None) => 0,
        (Some(part), None) => compare_part_to_release(*part),
        (None, Some(part)) => -compare_part_to_release(*part),
        (Some(VersionPart::Number(left)), Some(VersionPart::Number(right))) => {
            ordering_to_i64(left.cmp(right))
        }
        (Some(left), Some(right)) => {
            ordering_to_i64(version_part_rank(*left).cmp(&version_part_rank(*right)))
        }
    }
}

fn ordering_to_i64(ordering: std::cmp::Ordering) -> i64 {
    match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn compare_part_to_release(part: VersionPart) -> i64 {
    match part {
        VersionPart::Number(0) => 0,
        VersionPart::Number(_) => 1,
        VersionPart::Label(rank) => ordering_to_i64(rank.cmp(&0)),
    }
}

fn version_part_rank(part: VersionPart) -> i16 {
    match part {
        VersionPart::Number(0) => 0,
        VersionPart::Number(value) => 10 + value.min(1_000) as i16,
        VersionPart::Label(rank) => i16::from(rank),
    }
}

fn version_operator_matches(operator: &str, comparison: i64) -> Result<bool, BuiltinError> {
    match operator.to_ascii_lowercase().as_str() {
        "<" | "lt" => Ok(comparison < 0),
        "<=" | "le" => Ok(comparison <= 0),
        ">" | "gt" => Ok(comparison > 0),
        ">=" | "ge" => Ok(comparison >= 0),
        "==" | "=" | "eq" => Ok(comparison == 0),
        "!=" | "<>" | "ne" => Ok(comparison != 0),
        _ => Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_VALUE",
            format!("builtin version_compare received unsupported operator {operator}"),
        )),
    }
}

fn builtin_print(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("print", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    let string = to_string(&value).map_err(|message| {
        BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!("builtin print could not convert value: {message}"),
        )
    })?;
    context.output().write_php_string(&string);
    Ok(Value::Int(1))
}

fn builtin_printf(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error("printf", "one or more argument(s)"));
    }
    let format = string_arg("printf", &args[0])?;
    let rendered = php_format("printf", format.as_bytes(), &args[1..])?;
    let length = rendered.len() as i64;
    context.output().write_bytes(rendered);
    Ok(Value::Int(length))
}

fn builtin_sprintf(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error("sprintf", "one or more argument(s)"));
    }
    let format = string_arg("sprintf", &args[0])?;
    Ok(Value::string(php_format(
        "sprintf",
        format.as_bytes(),
        &args[1..],
    )?))
}

fn builtin_vprintf(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("vprintf", &args, 2)?;
    let format = string_arg("vprintf", &args[0])?;
    let values = format_array_values("vprintf", &args[1])?;
    let rendered = php_format("vprintf", format.as_bytes(), &values)?;
    let length = rendered.len() as i64;
    context.output().write_bytes(rendered);
    Ok(Value::Int(length))
}

fn builtin_vsprintf(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("vsprintf", &args, 2)?;
    let format = string_arg("vsprintf", &args[0])?;
    let values = format_array_values("vsprintf", &args[1])?;
    Ok(Value::string(php_format(
        "vsprintf",
        format.as_bytes(),
        &values,
    )?))
}

fn builtin_fprintf(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 {
        return Err(arity_error("fprintf", "two or more argument(s)"));
    }
    Err(BuiltinError::new(
        "E_PHP_RUNTIME_STREAM_GAP",
        "builtin fprintf requires stream/resource support, which is not available in this standard-library slice",
    ))
}

fn builtin_basename(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("basename", "one or two argument(s)"));
    }
    let path = string_arg("basename", &args[0])?.to_string_lossy();
    let suffix = args
        .get(1)
        .map(|value| string_arg("basename", value).map(|value| value.to_string_lossy()))
        .transpose()?;
    let mut base = php_basename(&path);
    if let Some(suffix) = suffix
        && !suffix.is_empty()
        && base.ends_with(&suffix)
    {
        base.truncate(base.len() - suffix.len());
    }
    Ok(Value::string(base.into_bytes()))
}

fn builtin_dirname(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("dirname", "one or two argument(s)"));
    }
    let mut path = string_arg("dirname", &args[0])?.to_string_lossy();
    let levels = args
        .get(1)
        .map(|value| int_arg("dirname", value))
        .transpose()?
        .unwrap_or(1)
        .max(1);
    for _ in 0..levels {
        path = php_dirname_once(&path);
    }
    Ok(Value::string(path.into_bytes()))
}

fn builtin_pathinfo(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("pathinfo", "one or two argument(s)"));
    }
    let path = string_arg("pathinfo", &args[0])?.to_string_lossy();
    let flags = args
        .get(1)
        .map(|value| int_arg("pathinfo", value))
        .transpose()?;
    let dirname = php_dirname_once(&path);
    let basename = php_basename(&path);
    let (filename, extension) = split_extension(&basename);
    match flags {
        None => {
            let mut array = crate::PhpArray::new();
            array.insert(
                string_array_key("dirname"),
                Value::string(dirname.into_bytes()),
            );
            array.insert(
                string_array_key("basename"),
                Value::string(basename.into_bytes()),
            );
            if let Some(extension) = extension.clone() {
                array.insert(
                    string_array_key("extension"),
                    Value::string(extension.into_bytes()),
                );
            }
            array.insert(
                string_array_key("filename"),
                Value::string(filename.into_bytes()),
            );
            Ok(Value::Array(array))
        }
        Some(1) => Ok(Value::string(dirname.into_bytes())),
        Some(2) => Ok(Value::string(basename.into_bytes())),
        Some(4) => {
            Ok(extension.map_or(Value::string(""), |value| Value::string(value.into_bytes())))
        }
        Some(8) => Ok(Value::string(filename.into_bytes())),
        Some(_) => Ok(Value::Array(crate::PhpArray::new())),
    }
}

fn builtin_realpath(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("realpath", &args, 1)?;
    let path = string_arg("realpath", &args[0])?.to_string_lossy();
    let resolved = resolve_runtime_path(context, &path);
    if !context.filesystem_capabilities().allows_path(&resolved) {
        return Ok(Value::Bool(false));
    }
    Ok(
        fs::canonicalize(&resolved).map_or(Value::Bool(false), |path| {
            Value::string(path.to_string_lossy().as_bytes().to_vec())
        }),
    )
}

fn builtin_file_exists(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("file_exists", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "file_exists", &args[0], true)?.is_some(),
    ))
}

fn builtin_is_file(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_file", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "is_file", &args[0], true)?
            .is_some_and(|metadata| metadata.is_file()),
    ))
}

fn builtin_is_dir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_dir", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "is_dir", &args[0], true)?
            .is_some_and(|metadata| metadata.is_dir()),
    ))
}

fn builtin_is_link(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_link", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "is_link", &args[0], false)?
            .is_some_and(|metadata| metadata.file_type().is_symlink()),
    ))
}

fn builtin_is_readable(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_readable", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "is_readable", &args[0], true)?.is_some(),
    ))
}

fn builtin_is_writable(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_writable", &args, 1)?;
    Ok(Value::Bool(
        metadata_for_arg(context, "is_writable", &args[0], true)?
            .is_some_and(|metadata| !metadata.permissions().readonly()),
    ))
}

fn builtin_filesize(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("filesize", &args, 1)?;
    Ok(metadata_for_arg(context, "filesize", &args[0], true)?
        .map_or(Value::Bool(false), |metadata| {
            Value::Int(metadata.len() as i64)
        }))
}

fn builtin_filemtime(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("filemtime", &args, 1)?;
    Ok(metadata_for_arg(context, "filemtime", &args[0], true)?
        .map_or(Value::Bool(false), |metadata| {
            Value::Int(metadata_mtime(&metadata))
        }))
}

fn builtin_filetype(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("filetype", &args, 1)?;
    Ok(metadata_for_arg(context, "filetype", &args[0], false)?
        .map_or(Value::Bool(false), |metadata| {
            Value::string(file_type_name(&metadata))
        }))
}

fn builtin_stat(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stat", &args, 1)?;
    Ok(metadata_for_arg(context, "stat", &args[0], true)?.map_or(Value::Bool(false), stat_array))
}

fn builtin_lstat(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("lstat", &args, 1)?;
    Ok(metadata_for_arg(context, "lstat", &args[0], false)?.map_or(Value::Bool(false), stat_array))
}

fn builtin_clearstatcache(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error(
            "clearstatcache",
            "zero, one, or two argument(s)",
        ));
    }
    Ok(Value::Null)
}

fn builtin_fopen(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fopen", &args, 2)?;
    let uri = string_arg("fopen", &args[0])?.to_string_lossy();
    let mode = string_arg("fopen", &args[1])?.to_string_lossy();
    let cwd = context.cwd().to_path_buf();
    let filesystem = context.filesystem_capabilities().clone();
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(StreamWrapperRegistry::new()
        .open(resources, &uri, &mode, &cwd, &filesystem)
        .map_or(Value::Bool(false), Value::Resource))
}

fn builtin_fclose(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fclose", &args, 1)?;
    Ok(resource_arg(&args[0]).map_or(Value::Bool(false), |resource| Value::Bool(resource.close())))
}

fn builtin_fread(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fread", &args, 2)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let length = int_arg("fread", &args[1])?.max(0) as usize;
    Ok(resource
        .read_bytes(length)
        .map_or(Value::Bool(false), Value::string))
}

fn builtin_fwrite(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("fwrite", "two or three argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let mut bytes = string_arg("fwrite", &args[1])?.as_bytes().to_vec();
    if let Some(length) = args.get(2) {
        bytes.truncate(int_arg("fwrite", length)?.max(0) as usize);
    }
    Ok(resource
        .write_bytes(&bytes)
        .map_or(Value::Bool(false), |written| Value::Int(written as i64)))
}

fn builtin_fgets(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("fgets", "one or two argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let mut line = resource.read_line().unwrap_or_default();
    if let Some(length) = args.get(1) {
        line.truncate(int_arg("fgets", length)?.max(0) as usize);
    }
    if line.is_empty() {
        Ok(Value::Bool(false))
    } else {
        Ok(Value::string(line))
    }
}

fn builtin_fgetc(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fgetc", &args, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let byte = resource.read_bytes(1).unwrap_or_default();
    if byte.is_empty() {
        Ok(Value::Bool(false))
    } else {
        Ok(Value::string(byte))
    }
}

fn builtin_feof(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("feof", &args, 1)?;
    Ok(
        resource_arg(&args[0]).map_or(Value::Bool(true), |resource| {
            Value::Bool(resource.eof().unwrap_or(true))
        }),
    )
}

fn builtin_fflush(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("fflush", &args, 1)?;
    Ok(
        resource_arg(&args[0]).map_or(Value::Bool(false), |resource| {
            Value::Bool(resource.flush().is_ok())
        }),
    )
}

fn builtin_fseek(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("fseek", "two or three argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Int(-1));
    };
    let offset = int_arg("fseek", &args[1])?.max(0) as usize;
    Ok(if resource.seek(offset).is_ok() {
        Value::Int(0)
    } else {
        Value::Int(-1)
    })
}

fn builtin_ftell(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("ftell", &args, 1)?;
    Ok(
        resource_arg(&args[0]).map_or(Value::Bool(false), |resource| {
            resource
                .tell()
                .map_or(Value::Bool(false), |offset| Value::Int(offset as i64))
        }),
    )
}

fn builtin_rewind(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("rewind", &args, 1)?;
    Ok(
        resource_arg(&args[0]).map_or(Value::Bool(false), |resource| {
            Value::Bool(resource.rewind().is_ok())
        }),
    )
}

fn builtin_file_get_contents(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("file_get_contents", "one or two argument(s)"));
    }
    let path = string_arg("file_get_contents", &args[0])?.to_string_lossy();
    read_file_value(context, &path)
}

fn builtin_file_put_contents(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 4 {
        return Err(arity_error(
            "file_put_contents",
            "two, three, or four argument(s)",
        ));
    }
    let path = string_arg("file_put_contents", &args[0])?.to_string_lossy();
    let bytes = string_arg("file_put_contents", &args[1])?
        .as_bytes()
        .to_vec();
    let resolved = resolve_runtime_path(context, &path);
    if !context.filesystem_capabilities().allows_path(&resolved) {
        return Ok(Value::Bool(false));
    }
    Ok(fs::write(&resolved, &bytes).map_or(Value::Bool(false), |_| Value::Int(bytes.len() as i64)))
}

fn builtin_readfile(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("readfile", &args, 1)?;
    let path = string_arg("readfile", &args[0])?.to_string_lossy();
    let Value::String(bytes) = read_file_value(context, &path)? else {
        return Ok(Value::Bool(false));
    };
    let len = bytes.len();
    context.output().write_php_string(&bytes);
    Ok(Value::Int(len as i64))
}

fn builtin_copy(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("copy", &args, 2)?;
    let from = resolve_runtime_path(context, &string_arg("copy", &args[0])?.to_string_lossy());
    let to = resolve_runtime_path(context, &string_arg("copy", &args[1])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&from)
        || !context.filesystem_capabilities().allows_path(&to)
    {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(fs::copy(from, to).is_ok()))
}

fn builtin_rename(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("rename", &args, 2)?;
    let from = resolve_runtime_path(context, &string_arg("rename", &args[0])?.to_string_lossy());
    let to = resolve_runtime_path(context, &string_arg("rename", &args[1])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&from)
        || !context.filesystem_capabilities().allows_path(&to)
    {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(fs::rename(from, to).is_ok()))
}

fn builtin_unlink(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("unlink", &args, 1)?;
    let path = resolve_runtime_path(context, &string_arg("unlink", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(fs::remove_file(path).is_ok()))
}

fn builtin_mkdir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 4 {
        return Err(arity_error("mkdir", "one to four argument(s)"));
    }
    let path = resolve_runtime_path(context, &string_arg("mkdir", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(fs::create_dir(&path).is_ok()))
}

fn builtin_rmdir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("rmdir", &args, 1)?;
    let path = resolve_runtime_path(context, &string_arg("rmdir", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(fs::remove_dir(path).is_ok()))
}

fn builtin_touch(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("touch", "one to three argument(s)"));
    }
    let path = resolve_runtime_path(context, &string_arg("touch", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .is_ok(),
    ))
}

fn builtin_tempnam(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("tempnam", &args, 2)?;
    let dir = resolve_runtime_path(context, &string_arg("tempnam", &args[0])?.to_string_lossy());
    let prefix = string_arg("tempnam", &args[1])?.to_string_lossy();
    if !context.filesystem_capabilities().allows_path(&dir) {
        return Ok(Value::Bool(false));
    }
    for index in 0..1000 {
        let path = dir.join(format!("{prefix}{}-{index}", std::process::id()));
        if fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .is_ok()
        {
            return Ok(Value::string(path.to_string_lossy().as_bytes().to_vec()));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_tmpfile(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("tmpfile", &args, 0)?;
    let Some(root) = context.filesystem_capabilities().first_allowed_root() else {
        return Ok(Value::Bool(false));
    };
    let path = root.join(format!("phrust-tmpfile-{}", std::process::id()));
    let _ = fs::write(&path, []);
    let cwd = context.cwd().to_path_buf();
    let filesystem = context.filesystem_capabilities().clone();
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(StreamWrapperRegistry::new()
        .open(resources, &path.to_string_lossy(), "c+", &cwd, &filesystem)
        .map_or(Value::Bool(false), Value::Resource))
}

fn builtin_opendir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("opendir", &args, 1)?;
    let path = string_arg("opendir", &args[0])?.to_string_lossy();
    let resolved = resolve_runtime_path(context, &path);
    if !context.filesystem_capabilities().allows_path(&resolved) || !resolved.is_dir() {
        return Ok(Value::Bool(false));
    }
    let Some(entries) = directory_entries_with_dots(&resolved) else {
        return Ok(Value::Bool(false));
    };
    let uri = resolved.to_string_lossy().to_string();
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Resource(
        resources.register_directory(resolved, entries, uri),
    ))
}

fn builtin_readdir(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("readdir", "zero or one argument(s)"));
    }
    let Some(resource) = args.first().and_then(resource_arg) else {
        return Ok(Value::Bool(false));
    };
    Ok(resource
        .read_dir_entry()
        .ok()
        .flatten()
        .map_or(Value::Bool(false), Value::string))
}

fn builtin_rewinddir(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("rewinddir", "zero or one argument(s)"));
    }
    let Some(resource) = args.first().and_then(resource_arg) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(resource.rewind_dir().is_ok()))
}

fn builtin_closedir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("closedir", &args, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(resources.close(resource.id())))
}

fn builtin_scandir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("scandir", "one or two argument(s)"));
    }
    let path = resolve_runtime_path(context, &string_arg("scandir", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) || !path.is_dir() {
        return Ok(Value::Bool(false));
    }
    let Some(mut entries) = directory_entries_with_dots(&path) else {
        return Ok(Value::Bool(false));
    };
    if args
        .get(1)
        .map(|value| int_arg("scandir", value))
        .transpose()?
        == Some(1)
    {
        entries.reverse();
    }
    Ok(Value::packed_array(
        entries.into_iter().map(Value::string).collect(),
    ))
}

fn builtin_glob(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("glob", "one or two argument(s)"));
    }
    let pattern = string_arg("glob", &args[0])?.to_string_lossy();
    let (directory, file_pattern) = glob_directory_and_pattern(context, &pattern);
    if !context.filesystem_capabilities().allows_path(&directory) || !directory.is_dir() {
        return Ok(Value::Bool(false));
    }
    let mut matches = Vec::new();
    let Ok(read_dir) = fs::read_dir(&directory) else {
        return Ok(Value::Bool(false));
    };
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if glob_pattern_matches(&file_pattern, &name) {
            matches.push(entry.path().to_string_lossy().to_string());
        }
    }
    matches.sort();
    Ok(Value::packed_array(
        matches.into_iter().map(Value::string).collect(),
    ))
}

fn builtin_getcwd(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("getcwd", &args, 0)?;
    Ok(Value::string(
        context.cwd().to_string_lossy().as_bytes().to_vec(),
    ))
}

fn builtin_chdir(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("chdir", &args, 1)?;
    let path = resolve_runtime_path(context, &string_arg("chdir", &args[0])?.to_string_lossy());
    if !context.filesystem_capabilities().allows_path(&path) || !path.is_dir() {
        return Ok(Value::Bool(false));
    }
    context.set_cwd(path);
    Ok(Value::Bool(true))
}

fn builtin_stream_get_wrappers(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_get_wrappers", &args, 0)?;
    Ok(Value::packed_array(vec![
        Value::string("file"),
        Value::string("php"),
    ]))
}

fn builtin_stream_get_meta_data(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_get_meta_data", &args, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let metadata = resource.metadata();
    let flags = resource.flags();
    let mut array = crate::PhpArray::new();
    array.insert(
        string_array_key("wrapper_type"),
        Value::string(metadata.wrapper_type),
    );
    array.insert(
        string_array_key("stream_type"),
        Value::string(metadata.stream_type),
    );
    array.insert(string_array_key("mode"), Value::string(metadata.mode));
    array.insert(string_array_key("uri"), Value::string(metadata.uri));
    array.insert(string_array_key("seekable"), Value::Bool(flags.seekable));
    array.insert(
        string_array_key("eof"),
        Value::Bool(resource.eof().unwrap_or(true)),
    );
    array.insert(string_array_key("timed_out"), Value::Bool(false));
    array.insert(string_array_key("blocked"), Value::Bool(true));
    Ok(Value::Array(array))
}

fn builtin_stream_get_contents(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error(
            "stream_get_contents",
            "one to three argument(s)",
        ));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    if let Some(offset) = args
        .get(2)
        .map(|value| int_arg("stream_get_contents", value))
        .transpose()?
        && offset >= 0
        && resource.seek(offset as usize).is_err()
    {
        return Ok(Value::Bool(false));
    }
    let bytes = if let Some(length) = args
        .get(1)
        .map(|value| int_arg("stream_get_contents", value))
        .transpose()?
    {
        if length < 0 {
            resource.read_to_end()
        } else {
            resource.read_bytes(length as usize)
        }
    } else {
        resource.read_to_end()
    };
    Ok(bytes.map_or(Value::Bool(false), Value::string))
}

fn builtin_stream_copy_to_stream(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 4 {
        return Err(arity_error(
            "stream_copy_to_stream",
            "two to four argument(s)",
        ));
    }
    let Some(source) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let Some(destination) = resource_arg(&args[1]) else {
        return Ok(Value::Bool(false));
    };
    if let Some(offset) = args
        .get(3)
        .map(|value| int_arg("stream_copy_to_stream", value))
        .transpose()?
        && offset >= 0
        && source.seek(offset as usize).is_err()
    {
        return Ok(Value::Bool(false));
    }
    let bytes = if let Some(length) = args
        .get(2)
        .map(|value| int_arg("stream_copy_to_stream", value))
        .transpose()?
    {
        if length < 0 {
            source.read_to_end()
        } else {
            source.read_bytes(length as usize)
        }
    } else {
        source.read_to_end()
    };
    let Ok(bytes) = bytes else {
        return Ok(Value::Bool(false));
    };
    Ok(destination
        .write_bytes(&bytes)
        .map(|written| Value::Int(written as i64))
        .unwrap_or(Value::Bool(false)))
}

fn builtin_stream_context_create(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error(
            "stream_context_create",
            "zero or one argument(s)",
        ));
    }
    let options = match args.first().map(deref_value) {
        None => crate::PhpArray::new(),
        Some(Value::Array(array)) => array,
        Some(_) => return Ok(Value::Bool(false)),
    };
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Resource(resources.register_stream_context(options)))
}

fn builtin_stream_context_get_options(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_context_get_options", &args, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    Ok(resource
        .context_options()
        .map_or(Value::Bool(false), Value::Array))
}

fn builtin_stream_context_set_option(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 2 && args.len() != 4 {
        return Err(arity_error(
            "stream_context_set_option",
            "two or four argument(s)",
        ));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    if args.len() == 2 {
        let Value::Array(options) = deref_value(&args[1]) else {
            return Ok(Value::Bool(false));
        };
        for (wrapper_key, wrapper_value) in options.iter() {
            let wrapper = match wrapper_key {
                ArrayKey::String(wrapper) => wrapper.to_string_lossy(),
                ArrayKey::Int(_) => continue,
            };
            let Value::Array(wrapper_options) = deref_value(wrapper_value) else {
                continue;
            };
            for (option_key, option_value) in wrapper_options.iter() {
                let option = match option_key {
                    ArrayKey::String(option) => option.to_string_lossy(),
                    ArrayKey::Int(_) => continue,
                };
                if resource
                    .set_context_option(wrapper.clone(), option, option_value.clone())
                    .is_err()
                {
                    return Ok(Value::Bool(false));
                }
            }
        }
        return Ok(Value::Bool(true));
    }
    let wrapper = string_arg("stream_context_set_option", &args[1])?.to_string_lossy();
    let option = string_arg("stream_context_set_option", &args[2])?.to_string_lossy();
    Ok(Value::Bool(
        resource
            .set_context_option(wrapper, option, args[3].clone())
            .is_ok(),
    ))
}

fn builtin_stream_resolve_include_path(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_resolve_include_path", &args, 1)?;
    let file = string_arg("stream_resolve_include_path", &args[0])?.to_string_lossy();
    let raw = Path::new(&file);
    let mut candidates = Vec::new();
    if raw.is_absolute() {
        candidates.push(normalize_runtime_path(raw));
    } else {
        for entry in context.include_path() {
            let base = if entry.is_absolute() {
                entry.clone()
            } else {
                context.cwd().join(entry)
            };
            candidates.push(normalize_runtime_path(&base.join(raw)));
        }
    }
    for candidate in candidates {
        if context.filesystem_capabilities().allows_path(&candidate) && candidate.exists() {
            return Ok(Value::string(
                candidate.to_string_lossy().as_bytes().to_vec(),
            ));
        }
    }
    Ok(Value::Bool(false))
}

fn builtin_stream_is_local(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_is_local", &args, 1)?;
    match deref_value(&args[0]) {
        Value::Resource(resource) => {
            let metadata = resource.metadata();
            Ok(Value::Bool(matches!(
                metadata.wrapper_type.as_str(),
                "plainfile" | "PHP"
            )))
        }
        Value::String(path) => {
            let path = path.to_string_lossy();
            if is_remote_stream_uri(&path) {
                return Ok(Value::Bool(false));
            }
            if path.starts_with("php://") {
                return Ok(Value::Bool(true));
            }
            let resolved = resolve_runtime_path(context, &path);
            Ok(Value::Bool(
                context.filesystem_capabilities().allows_path(&resolved),
            ))
        }
        _ => Ok(Value::Bool(false)),
    }
}

fn builtin_stream_isatty(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("stream_isatty", &args, 1)?;
    Ok(Value::Bool(false))
}

fn builtin_preg_match(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 5 {
        return Err(arity_error("preg_match", "two to five argument(s)"));
    }
    let pattern = string_arg("preg_match", &args[0])?;
    let subject = string_arg("preg_match", &args[1])?;
    let flags = args
        .get(3)
        .map(|value| int_arg("preg_match", value))
        .transpose()?
        .unwrap_or(0);
    let offset = args
        .get(4)
        .map(|value| int_arg("preg_match", value))
        .transpose()?
        .unwrap_or(0);
    let subject_bytes = subject.as_bytes();
    if offset < 0 || offset as usize > subject_bytes.len() {
        context.set_preg_last_error(
            pcre::PREG_BAD_UTF8_OFFSET_ERROR,
            pcre::preg_error_message(pcre::PREG_BAD_UTF8_OFFSET_ERROR),
        );
        return Ok(Value::Bool(false));
    }
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };
    match compiled.captures(&subject_bytes[offset as usize..]) {
        Ok(Some(captures)) => {
            let matches = pcre::captures_to_array(&captures, flags);
            assign_reference_arg(args.get(2), matches);
            context.clear_preg_last_error();
            Ok(Value::Int(1))
        }
        Ok(None) => {
            assign_reference_arg(args.get(2), Value::packed_array(Vec::new()));
            context.clear_preg_last_error();
            Ok(Value::Int(0))
        }
        Err(error) => preg_failure(context, error),
    }
}

fn builtin_preg_match_all(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 5 {
        return Err(arity_error("preg_match_all", "two to five argument(s)"));
    }
    let pattern = string_arg("preg_match_all", &args[0])?;
    let subject = string_arg("preg_match_all", &args[1])?;
    let flags = args
        .get(3)
        .map(|value| int_arg("preg_match_all", value))
        .transpose()?
        .unwrap_or(pcre::PREG_PATTERN_ORDER);
    let offset = args
        .get(4)
        .map(|value| int_arg("preg_match_all", value))
        .transpose()?
        .unwrap_or(0);
    let subject_bytes = subject.as_bytes();
    if offset < 0 || offset as usize > subject_bytes.len() {
        context.set_preg_last_error(
            pcre::PREG_BAD_UTF8_OFFSET_ERROR,
            pcre::preg_error_message(pcre::PREG_BAD_UTF8_OFFSET_ERROR),
        );
        return Ok(Value::Bool(false));
    }
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };

    let mut all = Vec::new();
    for captures in compiled.captures_iter(&subject_bytes[offset as usize..]) {
        match captures {
            Ok(captures) => all.push(pcre::captures_to_array(&captures, flags)),
            Err(error) => return preg_failure(context, error.into()),
        }
    }
    let count = all.len() as i64;
    let output = if flags & pcre::PREG_SET_ORDER != 0 {
        Value::packed_array(all)
    } else {
        pattern_order_matches(all)
    };
    assign_reference_arg(args.get(2), output);
    context.clear_preg_last_error();
    Ok(Value::Int(count))
}

fn builtin_preg_replace(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 3 || args.len() > 5 {
        return Err(arity_error("preg_replace", "three to five argument(s)"));
    }
    let pattern = string_arg("preg_replace", &args[0])?;
    let replacement = string_arg("preg_replace", &args[1])?;
    let limit = args
        .get(3)
        .map(|value| int_arg("preg_replace", value))
        .transpose()?
        .unwrap_or(-1);
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };
    let mut count = 0;
    let result = match preg_replace_subject(
        &compiled,
        replacement.as_bytes(),
        &args[2],
        limit,
        &mut count,
    ) {
        Ok(result) => result,
        Err(error) => return preg_failure(context, error),
    };
    assign_reference_arg(args.get(4), Value::Int(count));
    context.clear_preg_last_error();
    Ok(result)
}

fn builtin_preg_replace_callback(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 3 || args.len() > 5 {
        return Err(arity_error(
            "preg_replace_callback",
            "three to five argument(s)",
        ));
    }
    let pattern = string_arg("preg_replace_callback", &args[0])?;
    let limit = args
        .get(3)
        .map(|value| int_arg("preg_replace_callback", value))
        .transpose()?
        .unwrap_or(-1);
    let callback_name = match deref_value(&args[1]) {
        Value::Callable(CallableValue::InternalBuiltin { name }) => name.clone(),
        _ => {
            return Err(BuiltinError::new(
                "E_PHP_RUNTIME_CALLABLE_CONTEXT_REQUIRED",
                "preg_replace_callback requires VM callable dispatch for user callbacks",
            ));
        }
    };
    let Some(callback) = BuiltinRegistry::new().get(&callback_name) else {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_UNDEFINED_CALLBACK",
            format!("Undefined callback `{callback_name}`"),
        ));
    };
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };
    let mut count = 0;
    let result = preg_replace_callback_subject(
        context, &compiled, callback, &args[2], limit, &mut count, span,
    )?;
    assign_reference_arg(args.get(4), Value::Int(count));
    context.clear_preg_last_error();
    Ok(result)
}

fn builtin_preg_split(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 4 {
        return Err(arity_error("preg_split", "two to four argument(s)"));
    }
    let pattern = string_arg("preg_split", &args[0])?;
    let subject = string_arg("preg_split", &args[1])?;
    let limit = args
        .get(2)
        .map(|value| int_arg("preg_split", value))
        .transpose()?
        .unwrap_or(-1);
    let flags = args
        .get(3)
        .map(|value| int_arg("preg_split", value))
        .transpose()?
        .unwrap_or(0);
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };
    let mut pieces = PhpArray::new();
    let mut last_end = 0usize;
    let mut emitted = 0i64;
    for captures in compiled.captures_iter(subject.as_bytes()) {
        let captures = match captures {
            Ok(captures) => captures,
            Err(error) => return preg_failure(context, error.into()),
        };
        let Some(full) = captures.get(0) else {
            continue;
        };
        if limit > 0 && emitted >= limit - 1 {
            break;
        }
        append_split_piece(
            &mut pieces,
            &subject.as_bytes()[last_end..full.start()],
            last_end,
            flags,
        );
        emitted += 1;
        if flags & pcre::PREG_SPLIT_DELIM_CAPTURE != 0 {
            for index in 1..captures.len() {
                if let Some(capture) = captures.get(index) {
                    append_split_piece(&mut pieces, capture.as_bytes(), capture.start(), flags);
                }
            }
        }
        last_end = full.end();
    }
    append_split_piece(
        &mut pieces,
        &subject.as_bytes()[last_end..],
        last_end,
        flags,
    );
    context.clear_preg_last_error();
    Ok(Value::Array(pieces))
}

fn builtin_preg_grep(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("preg_grep", "two to three argument(s)"));
    }
    let pattern = string_arg("preg_grep", &args[0])?;
    let flags = args
        .get(2)
        .map(|value| int_arg("preg_grep", value))
        .transpose()?
        .unwrap_or(0);
    let Some(compiled) = compile_preg_pattern(context, pattern) else {
        return Ok(Value::Bool(false));
    };
    let Value::Array(input) = deref_value(&args[1]) else {
        return Err(type_error("preg_grep", "array", &args[1]));
    };
    let mut output = PhpArray::new();
    for (key, value) in input.iter() {
        let text = to_string(value)
            .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TYPE_ERROR", message))?;
        let is_match = match compiled.is_match(text.as_bytes()) {
            Ok(is_match) => is_match,
            Err(error) => return preg_failure(context, error),
        };
        if is_match != (flags & pcre::PREG_GREP_INVERT != 0) {
            output.insert(key.clone(), value.clone());
        }
    }
    context.clear_preg_last_error();
    Ok(Value::Array(output))
}

fn builtin_preg_quote(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("preg_quote", "one or two argument(s)"));
    }
    let text = string_arg("preg_quote", &args[0])?;
    let delimiter = args
        .get(1)
        .map(|value| string_arg("preg_quote", value))
        .transpose()?
        .and_then(|delimiter| delimiter.as_bytes().first().copied());
    Ok(Value::string(pcre::preg_quote(text.as_bytes(), delimiter)))
}

fn builtin_preg_last_error(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("preg_last_error", &args, 0)?;
    Ok(Value::Int(context.preg_last_error().0))
}

fn builtin_preg_last_error_msg(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("preg_last_error_msg", &args, 0)?;
    Ok(Value::string(context.preg_last_error().1))
}

fn builtin_date_default_timezone_get(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_default_timezone_get", &args, 0)?;
    Ok(Value::string(context.default_timezone()))
}

fn builtin_date(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("date", "one or two argument(s)"));
    }
    let format = string_arg("date", &args[0])?.to_string_lossy();
    let timestamp = args
        .get(1)
        .map(|value| int_arg("date", value))
        .transpose()?
        .unwrap_or_else(datetime::current_timestamp);
    Ok(Value::string(datetime::format_timestamp(
        timestamp,
        context.default_timezone(),
        &format,
    )))
}

fn builtin_time(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("time", &args, 0)?;
    Ok(Value::Int(datetime::current_timestamp()))
}

fn builtin_strtotime(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("strtotime", "one or two argument(s)"));
    }
    let text = string_arg("strtotime", &args[0])?.to_string_lossy();
    let base = args
        .get(1)
        .map(|value| int_arg("strtotime", value))
        .transpose()?
        .unwrap_or_else(datetime::current_timestamp);
    Ok(datetime::parse_datetime_text(&text, base).map_or(Value::Bool(false), Value::Int))
}

fn builtin_token_get_all(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(arity_error("token_get_all", "1 or 2 argument(s)"));
    }
    let source = to_string(&args[0])
        .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TOKENIZER_TYPE", message))?
        .to_string_lossy();
    let flags = args
        .get(1)
        .map_or(Ok(0), to_int)
        .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TOKENIZER_TYPE", message))?;
    crate::tokenizer::tokenize(&source, flags).map(crate::tokenizer::token_get_all_value)
}

fn builtin_token_name(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("token_name", &args, 1)?;
    let id = to_int(&args[0])
        .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TOKENIZER_TYPE", message))?;
    Ok(Value::string(
        crate::tokenizer::token_name_for_id(id)
            .unwrap_or("UNKNOWN")
            .as_bytes()
            .to_vec(),
    ))
}

fn builtin_spl_object_id(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("spl_object_id", &args, 1)?;
    let Value::Object(object) = deref_value(&args[0]) else {
        return Err(type_error("spl_object_id", "object", &args[0]));
    };
    Ok(Value::Int(object.id() as i64))
}

fn builtin_spl_object_hash(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("spl_object_hash", &args, 1)?;
    let Value::Object(object) = deref_value(&args[0]) else {
        return Err(type_error("spl_object_hash", "object", &args[0]));
    };
    Ok(Value::string(format!("{:032x}", object.id())))
}

fn builtin_date_default_timezone_set(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("date_default_timezone_set", &args, 1)?;
    let identifier = string_arg("date_default_timezone_set", &args[0])?.to_string_lossy();
    if !datetime::is_valid_timezone(&identifier) {
        return Ok(Value::Bool(false));
    }
    context.set_default_timezone(identifier);
    Ok(Value::Bool(true))
}

fn builtin_timezone_identifiers_list(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error(
            "timezone_identifiers_list",
            "zero to two argument(s)",
        ));
    }
    Ok(Value::packed_array(
        datetime::TIMEZONE_IDENTIFIERS
            .iter()
            .map(|identifier| Value::string(*identifier))
            .collect(),
    ))
}

fn builtin_json_encode(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("json_encode", "one to three argument(s)"));
    }
    let flags = args
        .get(1)
        .map(|value| int_arg("json_encode", value))
        .transpose()?
        .unwrap_or(0);
    match php_value_to_json(&args[0], flags) {
        Ok(json) => {
            let encoded = if flags & JSON_PRETTY_PRINT != 0 {
                serde_json::to_string_pretty(&json)
            } else {
                serde_json::to_string(&json)
            };
            match encoded {
                Ok(encoded) => {
                    context.set_json_last_error(JSON_ERROR_NONE);
                    Ok(Value::string(normalize_json_encoded(encoded, flags)))
                }
                Err(_) => json_failure(context, flags, JSON_ERROR_SYNTAX),
            }
        }
        Err(code) => json_failure(context, flags, code),
    }
}

fn builtin_json_decode(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 4 {
        return Err(arity_error("json_decode", "one to four argument(s)"));
    }
    let input = string_arg("json_decode", &args[0])?;
    let associative = args
        .get(1)
        .map(|value| {
            if matches!(deref_value(value), Value::Null) {
                Ok(false)
            } else {
                to_bool(value)
                    .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TYPE_ERROR", message))
            }
        })
        .transpose()?
        .unwrap_or(false);
    let depth = args
        .get(2)
        .map(|value| int_arg("json_decode", value))
        .transpose()?
        .unwrap_or(512);
    let flags = args
        .get(3)
        .map(|value| int_arg("json_decode", value))
        .transpose()?
        .unwrap_or(0);
    if depth <= 0 {
        return json_failure(context, flags, JSON_ERROR_DEPTH);
    }
    let Ok(input) = std::str::from_utf8(input.as_bytes()) else {
        return json_failure(context, flags, JSON_ERROR_UTF8);
    };
    match serde_json::from_str::<JsonValue>(input) {
        Ok(json) => {
            context.set_json_last_error(JSON_ERROR_NONE);
            Ok(json_to_php_value(
                json,
                associative || flags & JSON_OBJECT_AS_ARRAY != 0,
            ))
        }
        Err(_) => json_failure(context, flags, JSON_ERROR_SYNTAX),
    }
}

fn builtin_json_validate(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("json_validate", "one to three argument(s)"));
    }
    let input = string_arg("json_validate", &args[0])?;
    let depth = args
        .get(1)
        .map(|value| int_arg("json_validate", value))
        .transpose()?
        .unwrap_or(512);
    let flags = args
        .get(2)
        .map(|value| int_arg("json_validate", value))
        .transpose()?
        .unwrap_or(0);
    if depth <= 0 {
        context.set_json_last_error(JSON_ERROR_DEPTH);
        return Ok(Value::Bool(false));
    }
    let Ok(input) = std::str::from_utf8(input.as_bytes()) else {
        context.set_json_last_error(JSON_ERROR_UTF8);
        return Ok(Value::Bool(false));
    };
    match serde_json::from_str::<JsonValue>(input) {
        Ok(_) => {
            context.set_json_last_error(JSON_ERROR_NONE);
            Ok(Value::Bool(true))
        }
        Err(_) if flags & JSON_THROW_ON_ERROR != 0 => Err(BuiltinError::new(
            "E_PHP_RUNTIME_JSON_EXCEPTION",
            json_error_message(JSON_ERROR_SYNTAX),
        )),
        Err(_) => {
            context.set_json_last_error(JSON_ERROR_SYNTAX);
            Ok(Value::Bool(false))
        }
    }
}

fn builtin_json_last_error(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("json_last_error", &args, 0)?;
    Ok(Value::Int(context.json_last_error().0))
}

fn builtin_json_last_error_msg(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("json_last_error_msg", &args, 0)?;
    Ok(Value::string(context.json_last_error().1))
}

fn builtin_gettype(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("gettype", &args, 1)?;
    Ok(Value::string(php_gettype(
        &args.into_iter().next().expect("checked arity"),
    )))
}

fn builtin_get_debug_type(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("get_debug_type", &args, 1)?;
    Ok(Value::string(php_debug_type(
        &args.into_iter().next().expect("checked arity"),
    )))
}

fn builtin_get_resource_id(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("get_resource_id", &args, 1)?;
    match deref_value(args.first().expect("checked arity")) {
        Value::Resource(resource) => Ok(Value::Int(resource.id().get() as i64)),
        _ => Ok(Value::Bool(false)),
    }
}

fn builtin_get_resource_type(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("get_resource_type", &args, 1)?;
    match deref_value(args.first().expect("checked arity")) {
        Value::Resource(resource) => Ok(Value::string(resource.resource_type().into_bytes())),
        _ => Ok(Value::Bool(false)),
    }
}

fn builtin_is_int(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_int", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Int(_)
    )))
}

fn builtin_is_string(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_string", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::String(_)
    )))
}

fn builtin_is_bool(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_bool", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Bool(_)
    )))
}

fn builtin_is_null(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_null", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Null
    )))
}

fn builtin_is_array(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_array", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Array(_)
    )))
}

fn builtin_is_float(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_float", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Float(_)
    )))
}

fn builtin_is_object(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_object", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) | Value::Callable(_)
    )))
}

fn builtin_is_resource(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_resource", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Resource(_)
    )))
}

fn builtin_is_scalar(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_scalar", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_)
    )))
}

fn builtin_is_countable(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_countable", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Array(_)
    )))
}

fn builtin_is_iterable(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("is_iterable", &args, 1)?;
    Ok(Value::Bool(matches!(
        deref_value(args.first().expect("checked arity")),
        Value::Array(_) | Value::Generator(_)
    )))
}

fn builtin_boolval(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("boolval", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    to_bool(&value)
        .map(Value::Bool)
        .map_err(|message| conversion_error("boolval", message))
}

fn builtin_intval(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("intval", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    to_int(&value)
        .map(Value::Int)
        .map_err(|message| conversion_error("intval", message))
}

fn builtin_floatval(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("floatval", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    to_float(&value)
        .map(Value::float)
        .map_err(|message| conversion_error("floatval", message))
}

fn builtin_strval(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("strval", &args, 1)?;
    let value = args.into_iter().next().expect("checked arity");
    to_string(&value)
        .map(Value::String)
        .map_err(|message| conversion_error("strval", message))
}

fn builtin_var_dump(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    let mut formatter = DebugFormatter::default();
    for value in &args {
        formatter.write_var_dump_value(context.output(), value, 0);
    }
    Ok(Value::Null)
}

fn builtin_print_r(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin print_r expects one or two argument(s)",
        ));
    }
    let return_output = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("print_r", message))?
        .unwrap_or(false);
    let mut output = OutputBuffer::new();
    DebugFormatter::default().write_print_r_value(&mut output, &args[0], 0);
    if return_output {
        Ok(Value::string(output.into_bytes()))
    } else {
        context.output().write_bytes(output.as_bytes());
        Ok(Value::Bool(true))
    }
}

fn builtin_serialize(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("serialize", &args, 1)?;
    serialize_value(&args[0])
        .map(Value::String)
        .map_err(|error| serialization_error("serialize", error.message()))
}

fn builtin_unserialize(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin unserialize expects one or two argument(s)",
        ));
    }
    let Value::String(input) = &args[0] else {
        return Err(type_error("unserialize", "string", &args[0]));
    };
    match unserialize_value(input, UnserializeOptions::default()) {
        Ok(value) => Ok(value),
        Err(_) => {
            context.php_warning(
                "E_PHP_RUNTIME_UNSERIALIZE_OFFSET",
                format!(
                    "unserialize(): Error at offset 0 of {} bytes",
                    input.as_bytes().len()
                ),
                span,
            );
            Ok(Value::Bool(false))
        }
    }
}

fn builtin_var_export(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            "builtin var_export expects one or two argument(s)",
        ));
    }
    let return_output = args
        .get(1)
        .map(to_bool)
        .transpose()
        .map_err(|message| conversion_error("var_export", message))?
        .unwrap_or(false);
    let mut output = OutputBuffer::new();
    DebugFormatter::default().write_var_export_value(&mut output, &args[0], 0);
    if return_output {
        Ok(Value::string(output.into_bytes()))
    } else {
        context.output().write_bytes(output.as_bytes());
        Ok(Value::Null)
    }
}

fn serialization_error(name: &str, message: &str) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_SERIALIZATION_ERROR",
        format!("builtin {name} failed: {message}"),
    )
}

fn type_error(name: &str, expected: &str, actual: &Value) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_TYPE",
        format!(
            "builtin {name} expects {expected}, got {}",
            runtime_type_name(actual)
        ),
    )
}

fn value_error(name: &str, message: &str) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_VALUE",
        format!("builtin {name}: {message}"),
    )
}

fn conversion_error(name: &str, message: String) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_BUILTIN_TYPE",
        format!("builtin {name} could not convert value: {message}"),
    )
}

fn string_arg(name: &str, value: &Value) -> Result<crate::PhpString, BuiltinError> {
    to_string(value).map_err(|message| {
        BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!("builtin {name} expects string-compatible value: {message}"),
        )
    })
}

fn int_arg(name: &str, value: &Value) -> Result<i64, BuiltinError> {
    to_int(value).map_err(|message| {
        BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!("builtin {name} expects int-compatible value: {message}"),
        )
    })
}

fn float_arg(name: &str, value: &Value) -> Result<f64, BuiltinError> {
    to_float(value).map_err(|message| {
        BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!("builtin {name} expects float-compatible value: {message}"),
        )
    })
}

fn string_array_key(value: &str) -> ArrayKey {
    ArrayKey::String(crate::PhpString::from_test_str(value))
}

fn php_path_separators() -> &'static [char] {
    if cfg!(windows) { &['/', '\\'] } else { &['/'] }
}

fn trim_trailing_path_separators(path: &str) -> &str {
    let trimmed = path.trim_end_matches(php_path_separators());
    if trimmed.is_empty() && path.starts_with(php_path_separators()) {
        &path[..1]
    } else {
        trimmed
    }
}

fn php_basename(path: &str) -> String {
    let trimmed = trim_trailing_path_separators(path);
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .rsplit(php_path_separators())
        .next()
        .unwrap_or(trimmed)
        .to_owned()
}

fn php_dirname_once(path: &str) -> String {
    let trimmed = trim_trailing_path_separators(path);
    if trimmed.is_empty() {
        return ".".to_owned();
    }
    let Some(index) = trimmed.rfind(php_path_separators()) else {
        return ".".to_owned();
    };
    if index == 0 {
        return trimmed[..1].to_owned();
    }
    let parent = trimmed[..index].trim_end_matches(php_path_separators());
    if parent.is_empty() {
        ".".to_owned()
    } else {
        parent.to_owned()
    }
}

fn split_extension(basename: &str) -> (String, Option<String>) {
    let Some(index) = basename.rfind('.') else {
        return (basename.to_owned(), None);
    };
    if index == 0 {
        return (basename.to_owned(), None);
    }
    (
        basename[..index].to_owned(),
        Some(basename[index + 1..].to_owned()),
    )
}

fn resolve_runtime_path(context: &BuiltinContext<'_>, path: &str) -> PathBuf {
    let raw = Path::new(path);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        context.cwd().join(raw)
    };
    normalize_runtime_path(&joined)
}

fn normalize_runtime_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn metadata_for_arg(
    context: &BuiltinContext<'_>,
    name: &str,
    value: &Value,
    follow_links: bool,
) -> Result<Option<Metadata>, BuiltinError> {
    let path = string_arg(name, value)?.to_string_lossy();
    let resolved = resolve_runtime_path(context, &path);
    if !context.filesystem_capabilities().allows_path(&resolved) {
        return Ok(None);
    }
    let metadata = if follow_links {
        fs::metadata(&resolved)
    } else {
        fs::symlink_metadata(&resolved)
    };
    Ok(metadata.ok())
}

fn resource_arg(value: &Value) -> Option<crate::ResourceRef> {
    match deref_value(value) {
        Value::Resource(resource) => Some(resource),
        _ => None,
    }
}

fn read_file_value(context: &BuiltinContext<'_>, path: &str) -> BuiltinResult {
    let resolved = resolve_runtime_path(context, path);
    if !context.filesystem_capabilities().allows_path(&resolved) {
        return Ok(Value::Bool(false));
    }
    Ok(fs::read(resolved).map_or(Value::Bool(false), Value::string))
}

fn directory_entries_with_dots(path: &Path) -> Option<Vec<String>> {
    let mut entries = vec![".".to_string(), "..".to_string()];
    let read_dir = fs::read_dir(path).ok()?;
    let mut names = read_dir
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    names.sort();
    entries.extend(names);
    Some(entries)
}

fn glob_directory_and_pattern(context: &BuiltinContext<'_>, pattern: &str) -> (PathBuf, String) {
    let wildcard_index = pattern.find(['*', '?']).unwrap_or(pattern.len());
    let parent_end = pattern[..wildcard_index]
        .rfind(php_path_separators())
        .map_or(0, |index| index + 1);
    let (directory, file_pattern) = pattern.split_at(parent_end);
    let directory = if directory.is_empty() {
        context.cwd().to_path_buf()
    } else {
        resolve_runtime_path(context, directory)
    };
    (directory, file_pattern.to_string())
}

fn glob_pattern_matches(pattern: &str, name: &str) -> bool {
    fn matches_bytes(pattern: &[u8], name: &[u8]) -> bool {
        match pattern.split_first() {
            None => name.is_empty(),
            Some((&b'*', rest)) => {
                matches_bytes(rest, name)
                    || (!name.is_empty() && matches_bytes(pattern, &name[1..]))
            }
            Some((&b'?', rest)) => !name.is_empty() && matches_bytes(rest, &name[1..]),
            Some((&expected, rest)) => {
                name.first().copied() == Some(expected) && matches_bytes(rest, &name[1..])
            }
        }
    }
    matches_bytes(pattern.as_bytes(), name.as_bytes())
}

fn is_remote_stream_uri(uri: &str) -> bool {
    matches!(
        uri.split_once("://").map(|(scheme, _)| scheme),
        Some("http" | "https" | "ftp" | "ftps")
    )
}

fn php_value_to_json(value: &Value, flags: i64) -> Result<JsonValue, i64> {
    match deref_value(value) {
        Value::Null | Value::Uninitialized => Ok(JsonValue::Null),
        Value::Bool(value) => Ok(JsonValue::Bool(value)),
        Value::Int(value) => Ok(JsonValue::Number(JsonNumber::from(value))),
        Value::Float(value) => {
            let value = value.to_f64();
            if value.is_finite()
                && value.fract() == 0.0
                && flags & JSON_PRESERVE_ZERO_FRACTION == 0
                && value >= i64::MIN as f64
                && value <= i64::MAX as f64
            {
                Ok(JsonValue::Number(JsonNumber::from(value as i64)))
            } else {
                JsonNumber::from_f64(value)
                    .map(JsonValue::Number)
                    .ok_or(JSON_ERROR_SYNTAX)
            }
        }
        Value::String(value) => std::str::from_utf8(value.as_bytes())
            .map(|text| JsonValue::String(text.to_string()))
            .map_err(|_| JSON_ERROR_UTF8),
        Value::Array(array) => {
            if let Some(elements) = array.packed_elements() {
                elements
                    .into_iter()
                    .map(|value| php_value_to_json(value, flags))
                    .collect::<Result<Vec<_>, _>>()
                    .map(JsonValue::Array)
            } else {
                let mut object = JsonMap::new();
                for (key, value) in array.iter() {
                    let key = match key {
                        ArrayKey::Int(value) => value.to_string(),
                        ArrayKey::String(value) => value.to_string_lossy(),
                    };
                    object.insert(key, php_value_to_json(value, flags)?);
                }
                Ok(JsonValue::Object(object))
            }
        }
        Value::Object(object) => {
            let mut json = JsonMap::new();
            for (name, value) in object.properties_snapshot() {
                json.insert(name, php_value_to_json(&value, flags)?);
            }
            Ok(JsonValue::Object(json))
        }
        Value::Resource(_)
        | Value::Fiber(_)
        | Value::Generator(_)
        | Value::Callable(_)
        | Value::Reference(_) => Err(JSON_ERROR_SYNTAX),
    }
}

fn json_to_php_value(value: JsonValue, associative: bool) -> Value {
    match value {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(value) => Value::Bool(value),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Int)
            .or_else(|| value.as_f64().map(Value::float))
            .unwrap_or(Value::Null),
        JsonValue::String(value) => Value::string(value),
        JsonValue::Array(values) => Value::packed_array(
            values
                .into_iter()
                .map(|value| json_to_php_value(value, associative))
                .collect(),
        ),
        JsonValue::Object(values) if associative => {
            let mut array = crate::PhpArray::new();
            for (key, value) in values {
                array.insert(
                    ArrayKey::String(PhpString::from_test_str(&key)),
                    json_to_php_value(value, associative),
                );
            }
            Value::Array(array)
        }
        JsonValue::Object(values) => {
            let object = ObjectRef::new(&json_std_class());
            for (key, value) in values {
                object.set_property(key, json_to_php_value(value, associative));
            }
            Value::Object(object)
        }
    }
}

fn normalize_json_encoded(mut encoded: String, flags: i64) -> String {
    if flags & JSON_UNESCAPED_SLASHES != 0 {
        encoded = encoded.replace("\\/", "/");
    }

    // serde_json already keeps non-ASCII text unescaped and preserves the
    // decimal marker for finite PHP floats, so these flags are explicit no-ops.
    let _ = flags & (JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION);
    encoded
}

fn compile_preg_pattern(
    context: &mut BuiltinContext<'_>,
    pattern: PhpString,
) -> Option<std::sync::Arc<pcre::CompiledPattern>> {
    match context.pcre_cache().compile(&pattern) {
        Ok(compiled) => Some(compiled),
        Err(error) => {
            context.set_preg_last_error(error.code(), error.message().to_string());
            None
        }
    }
}

fn preg_failure(context: &mut BuiltinContext<'_>, error: pcre::PcreFailure) -> BuiltinResult {
    context.set_preg_last_error(error.code(), error.message().to_string());
    Ok(Value::Bool(false))
}

fn assign_reference_arg(argument: Option<&Value>, value: Value) {
    if let Some(Value::Reference(reference)) = argument {
        reference.set(value);
    }
}

fn pattern_order_matches(matches: Vec<Value>) -> Value {
    let mut grouped: Vec<PhpArray> = Vec::new();
    for match_value in matches {
        let Value::Array(captures) = match_value else {
            continue;
        };
        for (key, value) in captures.iter() {
            let ArrayKey::Int(index) = key else {
                continue;
            };
            let index = *index as usize;
            while grouped.len() <= index {
                grouped.push(PhpArray::new());
            }
            grouped[index].append(value.clone());
        }
    }
    Value::packed_array(grouped.into_iter().map(Value::Array).collect())
}

fn preg_replace_subject(
    compiled: &pcre::CompiledPattern,
    replacement: &[u8],
    subject: &Value,
    limit: i64,
    count: &mut i64,
) -> Result<Value, pcre::PcreFailure> {
    match deref_value(subject) {
        Value::Array(array) => {
            let mut output = PhpArray::new();
            for (key, value) in array.iter() {
                let text = to_string(value).map_err(|message| {
                    pcre::PcreFailure::new(pcre::PREG_INTERNAL_ERROR, message)
                })?;
                let replaced =
                    preg_replace_bytes(compiled, replacement, text.as_bytes(), limit, count)?;
                output.insert(key.clone(), Value::string(replaced));
            }
            Ok(Value::Array(output))
        }
        value => {
            let text = to_string(&value)
                .map_err(|message| pcre::PcreFailure::new(pcre::PREG_INTERNAL_ERROR, message))?;
            preg_replace_bytes(compiled, replacement, text.as_bytes(), limit, count)
                .map(Value::string)
        }
    }
}

fn preg_replace_bytes(
    compiled: &pcre::CompiledPattern,
    replacement: &[u8],
    subject: &[u8],
    limit: i64,
    count: &mut i64,
) -> Result<Vec<u8>, pcre::PcreFailure> {
    let mut output = Vec::new();
    let mut last_end = 0usize;
    for captures in compiled.captures_iter(subject) {
        let captures = captures.map_err(pcre::PcreFailure::from)?;
        let Some(full) = captures.get(0) else {
            continue;
        };
        if limit >= 0 && *count >= limit {
            break;
        }
        output.extend_from_slice(&subject[last_end..full.start()]);
        output.extend_from_slice(&expand_preg_replacement(replacement, &captures));
        last_end = full.end();
        *count += 1;
    }
    output.extend_from_slice(&subject[last_end..]);
    Ok(output)
}

fn preg_replace_callback_subject(
    context: &mut BuiltinContext<'_>,
    compiled: &pcre::CompiledPattern,
    callback: BuiltinEntry,
    subject: &Value,
    limit: i64,
    count: &mut i64,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    match deref_value(subject) {
        Value::Array(array) => {
            let mut output = PhpArray::new();
            for (key, value) in array.iter() {
                let text = to_string(value)
                    .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TYPE_ERROR", message))?;
                let replaced = preg_replace_callback_bytes(
                    context,
                    compiled,
                    callback,
                    text.as_bytes(),
                    limit,
                    count,
                    span.clone(),
                )?;
                output.insert(key.clone(), Value::string(replaced));
            }
            Ok(Value::Array(output))
        }
        value => {
            let text = to_string(&value)
                .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TYPE_ERROR", message))?;
            preg_replace_callback_bytes(
                context,
                compiled,
                callback,
                text.as_bytes(),
                limit,
                count,
                span,
            )
            .map(Value::string)
        }
    }
}

fn preg_replace_callback_bytes(
    context: &mut BuiltinContext<'_>,
    compiled: &pcre::CompiledPattern,
    callback: BuiltinEntry,
    subject: &[u8],
    limit: i64,
    count: &mut i64,
    span: RuntimeSourceSpan,
) -> Result<Vec<u8>, BuiltinError> {
    let mut output = Vec::new();
    let mut last_end = 0usize;
    for captures in compiled.captures_iter(subject) {
        let captures = captures.map_err(|error| {
            let error = pcre::PcreFailure::from(error);
            BuiltinError::new("E_PHP_RUNTIME_PCRE_ERROR", error.message().to_string())
        })?;
        let Some(full) = captures.get(0) else {
            continue;
        };
        if limit >= 0 && *count >= limit {
            break;
        }
        output.extend_from_slice(&subject[last_end..full.start()]);
        let callback_result = (callback.function())(
            context,
            vec![pcre::captures_to_array(&captures, 0)],
            span.clone(),
        )?;
        let callback_text = to_string(&callback_result)
            .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_TYPE_ERROR", message))?;
        output.extend_from_slice(callback_text.as_bytes());
        last_end = full.end();
        *count += 1;
    }
    output.extend_from_slice(&subject[last_end..]);
    Ok(output)
}

fn expand_preg_replacement(replacement: &[u8], captures: &pcre2::bytes::Captures<'_>) -> Vec<u8> {
    let mut output = Vec::new();
    let mut index = 0usize;
    while index < replacement.len() {
        let byte = replacement[index];
        if (byte == b'$' || byte == b'\\') && index + 1 < replacement.len() {
            let next = replacement[index + 1];
            if next.is_ascii_digit() {
                let capture_index = (next - b'0') as usize;
                if let Some(capture) = captures.get(capture_index) {
                    output.extend_from_slice(capture.as_bytes());
                }
                index += 2;
                continue;
            }
        }
        output.push(byte);
        index += 1;
    }
    output
}

fn append_split_piece(array: &mut PhpArray, bytes: &[u8], offset: usize, flags: i64) {
    if flags & pcre::PREG_SPLIT_NO_EMPTY != 0 && bytes.is_empty() {
        return;
    }
    let value = if flags & pcre::PREG_SPLIT_OFFSET_CAPTURE != 0 {
        Value::packed_array(vec![
            Value::string(bytes.to_vec()),
            Value::Int(offset as i64),
        ])
    } else {
        Value::string(bytes.to_vec())
    };
    array.append(value);
}

fn json_failure(context: &mut BuiltinContext<'_>, flags: i64, code: i64) -> BuiltinResult {
    context.set_json_last_error(code);
    if flags & JSON_THROW_ON_ERROR != 0 {
        Err(BuiltinError::new(
            "E_PHP_RUNTIME_JSON_EXCEPTION",
            json_error_message(code),
        ))
    } else {
        Ok(Value::Bool(false))
    }
}

fn json_error_message(code: i64) -> &'static str {
    match code {
        JSON_ERROR_NONE => "No error",
        JSON_ERROR_DEPTH => "Maximum stack depth exceeded",
        JSON_ERROR_UTF8 => "Malformed UTF-8 characters, possibly incorrectly encoded",
        JSON_ERROR_SYNTAX => "Syntax error",
        _ => "JSON error",
    }
}

fn json_std_class() -> ClassEntry {
    ClassEntry {
        name: "stdClass".to_string(),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: ClassFlags::default(),
    }
}

fn metadata_mtime(metadata: &Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs() as i64)
}

fn file_type_name(metadata: &Metadata) -> &'static str {
    let file_type = metadata.file_type();
    if file_type.is_file() {
        "file"
    } else if file_type.is_dir() {
        "dir"
    } else if file_type.is_symlink() {
        "link"
    } else {
        "unknown"
    }
}

fn stat_array(metadata: Metadata) -> Value {
    let size = metadata.len() as i64;
    let mtime = metadata_mtime(&metadata);
    let mode = if metadata.is_dir() {
        0o040000
    } else if metadata.is_file() {
        0o100000
    } else {
        0
    };
    let mut array = crate::PhpArray::new();
    array.insert(ArrayKey::Int(2), Value::Int(mode));
    array.insert(ArrayKey::Int(7), Value::Int(size));
    array.insert(ArrayKey::Int(9), Value::Int(mtime));
    array.insert(string_array_key("mode"), Value::Int(mode));
    array.insert(string_array_key("size"), Value::Int(size));
    array.insert(string_array_key("mtime"), Value::Int(mtime));
    array.insert(
        string_array_key("type"),
        Value::string(file_type_name(&metadata)),
    );
    Value::Array(array)
}

fn numeric_f64_arg(name: &str, value: &Value) -> Result<f64, BuiltinError> {
    to_number(value)
        .map(|number| number.as_f64())
        .map_err(|message| conversion_error(name, message))
}

fn min_max_builtin(name: &str, args: Vec<Value>, pick_max: bool) -> BuiltinResult {
    if args.is_empty() {
        return Err(arity_error(name, "at least one argument"));
    }
    let values = if args.len() == 1 {
        match &args[0] {
            Value::Array(array) => array
                .iter()
                .map(|(_, value)| value.clone())
                .collect::<Vec<_>>(),
            _ => return Err(type_error(name, "array", &args[0])),
        }
    } else {
        args
    };
    if values.is_empty() {
        return Err(value_error(name, "array must contain at least one element"));
    }
    let mut selected = values[0].clone();
    for value in values.into_iter().skip(1) {
        let ordering =
            compare(&value, &selected).map_err(|message| conversion_error(name, message))?;
        if (pick_max && ordering.is_gt()) || (!pick_max && ordering.is_lt()) {
            selected = value;
        }
    }
    Ok(selected)
}

fn group_decimal_integer(integer: &str, separator: &str) -> String {
    if separator.is_empty() || integer.len() <= 3 {
        return integer.to_owned();
    }
    let mut grouped = String::with_capacity(integer.len() + separator.len() * (integer.len() / 3));
    let first_group = integer.len() % 3;
    if first_group != 0 {
        grouped.push_str(&integer[..first_group]);
    }
    for chunk_start in (first_group..integer.len()).step_by(3) {
        if !grouped.is_empty() {
            grouped.push_str(separator);
        }
        grouped.push_str(&integer[chunk_start..chunk_start + 3]);
    }
    grouped
}

fn normalize_offset(len: usize, offset: i64) -> usize {
    if offset >= 0 {
        (offset as usize).min(len)
    } else {
        len.saturating_sub(offset.unsigned_abs() as usize)
    }
}

fn checked_search_offset(name: &str, len: usize, offset: i64) -> Result<usize, BuiltinError> {
    let abs = offset.unsigned_abs() as usize;
    if offset > len as i64 || (offset < 0 && abs > len) {
        return Err(value_error(name, "offset is out of range"));
    }
    Ok(normalize_offset(len, offset))
}

fn string_position(
    name: &str,
    args: Vec<Value>,
    case_insensitive: bool,
    reverse: bool,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            format!("builtin {name} expects two or three argument(s)"),
        ));
    }
    let haystack = string_arg(name, &args[0])?;
    let needle = string_arg(name, &args[1])?;
    let offset = args
        .get(2)
        .map(|value| int_arg(name, value))
        .transpose()?
        .unwrap_or(0);
    let start = checked_search_offset(name, haystack.len(), offset)?;
    let result = if reverse {
        rfind_bytes(
            haystack.as_bytes(),
            needle.as_bytes(),
            start,
            offset >= 0,
            case_insensitive,
        )
    } else {
        find_bytes_from(
            haystack.as_bytes(),
            needle.as_bytes(),
            start,
            case_insensitive,
        )
    };
    Ok(result.map_or(Value::Bool(false), |index| Value::Int(index as i64)))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    find_bytes_from(haystack, needle, 0, false)
}

fn find_bytes_from(
    haystack: &[u8],
    needle: &[u8],
    start: usize,
    case_insensitive: bool,
) -> Option<usize> {
    if needle.is_empty() {
        return Some(start.min(haystack.len()));
    }
    if start > haystack.len() || needle.len() > haystack.len().saturating_sub(start) {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| bytes_equal(window, needle, case_insensitive))
        .map(|index| index + start)
}

fn rfind_bytes(
    haystack: &[u8],
    needle: &[u8],
    start: usize,
    start_is_lower_bound: bool,
    case_insensitive: bool,
) -> Option<usize> {
    if needle.is_empty() {
        return Some(start.min(haystack.len()));
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let max_start = haystack.len().saturating_sub(needle.len());
    let (lower, upper) = if start_is_lower_bound {
        (start.min(max_start), max_start)
    } else {
        (0, start.min(max_start))
    };
    (lower..=upper).rev().find(|index| {
        bytes_equal(
            &haystack[*index..*index + needle.len()],
            needle,
            case_insensitive,
        )
    })
}

fn bytes_equal(left: &[u8], right: &[u8], case_insensitive: bool) -> bool {
    if case_insensitive {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn compare_strings(
    name: &str,
    args: &[Value],
    case_insensitive: bool,
    limit: Option<usize>,
) -> BuiltinResult {
    let left = string_arg(name, &args[0])?;
    let right = string_arg(name, &args[1])?;
    let mut left = left.as_bytes().to_vec();
    let mut right = right.as_bytes().to_vec();
    if let Some(limit) = limit {
        left.truncate(limit);
        right.truncate(limit);
    }
    if case_insensitive {
        left.iter_mut()
            .for_each(|byte| *byte = byte.to_ascii_lowercase());
        right
            .iter_mut()
            .for_each(|byte| *byte = byte.to_ascii_lowercase());
    }
    Ok(Value::Int(match left.cmp(&right) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}

fn trim_builtin(name: &str, args: Vec<Value>, left: bool, right: bool) -> BuiltinResult {
    if !(1..=2).contains(&args.len()) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_ARITY",
            format!("builtin {name} expects one or two argument(s)"),
        ));
    }
    let string = string_arg(name, &args[0])?;
    let mask = args
        .get(1)
        .map(|value| string_arg(name, value))
        .transpose()?;
    let mask = mask
        .as_ref()
        .map_or(b" \t\n\r\0\x0b".as_slice(), crate::PhpString::as_bytes);
    let bytes = string.as_bytes();
    let start = if left {
        bytes
            .iter()
            .position(|byte| !mask.contains(byte))
            .unwrap_or(bytes.len())
    } else {
        0
    };
    let end = if right {
        bytes
            .iter()
            .rposition(|byte| !mask.contains(byte))
            .map_or(start, |index| index + 1)
    } else {
        bytes.len()
    };
    Ok(Value::string(bytes[start..end].to_vec()))
}

fn split_bytes(bytes: &[u8], separator: &[u8]) -> Vec<Vec<u8>> {
    split_bytes_limited(bytes, separator, usize::MAX)
}

fn split_bytes_limited(bytes: &[u8], separator: &[u8], limit: usize) -> Vec<Vec<u8>> {
    if limit == 0 {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut start = 0;
    while parts.len() + 1 < limit {
        let Some(index) = find_bytes_from(bytes, separator, start, false) else {
            break;
        };
        parts.push(bytes[start..index].to_vec());
        start = index + separator.len();
    }
    parts.push(bytes[start..].to_vec());
    parts
}

fn array_arg(name: &str, value: &Value) -> Result<Vec<crate::PhpString>, BuiltinError> {
    let Value::Array(array) = deref_value(value) else {
        return Err(type_error(name, "array", value));
    };
    array
        .iter()
        .map(|(_, value)| string_arg(name, value))
        .collect::<Result<Vec<_>, _>>()
}

fn array_key_arg(name: &str, value: &Value) -> Result<ArrayKey, BuiltinError> {
    ArrayKey::from_value_mvp(&deref_value(value))
        .ok_or_else(|| type_error(name, "int|string key-compatible value", value))
}

fn array_value_arg(name: &str, value: &Value) -> Result<crate::PhpArray, BuiltinError> {
    let Value::Array(array) = deref_value(value) else {
        return Err(type_error(name, "array", value));
    };
    Ok(array)
}

fn array_reference_cell(name: &str, value: &Value) -> Result<crate::ReferenceCell, BuiltinError> {
    let Value::Reference(cell) = value else {
        return Err(type_error(name, "array reference", value));
    };
    Ok(cell.clone())
}

fn array_from_reference_cell(
    name: &str,
    cell: &crate::ReferenceCell,
) -> Result<crate::PhpArray, BuiltinError> {
    let value = cell.get();
    let Value::Array(array) = value else {
        return Err(type_error(name, "array", &value));
    };
    Ok(array)
}

fn array_key_to_value(key: &ArrayKey) -> Value {
    match key {
        ArrayKey::Int(value) => Value::Int(*value),
        ArrayKey::String(value) => Value::String(value.clone()),
    }
}

fn array_value_matches(
    name: &str,
    left: &Value,
    right: &Value,
    strict: bool,
) -> Result<bool, BuiltinError> {
    if strict {
        Ok(identical(left, right))
    } else {
        equal(left, right).map_err(|message| conversion_error(name, message))
    }
}

fn count_recursive(array: &crate::PhpArray) -> usize {
    let mut count = array.len();
    for (_, value) in array.iter() {
        if let Value::Array(child) = deref_value(value) {
            count += count_recursive(&child);
        }
    }
    count
}

fn array_entries(array: &crate::PhpArray) -> Vec<(ArrayKey, Value)> {
    array
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn array_from_entries_preserve(entries: Vec<(ArrayKey, Value)>) -> crate::PhpArray {
    let mut array = crate::PhpArray::new();
    for (key, value) in entries {
        array.insert(key, value);
    }
    array
}

fn array_from_entries_reindex_ints(entries: Vec<(ArrayKey, Value)>) -> crate::PhpArray {
    let mut array = crate::PhpArray::new();
    for (key, value) in entries {
        match key {
            ArrayKey::Int(_) => {
                array.append(value);
            }
            ArrayKey::String(key) => {
                array.insert(ArrayKey::String(key), value);
            }
        }
    }
    array
}

fn array_from_entries_for_slice(
    entries: Vec<(ArrayKey, Value)>,
    preserve_keys: bool,
) -> crate::PhpArray {
    if preserve_keys {
        return array_from_entries_preserve(entries);
    }
    array_from_entries_reindex_ints(entries)
}

fn normalize_slice_start(len: usize, offset: i64) -> usize {
    if offset >= 0 {
        (offset as usize).min(len)
    } else {
        len.saturating_sub(offset.unsigned_abs() as usize)
    }
}

fn slice_entries(
    entries: Vec<(ArrayKey, Value)>,
    offset: i64,
    length: Option<i64>,
) -> Vec<(ArrayKey, Value)> {
    let start = normalize_slice_start(entries.len(), offset);
    let end = match length {
        None => entries.len(),
        Some(length) if length >= 0 => start.saturating_add(length as usize).min(entries.len()),
        Some(length) => entries.len().saturating_sub(length.unsigned_abs() as usize),
    };
    if end < start {
        Vec::new()
    } else {
        entries[start..end].to_vec()
    }
}

fn splice_length(total: usize, start: usize, length: i64) -> Result<usize, BuiltinError> {
    Ok(if length >= 0 {
        (length as usize).min(total.saturating_sub(start))
    } else {
        total
            .saturating_sub(start)
            .saturating_sub(length.unsigned_abs() as usize)
    })
}

fn splice_replacement_values(name: &str, value: &Value) -> Result<Vec<Value>, BuiltinError> {
    match deref_value(value) {
        Value::Array(array) => Ok(array.iter().map(|(_, value)| value.clone()).collect()),
        value => Ok(vec![string_arg(name, &value).map(Value::String)?]),
    }
}

fn merge_recursive_into(output: &mut crate::PhpArray, input: &crate::PhpArray) {
    for (key, value) in input.iter() {
        match key {
            ArrayKey::Int(_) => {
                output.append(value.clone());
            }
            ArrayKey::String(key) => {
                let out_key = ArrayKey::String(key.clone());
                if let Some(existing) = output.get(&out_key).cloned() {
                    let merged = merge_recursive_values(existing, value.clone());
                    output.insert(out_key, merged);
                } else {
                    output.insert(out_key, value.clone());
                }
            }
        }
    }
}

fn merge_recursive_values(left: Value, right: Value) -> Value {
    match (deref_value(&left), deref_value(&right)) {
        (Value::Array(mut left), Value::Array(right)) => {
            merge_recursive_into(&mut left, &right);
            Value::Array(left)
        }
        (left, right) => Value::packed_array(vec![left, right]),
    }
}

fn string_list_arg(name: &str, value: &Value) -> Result<Vec<crate::PhpString>, BuiltinError> {
    match deref_value(value) {
        Value::Array(array) => array
            .iter()
            .map(|(_, value)| string_arg(name, value))
            .collect::<Result<Vec<_>, _>>(),
        value => Ok(vec![string_arg(name, &value)?]),
    }
}

fn replace_subject(
    subject: &Value,
    search: &[crate::PhpString],
    replace: &[crate::PhpString],
    count: &mut i64,
) -> BuiltinResult {
    match deref_value(subject) {
        Value::Array(array) => Ok(Value::Array(crate::PhpArray::from_packed(
            array
                .iter()
                .map(|(_, value)| replace_subject(value, search, replace, count))
                .collect::<Result<Vec<_>, _>>()?,
        ))),
        value => {
            let mut bytes = string_arg("str_replace", &value)?.into_bytes();
            for (index, needle) in search.iter().enumerate() {
                if needle.is_empty() {
                    continue;
                }
                let replacement = replace
                    .get(index)
                    .map_or(b"".as_slice(), crate::PhpString::as_bytes);
                bytes = replace_all(&bytes, needle.as_bytes(), replacement, count);
            }
            Ok(Value::string(bytes))
        }
    }
}

fn replace_all(bytes: &[u8], needle: &[u8], replacement: &[u8], count: &mut i64) -> Vec<u8> {
    let mut output = Vec::new();
    let mut start = 0;
    while let Some(index) = find_bytes_from(bytes, needle, start, false) {
        output.extend_from_slice(&bytes[start..index]);
        output.extend_from_slice(replacement);
        *count += 1;
        start = index + needle.len();
    }
    output.extend_from_slice(&bytes[start..]);
    output
}

fn replace_map(bytes: &[u8], replacements: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if let Some((needle, replacement)) = replacements
            .iter()
            .find(|(needle, _)| !needle.is_empty() && bytes[index..].starts_with(needle))
        {
            output.extend_from_slice(replacement);
            index += needle.len();
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    output
}

fn change_first_ascii(string: crate::PhpString, uppercase: bool) -> Vec<u8> {
    let mut bytes = string.into_bytes();
    if let Some(first) = bytes.first_mut() {
        *first = if uppercase {
            first.to_ascii_uppercase()
        } else {
            first.to_ascii_lowercase()
        };
    }
    bytes
}

fn repeat_pad(pad: &[u8], length: usize) -> Vec<u8> {
    let mut output = Vec::with_capacity(length);
    while output.len() < length {
        let remaining = length - output.len();
        output.extend_from_slice(&pad[..pad.len().min(remaining)]);
    }
    output
}

#[derive(Clone, Copy, Debug)]
struct PrintfSpec {
    left_align: bool,
    force_sign: bool,
    space_sign: bool,
    zero_pad: bool,
    pad_byte: u8,
    width: Option<usize>,
    precision: Option<usize>,
    specifier: u8,
}

fn php_format(name: &str, format: &[u8], args: &[Value]) -> Result<Vec<u8>, BuiltinError> {
    let mut output = Vec::new();
    let mut format_index = 0;
    let mut arg_index = 0;

    while format_index < format.len() {
        if format[format_index] != b'%' {
            output.push(format[format_index]);
            format_index += 1;
            continue;
        }
        format_index += 1;
        if format_index >= format.len() {
            return Err(value_error(name, "incomplete format specifier"));
        }
        if format[format_index] == b'%' {
            output.push(b'%');
            format_index += 1;
            continue;
        }

        let (spec, next_index) = parse_printf_spec(name, format, format_index)?;
        format_index = next_index;
        let Some(value) = args.get(arg_index) else {
            return Err(BuiltinError::new(
                "E_PHP_RUNTIME_PRINTF_ARGUMENTS",
                format!("builtin {name} has too few arguments for format string"),
            ));
        };
        arg_index += 1;
        output.extend_from_slice(&format_printf_value(name, &spec, value)?);
    }

    Ok(output)
}

fn parse_printf_spec(
    name: &str,
    format: &[u8],
    mut index: usize,
) -> Result<(PrintfSpec, usize), BuiltinError> {
    let mut spec = PrintfSpec {
        left_align: false,
        force_sign: false,
        space_sign: false,
        zero_pad: false,
        pad_byte: b' ',
        width: None,
        precision: None,
        specifier: 0,
    };

    loop {
        match format.get(index).copied() {
            Some(b'-') => spec.left_align = true,
            Some(b'+') => spec.force_sign = true,
            Some(b' ') => {}
            Some(b'0') => spec.zero_pad = true,
            Some(b'\'') => {
                index += 1;
                spec.pad_byte = *format
                    .get(index)
                    .ok_or_else(|| value_error(name, "missing custom padding character"))?;
            }
            _ => break,
        }
        index += 1;
    }

    let width_start = index;
    while format
        .get(index)
        .copied()
        .is_some_and(|byte| byte.is_ascii_digit())
    {
        index += 1;
    }
    if index > width_start {
        spec.width = Some(parse_ascii_usize(
            name,
            &format[width_start..index],
            "width",
        )?);
    }

    if format.get(index) == Some(&b'.') {
        index += 1;
        let precision_start = index;
        while format
            .get(index)
            .copied()
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            index += 1;
        }
        spec.precision = Some(if index == precision_start {
            0
        } else {
            parse_ascii_usize(name, &format[precision_start..index], "precision")?
        });
    }

    while matches!(format.get(index), Some(b'h' | b'l' | b'L')) {
        index += 1;
    }

    let Some(specifier) = format.get(index).copied() else {
        return Err(value_error(name, "incomplete format specifier"));
    };
    if !matches!(
        specifier,
        b's' | b'd' | b'u' | b'f' | b'F' | b'x' | b'X' | b'o' | b'c'
    ) {
        return Err(value_error(name, "unsupported format specifier"));
    }
    spec.specifier = specifier;
    Ok((spec, index + 1))
}

fn parse_ascii_usize(name: &str, digits: &[u8], field: &str) -> Result<usize, BuiltinError> {
    std::str::from_utf8(digits)
        .ok()
        .and_then(|text| text.parse::<usize>().ok())
        .ok_or_else(|| value_error(name, &format!("invalid format {field}")))
}

fn format_printf_value(
    name: &str,
    spec: &PrintfSpec,
    value: &Value,
) -> Result<Vec<u8>, BuiltinError> {
    let bytes = match spec.specifier {
        b's' => {
            let mut bytes = string_arg(name, value)?.into_bytes();
            if let Some(precision) = spec.precision {
                bytes.truncate(precision);
            }
            bytes
        }
        b'c' => vec![int_arg(name, value)?.rem_euclid(256) as u8],
        b'd' => format_signed_decimal(name, spec, int_arg(name, value)?)?.into_bytes(),
        b'u' => (int_arg(name, value)? as u64).to_string().into_bytes(),
        b'x' => format!("{:x}", int_arg(name, value)? as u64).into_bytes(),
        b'X' => format!("{:X}", int_arg(name, value)? as u64).into_bytes(),
        b'o' => format!("{:o}", int_arg(name, value)? as u64).into_bytes(),
        b'f' | b'F' => format_float_decimal(name, spec, float_arg(name, value)?)?.into_bytes(),
        _ => unreachable!("parse_printf_spec validates specifier"),
    };
    Ok(apply_printf_padding(spec, bytes))
}

fn format_signed_decimal(
    name: &str,
    spec: &PrintfSpec,
    value: i64,
) -> Result<String, BuiltinError> {
    let negative = value < 0;
    let mut digits = if negative {
        (-(value as i128)).to_string()
    } else {
        (value as i128).to_string()
    };
    if let Some(precision) = spec.precision
        && digits.len() < precision
    {
        digits = format!("{}{}", "0".repeat(precision - digits.len()), digits);
    }
    Ok(format_numeric_sign(name, spec, negative, digits))
}

fn format_float_decimal(name: &str, spec: &PrintfSpec, value: f64) -> Result<String, BuiltinError> {
    if !value.is_finite() {
        return Err(value_error(
            name,
            "non-finite float formatting is not implemented",
        ));
    }
    let precision = spec.precision.unwrap_or(6);
    let negative = value.is_sign_negative();
    let digits = format!("{:.precision$}", value.abs());
    Ok(format_numeric_sign(name, spec, negative, digits))
}

fn format_numeric_sign(_name: &str, spec: &PrintfSpec, negative: bool, digits: String) -> String {
    if negative {
        format!("-{digits}")
    } else if spec.force_sign {
        format!("+{digits}")
    } else if spec.space_sign {
        format!(" {digits}")
    } else {
        digits
    }
}

fn apply_printf_padding(spec: &PrintfSpec, mut bytes: Vec<u8>) -> Vec<u8> {
    let Some(width) = spec.width else {
        return bytes;
    };
    if bytes.len() >= width {
        return bytes;
    }
    let pad_len = width - bytes.len();
    let pad_byte = if spec.zero_pad && !spec.left_align && spec.pad_byte == b' ' {
        b'0'
    } else {
        spec.pad_byte
    };
    let mut output = Vec::with_capacity(width);
    if spec.left_align {
        output.extend_from_slice(&bytes);
        output.extend(std::iter::repeat_n(b' ', pad_len));
    } else if pad_byte == b'0' && matches!(bytes.first(), Some(b'-' | b'+' | b' ')) {
        output.push(bytes[0]);
        output.extend(std::iter::repeat_n(pad_byte, pad_len));
        output.extend_from_slice(&bytes[1..]);
    } else {
        output.extend(std::iter::repeat_n(pad_byte, pad_len));
        output.append(&mut bytes);
    }
    output
}

fn format_array_values(name: &str, value: &Value) -> Result<Vec<Value>, BuiltinError> {
    let Value::Array(array) = deref_value(value) else {
        return Err(type_error(name, "array", value));
    };
    Ok(array.iter().map(|(_, value)| value.clone()).collect())
}

fn hex_encode(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0f) as usize]);
    }
    output
}

fn hash_digest_bytes(name: &str, algorithm: &str, input: &[u8]) -> Result<Vec<u8>, BuiltinError> {
    match normalized_hash_algorithm(algorithm).as_deref() {
        Some("md5") => Ok(Md5::digest(input).to_vec()),
        Some("sha1") => Ok(Sha1::digest(input).to_vec()),
        Some("crc32") | Some("crc32b") => Ok(crc32fast::hash(input).to_be_bytes().to_vec()),
        _ => Err(value_error(name, "unsupported hash algorithm")),
    }
}

fn hmac_digest_bytes(
    name: &str,
    algorithm: &str,
    key: &[u8],
    input: &[u8],
) -> Result<Vec<u8>, BuiltinError> {
    match normalized_hash_algorithm(algorithm).as_deref() {
        Some("md5") => Ok(hmac_with_block_64(
            if key.len() > 64 {
                Md5::digest(key).to_vec()
            } else {
                key.to_vec()
            },
            input,
            |bytes| Md5::digest(bytes).to_vec(),
        )),
        Some("sha1") => Ok(hmac_with_block_64(
            if key.len() > 64 {
                Sha1::digest(key).to_vec()
            } else {
                key.to_vec()
            },
            input,
            |bytes| Sha1::digest(bytes).to_vec(),
        )),
        _ => Err(value_error(name, "unsupported hash algorithm")),
    }
}

fn hmac_with_block_64(
    mut key: Vec<u8>,
    input: &[u8],
    digest: impl Fn(&[u8]) -> Vec<u8>,
) -> Vec<u8> {
    key.resize(64, 0);
    let outer_pad = key.iter().map(|byte| byte ^ 0x5c).collect::<Vec<_>>();
    let mut inner = key.iter().map(|byte| byte ^ 0x36).collect::<Vec<_>>();
    inner.extend_from_slice(input);
    let inner_digest = digest(&inner);
    let mut outer = outer_pad;
    outer.extend_from_slice(&inner_digest);
    digest(&outer)
}

fn normalized_hash_algorithm(algorithm: &str) -> Option<String> {
    let normalized = algorithm.to_ascii_lowercase().replace('-', "");
    match normalized.as_str() {
        "md5" | "sha1" | "crc32" | "crc32b" => Some(normalized),
        _ => None,
    }
}

fn hex_decode(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        output.push((hex_nibble(chunk[0])? << 4) | hex_nibble(chunk[1])?);
    }
    Some(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn html_escape(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    for byte in bytes {
        match byte {
            b'&' => output.extend_from_slice(b"&amp;"),
            b'<' => output.extend_from_slice(b"&lt;"),
            b'>' => output.extend_from_slice(b"&gt;"),
            b'"' => output.extend_from_slice(b"&quot;"),
            b'\'' => output.extend_from_slice(b"&#039;"),
            _ => output.push(*byte),
        }
    }
    output
}

fn html_decode(text: &str) -> Vec<u8> {
    text.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .into_bytes()
}

fn url_encode(bytes: &[u8], raw: bool) -> Vec<u8> {
    let mut output = Vec::new();
    for byte in bytes {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_')
            || (!raw && *byte == b'.')
            || (raw && matches!(byte, b'.' | b'~'))
        {
            output.push(*byte);
        } else if !raw && *byte == b' ' {
            output.push(b'+');
        } else {
            output.extend_from_slice(format!("%{byte:02X}").as_bytes());
        }
    }
    output
}

fn url_decode(bytes: &[u8], raw: bool) -> Vec<u8> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_nibble(bytes[index + 1]), hex_nibble(bytes[index + 2]))
        {
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(if !raw && bytes[index] == b'+' {
                b' '
            } else {
                bytes[index]
            });
            index += 1;
        }
    }
    output
}

fn build_query_pairs(
    prefix: Option<String>,
    value: &Value,
    pairs: &mut Vec<String>,
) -> Result<(), BuiltinError> {
    match deref_value(value) {
        Value::Array(array) => {
            for (key, value) in array.iter() {
                let key = match key {
                    ArrayKey::Int(index) => index.to_string(),
                    ArrayKey::String(key) => key.to_string_lossy(),
                };
                let name = prefix
                    .as_ref()
                    .map_or(key.clone(), |prefix| format!("{prefix}[{key}]"));
                build_query_pairs(Some(name), value, pairs)?;
            }
        }
        Value::Null => {}
        scalar => {
            let Some(name) = prefix else {
                return Ok(());
            };
            let value = match scalar {
                Value::Bool(true) => crate::PhpString::from_test_str("1"),
                Value::Bool(false) => crate::PhpString::from_test_str("0"),
                other => string_arg("http_build_query", &other)?,
            };
            pairs.push(format!(
                "{}={}",
                String::from_utf8_lossy(&url_encode(name.as_bytes(), false)),
                String::from_utf8_lossy(&url_encode(value.as_bytes(), false))
            ));
        }
    }
    Ok(())
}

fn deref_value(value: &Value) -> Value {
    match value {
        Value::Reference(cell) => cell.get(),
        value => value.clone(),
    }
}

fn php_gettype(value: &Value) -> &'static str {
    match deref_value(value) {
        Value::Null => "NULL",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "integer",
        Value::Float(_) => "double",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => "object",
        Value::Resource(_) => "resource",
        Value::Callable(_) => "object",
        Value::Uninitialized => "NULL",
        Value::Reference(_) => unreachable!("deref_value removes references"),
    }
}

fn php_debug_type(value: &Value) -> String {
    match deref_value(value) {
        Value::Null | Value::Uninitialized => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Int(_) => "int".to_owned(),
        Value::Float(_) => "float".to_owned(),
        Value::String(_) => "string".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(object) => object.class_name(),
        Value::Resource(resource) => format!("resource ({})", resource.resource_type()),
        Value::Fiber(_) => "Fiber".to_owned(),
        Value::Generator(_) => "Generator".to_owned(),
        Value::Callable(_) => "Closure".to_owned(),
        Value::Reference(_) => unreachable!("deref_value removes references"),
    }
}

fn runtime_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) => "object",
        Value::Resource(_) => "resource",
        Value::Callable(_) => "callable",
        Value::Reference(_) => "reference",
        Value::Uninitialized => "uninitialized",
    }
}

#[derive(Default)]
struct DebugFormatter {
    active_references: BTreeSet<usize>,
}

impl DebugFormatter {
    fn write_var_dump_value(&mut self, output: &mut OutputBuffer, value: &Value, indent: usize) {
        match value {
            Value::Null | Value::Uninitialized => output.write_test_str("NULL\n"),
            Value::Bool(true) => output.write_test_str("bool(true)\n"),
            Value::Bool(false) => output.write_test_str("bool(false)\n"),
            Value::Int(value) => output.write_test_str(&format!("int({value})\n")),
            Value::Float(value) => {
                output.write_test_str(&format!("float({})\n", php_float_debug_string(*value)));
            }
            Value::String(value) => output.write_test_str(&format!(
                "string({}) \"{}\"\n",
                value.len(),
                value.to_string_lossy()
            )),
            Value::Array(array) => {
                output.write_test_str(&format!("array({}) {{\n", array.len()));
                for (key, element) in array.iter() {
                    write_indent(output, indent + 2);
                    write_array_key_dump(output, key);
                    write_indent(output, indent + 2);
                    self.write_var_dump_value(output, element, indent + 2);
                }
                write_indent(output, indent);
                output.write_test_str("}\n");
            }
            Value::Object(object) => {
                let properties = object.properties_snapshot();
                output.write_test_str(&format!(
                    "object({})#{} ({}) {{\n",
                    object.class_name(),
                    object.id(),
                    properties.len()
                ));
                for (name, property) in properties {
                    write_indent(output, indent + 2);
                    output.write_test_str(&format!("[\"{name}\"]=>\n"));
                    write_indent(output, indent + 2);
                    self.write_var_dump_value(output, &property, indent + 2);
                }
                write_indent(output, indent);
                output.write_test_str("}\n");
            }
            Value::Resource(resource) => output.write_test_str(&format!(
                "resource({}) of type ({})\n",
                resource.id().get(),
                resource.resource_type()
            )),
            Value::Fiber(_) => output.write_test_str("object(Fiber)#0 (0) {\n}\n"),
            Value::Generator(_) => output.write_test_str("object(Generator)#0 (0) {\n}\n"),
            Value::Callable(_) => output.write_test_str("object(Closure)#0 (0) {\n}\n"),
            Value::Reference(cell) => {
                let id = cell.gc_debug_id();
                if !self.active_references.insert(id) {
                    output.write_test_str("*RECURSION*\n");
                    return;
                }
                output.write_test_str("&");
                self.write_var_dump_value(output, &cell.get(), indent);
                self.active_references.remove(&id);
            }
        }
    }

    fn write_print_r_value(&mut self, output: &mut OutputBuffer, value: &Value, indent: usize) {
        match value {
            Value::Null | Value::Uninitialized | Value::Bool(false) => {}
            Value::Bool(true) => output.write_test_str("1"),
            Value::Int(value) => output.write_test_str(&value.to_string()),
            Value::Float(value) => output.write_test_str(&value.to_string()),
            Value::String(value) => output.write_php_string(value),
            Value::Array(array) => {
                output.write_test_str("Array\n");
                write_indent(output, indent);
                output.write_test_str("(\n");
                for (key, element) in array.iter() {
                    write_indent(output, indent + 4);
                    write_print_r_key(output, key);
                    output.write_test_str(" => ");
                    self.write_print_r_value(output, element, indent + 4);
                    output.write_test_str("\n");
                }
                write_indent(output, indent);
                output.write_test_str(")\n");
            }
            Value::Object(object) => {
                output.write_test_str(&format!("{} Object\n", object.class_name()));
                write_indent(output, indent);
                output.write_test_str("(\n");
                for (name, property) in object.properties_snapshot() {
                    write_indent(output, indent + 4);
                    output.write_test_str(&format!("[{name}] => "));
                    self.write_print_r_value(output, &property, indent + 4);
                    output.write_test_str("\n");
                }
                write_indent(output, indent);
                output.write_test_str(")\n");
            }
            Value::Resource(resource) => {
                output.write_test_str(&format!("Resource id #{}", resource.id().get()));
            }
            Value::Fiber(_) => output.write_test_str("Fiber Object\n(\n)\n"),
            Value::Generator(_) => output.write_test_str("Generator Object\n(\n)\n"),
            Value::Callable(_) => output.write_test_str("Closure Object\n(\n)\n"),
            Value::Reference(cell) => {
                let id = cell.gc_debug_id();
                if !self.active_references.insert(id) {
                    output.write_test_str("*RECURSION*");
                    return;
                }
                self.write_print_r_value(output, &cell.get(), indent);
                self.active_references.remove(&id);
            }
        }
    }

    fn write_var_export_value(&mut self, output: &mut OutputBuffer, value: &Value, indent: usize) {
        match value {
            Value::Null | Value::Uninitialized => output.write_test_str("NULL"),
            Value::Bool(true) => output.write_test_str("true"),
            Value::Bool(false) => output.write_test_str("false"),
            Value::Int(value) => output.write_test_str(&value.to_string()),
            Value::Float(value) => output.write_test_str(&php_float_export_string(*value)),
            Value::String(value) => write_export_string(output, &value.to_string_lossy()),
            Value::Array(array) => {
                output.write_test_str("array (\n");
                for (key, element) in array.iter() {
                    write_indent(output, indent + 2);
                    write_export_key(output, key);
                    output.write_test_str(" => ");
                    if var_export_value_starts_multiline(element) {
                        output.write_test_str("\n");
                        write_indent(output, indent + 2);
                    }
                    self.write_var_export_value(output, element, indent + 2);
                    output.write_test_str(",\n");
                }
                write_indent(output, indent);
                output.write_test_str(")");
            }
            Value::Object(object) => {
                output.write_test_str(&format!("{}::__set_state(array(\n", object.class_name()));
                for (name, property) in object.properties_snapshot() {
                    write_indent(output, indent + 2);
                    write_export_string(output, &name);
                    output.write_test_str(" => ");
                    self.write_var_export_value(output, &property, indent + 2);
                    output.write_test_str(",\n");
                }
                write_indent(output, indent);
                output.write_test_str("))");
            }
            Value::Resource(resource) => {
                output.write_test_str(&format!("NULL /* resource #{} */", resource.id().get()));
            }
            Value::Fiber(_) => output.write_test_str("Fiber::__set_state(array(\n))"),
            Value::Generator(_) => output.write_test_str("Generator::__set_state(array(\n))"),
            Value::Callable(_) => output.write_test_str("Closure::__set_state(array(\n))"),
            Value::Reference(cell) => {
                let id = cell.gc_debug_id();
                if !self.active_references.insert(id) {
                    output.write_test_str("NULL /* *RECURSION* */");
                    return;
                }
                self.write_var_export_value(output, &cell.get(), indent);
                self.active_references.remove(&id);
            }
        }
    }
}

fn write_array_key_dump(output: &mut OutputBuffer, key: &ArrayKey) {
    match key {
        ArrayKey::Int(index) => output.write_test_str(&format!("[{index}]=>\n")),
        ArrayKey::String(key) => {
            output.write_test_str(&format!("[\"{}\"]=>\n", key.to_string_lossy()))
        }
    }
}

fn var_export_value_starts_multiline(value: &Value) -> bool {
    match value {
        Value::Array(_) | Value::Object(_) => true,
        Value::Reference(cell) => var_export_value_starts_multiline(&cell.get()),
        _ => false,
    }
}

fn write_print_r_key(output: &mut OutputBuffer, key: &ArrayKey) {
    match key {
        ArrayKey::Int(index) => output.write_test_str(&format!("[{index}]")),
        ArrayKey::String(key) => output.write_test_str(&format!("[{}]", key.to_string_lossy())),
    }
}

fn write_export_key(output: &mut OutputBuffer, key: &ArrayKey) {
    match key {
        ArrayKey::Int(index) => output.write_test_str(&index.to_string()),
        ArrayKey::String(key) => write_export_string(output, &key.to_string_lossy()),
    }
}

fn write_export_string(output: &mut OutputBuffer, text: &str) {
    output.write_test_str("'");
    for character in text.chars() {
        match character {
            '\\' => output.write_test_str("\\\\"),
            '\'' => output.write_test_str("\\'"),
            _ => output.write_test_str(&character.to_string()),
        }
    }
    output.write_test_str("'");
}

fn php_float_debug_string(value: FloatValue) -> String {
    let value = value.to_f64();
    if value.is_nan() {
        return "NAN".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-INF".to_owned()
        } else {
            "INF".to_owned()
        };
    }

    let abs = value.abs();
    if abs != 0.0 && !(1e-4..1e16).contains(&abs) {
        return php_scientific_float_debug_string(value);
    }

    value.to_string()
}

fn php_float_export_string(value: FloatValue) -> String {
    let value = value.to_f64();
    if value.is_nan() {
        return "NAN".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-INF".to_owned()
        } else {
            "INF".to_owned()
        };
    }

    let mut formatted = value.to_string();
    if !formatted.contains(['.', 'E', 'e']) {
        formatted.push_str(".0");
    }
    formatted
}

fn php_scientific_float_debug_string(value: f64) -> String {
    let formatted = format!("{value:E}");
    if let Some((mantissa, exponent)) = formatted.split_once('E') {
        let sign = if exponent.starts_with('-') { "" } else { "+" };
        format!("{mantissa}E{sign}{exponent}")
    } else {
        formatted
    }
}

fn write_indent(output: &mut OutputBuffer, spaces: usize) {
    output.write_bytes(vec![b' '; spaces]);
}

#[cfg(test)]
mod tests {
    use super::{
        BuiltinCompatibility, BuiltinContext, BuiltinRegistry, JSON_ERROR_NONE, JSON_ERROR_SYNTAX,
        JSON_OBJECT_AS_ARRAY, JSON_PRESERVE_ZERO_FRACTION, JSON_PRETTY_PRINT, JSON_THROW_ON_ERROR,
        JSON_UNESCAPED_SLASHES, JSON_UNESCAPED_UNICODE, RuntimeSourceSpan,
    };
    use crate::{
        ArrayKey, ClassEntry, ClassFlags, FilesystemCapabilities, ObjectRef, OutputBuffer,
        PhpArray, PhpString, ReferenceCell, ResourceTable, StreamFlags, StreamMetadata, Value,
        datetime, pcre,
    };
    use std::path::PathBuf;

    fn call(name: &str, args: Vec<Value>, output: &mut OutputBuffer) -> Value {
        let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
        let mut context = BuiltinContext::new(output);
        (entry.function())(&mut context, args, RuntimeSourceSpan::default()).expect("builtin ok")
    }

    fn call_with_fs(
        name: &str,
        args: Vec<Value>,
        output: &mut OutputBuffer,
        cwd: PathBuf,
        filesystem: FilesystemCapabilities,
    ) -> Value {
        let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
        let mut context = BuiltinContext::with_runtime(output, cwd, filesystem, None);
        (entry.function())(&mut context, args, RuntimeSourceSpan::default()).expect("builtin ok")
    }

    fn call_with_fs_resources(
        name: &str,
        args: Vec<Value>,
        output: &mut OutputBuffer,
        cwd: PathBuf,
        filesystem: FilesystemCapabilities,
        resources: &mut ResourceTable,
    ) -> Value {
        let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
        let mut context = BuiltinContext::with_runtime(output, cwd, filesystem, Some(resources));
        (entry.function())(&mut context, args, RuntimeSourceSpan::default()).expect("builtin ok")
    }

    fn call_in_context(context: &mut BuiltinContext<'_>, name: &str, args: Vec<Value>) -> Value {
        let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
        (entry.function())(context, args, RuntimeSourceSpan::default()).expect("builtin ok")
    }

    fn array_strings(value: Value) -> Vec<String> {
        let Value::Array(array) = value else {
            panic!("expected array");
        };
        array
            .iter()
            .map(|(_, value)| match value {
                Value::String(text) => text.to_string_lossy(),
                other => panic!("expected string entry, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn builtins_registry_is_sorted_and_classified() {
        let registry = BuiltinRegistry::new();
        let names = registry
            .entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_unstable();

        assert_eq!(names, sorted);
        assert!(registry.contains("print"));
        assert!(registry.contains("strlen"));
        assert!(
            registry
                .entries()
                .iter()
                .all(|entry| entry.compatibility() == BuiltinCompatibility::Php)
        );
    }

    #[test]
    fn tokenizer_builtins_use_lexer_lexer_names_and_lines() {
        let mut output = OutputBuffer::new();
        let tokens = call(
            "token_get_all",
            vec![Value::string("<?php echo $name + 1;")],
            &mut output,
        );
        let Value::Array(tokens) = tokens else {
            panic!("expected token array");
        };
        let first = tokens.get(&ArrayKey::Int(0)).expect("open tag token");
        let Value::Array(first) = first else {
            panic!("expected named token entry");
        };
        let id = first.get(&ArrayKey::Int(0)).expect("token id").clone();
        assert_eq!(
            call("token_name", vec![id], &mut output),
            Value::string("T_OPEN_TAG")
        );
        assert_eq!(first.get(&ArrayKey::Int(1)), Some(&Value::string("<?php ")));
        assert_eq!(first.get(&ArrayKey::Int(2)), Some(&Value::Int(1)));

        let names = tokens
            .iter()
            .filter_map(|(_, value)| match value {
                Value::Array(entry) => entry.get(&ArrayKey::Int(0)).cloned(),
                _ => None,
            })
            .map(|id| call("token_name", vec![id], &mut output))
            .collect::<Vec<_>>();
        assert!(names.contains(&Value::string("T_ECHO")));
        assert!(names.contains(&Value::string("T_VARIABLE")));
        assert!(names.contains(&Value::string("T_LNUMBER")));
        assert!(
            tokens
                .iter()
                .any(|(_, value)| matches!(value, Value::String(text) if text.as_bytes() == b"+"))
        );
    }

    #[test]
    fn tokenizer_builtins_cover_modern_php_85_tokens() {
        let mut output = OutputBuffer::new();
        let tokens = call(
            "token_get_all",
            vec![Value::string(
                "<?php class C { public(set) string $name { get => $this->name; } }",
            )],
            &mut output,
        );
        let Value::Array(tokens) = tokens else {
            panic!("expected token array");
        };
        let names = tokens
            .iter()
            .filter_map(|(_, value)| match value {
                Value::Array(entry) => entry.get(&ArrayKey::Int(0)).cloned(),
                _ => None,
            })
            .map(|id| call("token_name", vec![id], &mut output))
            .collect::<Vec<_>>();
        assert!(names.contains(&Value::string("T_PUBLIC_SET")));
        assert!(names.contains(&Value::string("T_VARIABLE")));
        assert_eq!(
            call("token_name", vec![Value::Int(-1)], &mut output),
            Value::string("UNKNOWN")
        );
    }

    #[test]
    fn builtins_cover_scalar_type_queries_and_print() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("gettype", vec![Value::Int(7)], &mut output),
            Value::string("integer")
        );
        assert_eq!(
            call("is_int", vec![Value::Int(7)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_string", vec![Value::string("x")], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_bool", vec![Value::Bool(false)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_null", vec![Value::Null], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_array", vec![Value::packed_array(vec![])], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_float", vec![Value::float(1.5)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_scalar", vec![Value::string("x")], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "is_countable",
                vec![Value::packed_array(vec![])],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "is_iterable",
                vec![Value::packed_array(vec![])],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call("print", vec![Value::string("p")], &mut output),
            Value::Int(1)
        );
        assert_eq!(output.to_string_lossy(), "p");
    }

    #[test]
    fn variable_type_builtins_cover_objects_references_and_casts() {
        let mut output = OutputBuffer::new();
        let object = Value::Object(ObjectRef::new(&empty_class("DebugBox")));
        let reference = Value::Reference(ReferenceCell::new(Value::Int(42)));

        assert_eq!(
            call("get_debug_type", vec![object.clone()], &mut output),
            Value::string("DebugBox")
        );
        assert_eq!(
            call("is_object", vec![object], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("gettype", vec![reference.clone()], &mut output),
            Value::string("integer")
        );
        assert_eq!(
            call("is_int", vec![reference], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("boolval", vec![Value::string("0")], &mut output),
            Value::Bool(false)
        );
        assert_eq!(
            call("intval", vec![Value::string("12abc")], &mut output),
            Value::Int(12)
        );
        assert_eq!(
            call("floatval", vec![Value::string("1.5x")], &mut output),
            Value::float(1.5)
        );
        assert_eq!(
            call("strval", vec![Value::Bool(true)], &mut output),
            Value::string("1")
        );
    }

    #[test]
    fn resource_type_builtins_report_open_and_closed_handles() {
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();
        let resource = Value::Resource(resources.register_stream(
            StreamFlags::new(true, true, false),
            StreamMetadata::new("php", "stream", "r+", "php://memory"),
        ));

        assert_eq!(
            call("is_resource", vec![resource.clone()], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call("get_resource_id", vec![resource.clone()], &mut output),
            Value::Int(1)
        );
        assert_eq!(
            call("get_resource_type", vec![resource.clone()], &mut output),
            Value::string("stream")
        );
        assert_eq!(
            call("gettype", vec![resource.clone()], &mut output),
            Value::string("resource")
        );
        assert_eq!(
            call("get_debug_type", vec![resource.clone()], &mut output),
            Value::string("resource (stream)")
        );

        assert!(resources.close(crate::ResourceId::new(1)));
        assert!(!resources.close(crate::ResourceId::new(1)));
        assert_eq!(
            call("get_resource_type", vec![resource.clone()], &mut output),
            Value::string("Unknown")
        );
        assert_eq!(
            call("get_resource_id", vec![Value::Null], &mut output),
            Value::Bool(false)
        );
        assert_eq!(
            call("get_resource_type", vec![Value::Null], &mut output),
            Value::Bool(false)
        );
    }

    #[test]
    fn path_helpers_cover_basename_dirname_and_pathinfo() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call(
                "basename",
                vec![Value::string("/tmp/example.php"), Value::string(".php")],
                &mut output
            ),
            Value::string("example")
        );
        assert_eq!(
            call("dirname", vec![Value::string("/tmp/a/b.php")], &mut output),
            Value::string("/tmp/a")
        );
        let Value::Array(info) = call("pathinfo", vec![Value::string("/tmp/a/b.php")], &mut output)
        else {
            panic!("pathinfo should return array");
        };
        assert_eq!(
            info.get(&ArrayKey::String(PhpString::from_test_str("dirname"))),
            Some(&Value::string("/tmp/a"))
        );
        assert_eq!(
            info.get(&ArrayKey::String(PhpString::from_test_str("basename"))),
            Some(&Value::string("b.php"))
        );
        assert_eq!(
            info.get(&ArrayKey::String(PhpString::from_test_str("extension"))),
            Some(&Value::string("php"))
        );
        assert_eq!(
            info.get(&ArrayKey::String(PhpString::from_test_str("filename"))),
            Some(&Value::string("b"))
        );
    }

    #[test]
    fn stat_builtins_are_restricted_to_allowed_roots() {
        let root = std::env::temp_dir().join(format!("phrust-stdlib-stat-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create temp root");
        let file = root.join("fixture.txt");
        std::fs::write(&file, b"fixture").expect("write fixture");
        let mut output = OutputBuffer::new();

        assert_eq!(
            call_with_fs(
                "file_exists",
                vec![Value::string(file.to_string_lossy().as_bytes().to_vec())],
                &mut output,
                PathBuf::from("."),
                FilesystemCapabilities::none()
            ),
            Value::Bool(false)
        );

        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        assert_eq!(
            call_with_fs(
                "file_exists",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "is_file",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "is_dir",
                vec![Value::string(".")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "filesize",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Int(7)
        );
        assert_eq!(
            call_with_fs(
                "filetype",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::string("file")
        );
        assert!(matches!(
            call_with_fs(
                "stat",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Array(_)
        ));
        assert!(matches!(
            call_with_fs(
                "realpath",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities
            ),
            Value::String(_)
        ));
        assert_eq!(call("clearstatcache", Vec::new(), &mut output), Value::Null);

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(root);
    }

    #[test]
    fn file_handle_builtins_cover_read_write_seek_and_modes() {
        let root =
            std::env::temp_dir().join(format!("phrust-stdlib-fileio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create temp root");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();

        let handle = call_with_fs_resources(
            "fopen",
            vec![Value::string("data.txt"), Value::string("w+")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(handle, Value::Resource(_)));
        assert_eq!(
            call_with_fs_resources(
                "fwrite",
                vec![handle.clone(), Value::string("alpha\nbeta")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(10)
        );
        assert_eq!(
            call_with_fs_resources(
                "rewind",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "fgets",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("alpha\n")
        );
        assert_eq!(
            call_with_fs_resources(
                "fgetc",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("b")
        );
        assert_eq!(
            call_with_fs_resources(
                "fseek",
                vec![handle.clone(), Value::Int(0)],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(0)
        );
        assert_eq!(
            call_with_fs_resources(
                "fread",
                vec![handle.clone(), Value::Int(5)],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("alpha")
        );
        assert_eq!(
            call_with_fs_resources(
                "ftell",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(5)
        );
        assert_eq!(
            call_with_fs_resources(
                "fread",
                vec![handle.clone(), Value::Int(99)],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("\nbeta")
        );
        assert_eq!(
            call_with_fs_resources(
                "feof",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "fflush",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "fclose",
                vec![handle],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );

        let readable = call_with_fs_resources(
            "fopen",
            vec![Value::string("data.txt"), Value::string("r")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(readable, Value::Resource(_)));
        assert_eq!(
            call_with_fs_resources(
                "fclose",
                vec![readable],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );

        assert_eq!(
            call_with_fs(
                "file_put_contents",
                vec![Value::string("append.txt"), Value::string("one")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Int(3)
        );
        let append = call_with_fs_resources(
            "fopen",
            vec![Value::string("append.txt"), Value::string("a+")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert_eq!(
            call_with_fs_resources(
                "fwrite",
                vec![append.clone(), Value::string("two")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(3)
        );
        assert_eq!(
            call_with_fs_resources(
                "fclose",
                vec![append],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "file_get_contents",
                vec![Value::string("append.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::string("onetwo")
        );

        assert_eq!(
            call_with_fs_resources(
                "fopen",
                vec![Value::string("append.txt"), Value::string("x")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(false)
        );
        let exclusive = call_with_fs_resources(
            "fopen",
            vec![Value::string("exclusive.txt"), Value::string("x")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(exclusive, Value::Resource(_)));
        assert_eq!(
            call_with_fs_resources(
                "fclose",
                vec![exclusive],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );

        assert_eq!(
            call_with_fs(
                "file_put_contents",
                vec![Value::string("create.txt"), Value::string("keep")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Int(4)
        );
        let create = call_with_fs_resources(
            "fopen",
            vec![Value::string("create.txt"), Value::string("c+")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(create, Value::Resource(_)));
        assert_eq!(
            call_with_fs_resources(
                "fclose",
                vec![create],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "file_get_contents",
                vec![Value::string("create.txt")],
                &mut output,
                root.clone(),
                capabilities,
            ),
            Value::string("keep")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn file_operations_are_root_constrained_and_return_false() {
        let root =
            std::env::temp_dir().join(format!("phrust-stdlib-fileops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create temp root");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();

        assert_eq!(
            call_with_fs(
                "file_get_contents",
                vec![Value::string(
                    root.join("outside.txt")
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec()
                )],
                &mut output,
                PathBuf::from("."),
                FilesystemCapabilities::none(),
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call_with_fs_resources(
                "fopen",
                vec![Value::string("../escape.txt"), Value::string("w")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(false)
        );

        assert_eq!(
            call_with_fs(
                "file_put_contents",
                vec![Value::string("fixture.txt"), Value::string("hello")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Int(5)
        );
        assert_eq!(
            call_with_fs(
                "file_get_contents",
                vec![Value::string("fixture.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::string("hello")
        );

        let mut read_output = OutputBuffer::new();
        assert_eq!(
            call_with_fs(
                "readfile",
                vec![Value::string("fixture.txt")],
                &mut read_output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Int(5)
        );
        assert_eq!(read_output.to_string_lossy(), "hello");

        assert_eq!(
            call_with_fs(
                "copy",
                vec![Value::string("fixture.txt"), Value::string("copy.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "rename",
                vec![Value::string("copy.txt"), Value::string("renamed.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "touch",
                vec![Value::string("touched.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "mkdir",
                vec![Value::string("nested")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "rmdir",
                vec![Value::string("nested")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs(
                "unlink",
                vec![Value::string("renamed.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            ),
            Value::Bool(true)
        );

        let temp_path = call_with_fs(
            "tempnam",
            vec![Value::string("."), Value::string("pre")],
            &mut output,
            root.clone(),
            capabilities.clone(),
        );
        assert!(matches!(temp_path, Value::String(_)));
        let tmp_handle = call_with_fs_resources(
            "tmpfile",
            Vec::new(),
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(tmp_handle, Value::Resource(_)));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn directory_handles_read_rewind_and_close_with_sorted_entries() {
        let root = std::env::temp_dir().join(format!("phrust-stdlib-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(root.join("b.log"), b"b").expect("write fixture");
        std::fs::write(root.join("a.txt"), b"a").expect("write fixture");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();

        let handle = call_with_fs_resources(
            "opendir",
            vec![Value::string(".")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert!(matches!(handle, Value::Resource(_)));
        assert_eq!(
            call_with_fs_resources(
                "readdir",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string(".")
        );
        assert_eq!(
            call_with_fs_resources(
                "readdir",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("..")
        );
        assert_eq!(
            call_with_fs_resources(
                "readdir",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("a.txt")
        );
        assert_eq!(
            call_with_fs_resources(
                "rewinddir",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "readdir",
                vec![handle.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string(".")
        );
        assert_eq!(
            call_with_fs_resources(
                "closedir",
                vec![handle],
                &mut output,
                root.clone(),
                capabilities,
                &mut resources,
            ),
            Value::Bool(true)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scandir_glob_and_directory_capabilities_are_normalized() {
        let root = std::env::temp_dir().join(format!("phrust-stdlib-glob-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("nested")).expect("create temp root");
        std::fs::write(root.join("b.log"), b"b").expect("write fixture");
        std::fs::write(root.join("a.txt"), b"a").expect("write fixture");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();

        assert_eq!(
            call_with_fs_resources(
                "opendir",
                vec![Value::string(root.to_string_lossy().as_bytes().to_vec())],
                &mut output,
                PathBuf::from("."),
                FilesystemCapabilities::none(),
                &mut resources,
            ),
            Value::Bool(false)
        );
        assert_eq!(
            array_strings(call_with_fs(
                "scandir",
                vec![Value::string(".")],
                &mut output,
                root.clone(),
                capabilities.clone(),
            )),
            vec![".", "..", "a.txt", "b.log", "nested"]
        );
        assert_eq!(
            array_strings(call_with_fs(
                "scandir",
                vec![Value::string("."), Value::Int(1)],
                &mut output,
                root.clone(),
                capabilities.clone(),
            )),
            vec!["nested", "b.log", "a.txt", "..", "."]
        );
        let globbed = array_strings(call_with_fs(
            "glob",
            vec![Value::string("*.txt")],
            &mut output,
            root.clone(),
            capabilities,
        ));
        assert_eq!(globbed.len(), 1);
        assert!(globbed[0].ends_with("a.txt"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn getcwd_and_chdir_are_request_local_to_builtin_context() {
        let root = std::env::temp_dir().join(format!("phrust-stdlib-cwd-{}", std::process::id()));
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).expect("create temp root");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut context =
            BuiltinContext::with_runtime(&mut output, root.clone(), capabilities, None);

        assert_eq!(
            call_in_context(&mut context, "getcwd", Vec::new()),
            Value::string(root.to_string_lossy().as_bytes().to_vec())
        );
        assert_eq!(
            call_in_context(&mut context, "chdir", vec![Value::string("nested")]),
            Value::Bool(true)
        );
        assert_eq!(
            call_in_context(&mut context, "getcwd", Vec::new()),
            Value::string(nested.to_string_lossy().as_bytes().to_vec())
        );
        assert_eq!(
            call_in_context(&mut context, "chdir", vec![Value::string("../..")]),
            Value::Bool(false)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stream_metadata_contents_copy_and_local_checks_are_capability_aware() {
        let root =
            std::env::temp_dir().join(format!("phrust-stdlib-streams-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create temp root");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();

        assert_eq!(
            array_strings(call("stream_get_wrappers", Vec::new(), &mut output)),
            vec!["file".to_string(), "php".to_string()]
        );

        let source = call_with_fs_resources(
            "fopen",
            vec![Value::string("php://memory"), Value::string("w+")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        let destination = call_with_fs_resources(
            "fopen",
            vec![Value::string("php://memory"), Value::string("w+")],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        );
        assert_eq!(
            call_with_fs_resources(
                "fwrite",
                vec![source.clone(), Value::string("abcdef")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(6)
        );
        assert_eq!(
            call_with_fs_resources(
                "stream_get_contents",
                vec![source.clone(), Value::Int(3), Value::Int(2)],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("cde")
        );
        assert_eq!(
            call_with_fs_resources(
                "stream_copy_to_stream",
                vec![
                    source.clone(),
                    destination.clone(),
                    Value::Int(4),
                    Value::Int(0)
                ],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Int(4)
        );
        assert_eq!(
            call_with_fs_resources(
                "rewind",
                vec![destination.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "stream_get_contents",
                vec![destination.clone()],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::string("abcd")
        );

        let Value::Array(metadata) = call_with_fs_resources(
            "stream_get_meta_data",
            vec![source.clone()],
            &mut output,
            root.clone(),
            capabilities.clone(),
            &mut resources,
        ) else {
            panic!("expected metadata array");
        };
        assert_eq!(
            metadata.get(&ArrayKey::String(PhpString::from_test_str("wrapper_type"))),
            Some(&Value::string("PHP"))
        );
        assert_eq!(
            metadata.get(&ArrayKey::String(PhpString::from_test_str("stream_type"))),
            Some(&Value::string("MEMORY"))
        );

        assert_eq!(
            call_with_fs_resources(
                "stream_is_local",
                vec![Value::string("https://example.test/file.txt")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call_with_fs_resources(
                "stream_is_local",
                vec![Value::string("php://memory")],
                &mut output,
                root.clone(),
                capabilities.clone(),
                &mut resources,
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_with_fs_resources(
                "stream_isatty",
                vec![source],
                &mut output,
                root.clone(),
                capabilities,
                &mut resources,
            ),
            Value::Bool(false)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn stream_context_options_and_include_path_resolution_are_preserved() {
        let root = std::env::temp_dir().join(format!(
            "phrust-stdlib-stream-context-{}",
            std::process::id()
        ));
        let lib = root.join("lib");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&lib).expect("create include dir");
        std::fs::write(lib.join("Foo.php"), b"<?php").expect("write include fixture");
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        let mut output = OutputBuffer::new();
        let mut resources = ResourceTable::new();
        let mut context = BuiltinContext::with_runtime(
            &mut output,
            root.clone(),
            capabilities.clone(),
            Some(&mut resources),
        );
        context.set_include_path(vec![PathBuf::from("lib")]);

        let stream_context = call_in_context(&mut context, "stream_context_create", Vec::new());
        assert!(matches!(stream_context, Value::Resource(_)));
        assert_eq!(
            call_in_context(
                &mut context,
                "stream_context_set_option",
                vec![
                    stream_context.clone(),
                    Value::string("http"),
                    Value::string("timeout"),
                    Value::Int(5),
                ],
            ),
            Value::Bool(true)
        );
        let Value::Array(options) = call_in_context(
            &mut context,
            "stream_context_get_options",
            vec![stream_context.clone()],
        ) else {
            panic!("expected context options");
        };
        let Some(Value::Array(http_options)) =
            options.get(&ArrayKey::String(PhpString::from_test_str("http")))
        else {
            panic!("expected http options");
        };
        assert_eq!(
            http_options.get(&ArrayKey::String(PhpString::from_test_str("timeout"))),
            Some(&Value::Int(5))
        );

        let resolved = call_in_context(
            &mut context,
            "stream_resolve_include_path",
            vec![Value::string("Foo.php")],
        );
        let Value::String(path) = resolved else {
            panic!("expected resolved include path");
        };
        assert!(path.to_string_lossy().ends_with("lib/Foo.php"));
        assert_eq!(
            call_in_context(
                &mut context,
                "stream_resolve_include_path",
                vec![Value::string("../escape.php")],
            ),
            Value::Bool(false)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preg_match_and_match_all_capture_offsets_and_modifiers() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        let matches = ReferenceCell::new(Value::Null);
        assert_eq!(
            call_in_context(
                &mut context,
                "preg_match",
                vec![
                    Value::string(r#"/([a-z]+)-(\d+)/i"#),
                    Value::string("ABC-123"),
                    Value::Reference(matches.clone()),
                    Value::Int(pcre::PREG_OFFSET_CAPTURE),
                ],
            ),
            Value::Int(1)
        );
        let Value::Array(captures) = matches.get() else {
            panic!("expected captures array");
        };
        assert_eq!(
            captures.get(&ArrayKey::Int(0)),
            Some(&Value::packed_array(vec![
                Value::string("ABC-123"),
                Value::Int(0)
            ]))
        );
        assert_eq!(
            captures.get(&ArrayKey::Int(2)),
            Some(&Value::packed_array(vec![
                Value::string("123"),
                Value::Int(4)
            ]))
        );

        let all = ReferenceCell::new(Value::Null);
        assert_eq!(
            call_in_context(
                &mut context,
                "preg_match_all",
                vec![
                    Value::string(r#"/([a-z]+)=(\d+)/i"#),
                    Value::string("A=1 b=22"),
                    Value::Reference(all.clone()),
                    Value::Int(pcre::PREG_SET_ORDER | pcre::PREG_OFFSET_CAPTURE),
                ],
            ),
            Value::Int(2)
        );
        let Value::Array(rows) = all.get() else {
            panic!("expected match rows");
        };
        assert_eq!(rows.len(), 2);
        assert_eq!(
            call_in_context(&mut context, "preg_last_error", Vec::new()),
            Value::Int(pcre::PREG_NO_ERROR)
        );
    }

    #[test]
    fn preg_replace_split_grep_quote_callback_and_errors_are_pcre2_backed() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        let count = ReferenceCell::new(Value::Null);
        assert_eq!(
            call_in_context(
                &mut context,
                "preg_replace",
                vec![
                    Value::string(r#"/([a-z]+)=(\d+)/"#),
                    Value::string(r#"$1:$2"#),
                    Value::string("a=1 b=22"),
                    Value::Int(-1),
                    Value::Reference(count.clone()),
                ],
            ),
            Value::string("a:1 b:22")
        );
        assert_eq!(count.get(), Value::Int(2));

        assert_eq!(
            call_in_context(
                &mut context,
                "preg_replace_callback",
                vec![
                    Value::string(r#"/(foo)/"#),
                    Value::internal_builtin_callable("count"),
                    Value::string("foo foo"),
                ],
            ),
            Value::string("2 2")
        );

        assert_eq!(
            array_strings(call_in_context(
                &mut context,
                "preg_split",
                vec![
                    Value::string(r#"/[,;]\s*/"#),
                    Value::string("a, b; c"),
                    Value::Int(-1),
                    Value::Int(pcre::PREG_SPLIT_NO_EMPTY),
                ],
            )),
            ["a", "b", "c"]
        );

        let input = Value::packed_array(vec![
            Value::string("src/Foo.php"),
            Value::string("README.md"),
            Value::string("tests/FooTest.php"),
        ]);
        assert_eq!(
            array_strings(call_in_context(
                &mut context,
                "preg_grep",
                vec![Value::string(r#"/\.php$/"#), input],
            )),
            ["src/Foo.php", "tests/FooTest.php"]
        );

        assert_eq!(
            call_in_context(
                &mut context,
                "preg_quote",
                vec![Value::string("a+b/c"), Value::string("/")],
            ),
            Value::string(r#"a\+b\/c"#)
        );

        assert_eq!(
            call_in_context(
                &mut context,
                "preg_match",
                vec![Value::string("/["), Value::string("x")],
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call_in_context(&mut context, "preg_last_error", Vec::new()),
            Value::Int(pcre::PREG_INTERNAL_ERROR)
        );
        assert_eq!(
            call_in_context(&mut context, "preg_last_error_msg", Vec::new()),
            Value::string("No ending delimiter found")
        );
    }

    #[test]
    fn date_timezone_defaults_set_and_list_are_request_local() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        assert_eq!(
            call_in_context(&mut context, "date_default_timezone_get", Vec::new()),
            Value::string("UTC")
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "date_default_timezone_set",
                vec![Value::string("Europe/Berlin")],
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_in_context(&mut context, "date_default_timezone_get", Vec::new()),
            Value::string("Europe/Berlin")
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "date_default_timezone_set",
                vec![Value::string("Mars/Base")],
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call_in_context(&mut context, "date_default_timezone_get", Vec::new()),
            Value::string("Europe/Berlin")
        );

        let identifiers = array_strings(call_in_context(
            &mut context,
            "timezone_identifiers_list",
            Vec::new(),
        ));
        assert!(identifiers.contains(&"UTC".to_string()));
        assert!(identifiers.contains(&"Europe/Berlin".to_string()));
    }

    #[test]
    fn date_functions_parse_format_and_use_request_timezone() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        assert_eq!(
            call_in_context(
                &mut context,
                "date",
                vec![Value::string("Y-m-d H:i:s O"), Value::Int(0)],
            ),
            Value::string("1970-01-01 00:00:00 +0000")
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "date_default_timezone_set",
                vec![Value::string("Europe/Berlin")],
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "date",
                vec![Value::string("Y-m-d H:i:s T"), Value::Int(0)],
            ),
            Value::string("1970-01-01 01:00:00 CET")
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "strtotime",
                vec![Value::string("2024-01-02 03:04:05")],
            ),
            Value::Int(1_704_164_645)
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "strtotime",
                vec![Value::string("+2 days"), Value::Int(0)],
            ),
            Value::Int(172_800)
        );
        assert!(matches!(
            call_in_context(&mut context, "time", Vec::new()),
            Value::Int(value) if value > 0
        ));
    }

    #[test]
    fn spl_object_identity_builtins_use_stable_runtime_object_ids() {
        let mut output = OutputBuffer::new();
        let object = Value::Object(ObjectRef::new(&empty_class("SplBox")));

        let Value::Int(id) = call("spl_object_id", vec![object.clone()], &mut output) else {
            panic!("expected object id int");
        };
        assert!(id > 0);
        assert_eq!(
            call("spl_object_id", vec![object.clone()], &mut output),
            Value::Int(id)
        );
        assert_eq!(
            call("spl_object_hash", vec![object], &mut output),
            Value::string(format!("{id:032x}"))
        );
    }

    #[test]
    fn datetime_objects_cover_mutable_immutable_interval_and_diff_mvp() {
        let Value::Object(datetime) = datetime::datetime_object(0, "UTC") else {
            panic!("expected DateTime object");
        };
        assert_eq!(datetime.class_name(), "DateTime");
        assert_eq!(
            datetime::format_timestamp(
                datetime::object_timestamp(&datetime).expect("timestamp"),
                &datetime::object_timezone(&datetime).expect("timezone"),
                "Y-m-d H:i:s"
            ),
            "1970-01-01 00:00:00"
        );

        let updated = datetime::with_timestamp(&datetime, 60, false);
        assert!(matches!(updated, Value::Object(_)));
        assert_eq!(datetime::object_timestamp(&datetime), Some(60));

        let Value::Object(immutable) = datetime::datetime_immutable_object(0, "UTC") else {
            panic!("expected DateTimeImmutable object");
        };
        let changed = datetime::with_timestamp(&immutable, 60, true);
        let Value::Object(changed) = changed else {
            panic!("expected changed immutable object");
        };
        assert_eq!(datetime::object_timestamp(&immutable), Some(0));
        assert_eq!(datetime::object_timestamp(&changed), Some(60));
        assert_eq!(changed.class_name(), "DateTimeImmutable");

        let interval_seconds = datetime::parse_interval_spec("P1DT2H").expect("interval");
        assert_eq!(interval_seconds, 93_600);
        let added = datetime::add_interval(&immutable, interval_seconds, true);
        let Value::Object(added) = added else {
            panic!("expected DateTimeImmutable after add");
        };
        assert_eq!(datetime::object_timestamp(&added), Some(93_600));
        let diff = datetime::diff_objects(&immutable, &added);
        let Value::Object(diff) = diff else {
            panic!("expected DateInterval object");
        };
        assert_eq!(diff.class_name(), "DateInterval");
        assert_eq!(diff.get_property("__seconds"), Some(Value::Int(93_600)));

        let modified = datetime::modify_object(&immutable, "+1 day", true).expect("modify");
        let Value::Object(modified) = modified else {
            panic!("expected modified object");
        };
        assert_eq!(datetime::object_timestamp(&modified), Some(86_400));
        assert!(datetime::modify_object(&immutable, "next tuesday", true).is_none());
    }

    #[test]
    fn json_builtins_cover_composer_style_documents_and_modes() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        let decoded = call_in_context(
            &mut context,
            "json_decode",
            vec![
                Value::string(r#"{"name":"pkg","autoload":{"psr-4":{"App\\":"src/"}}}"#),
                Value::Bool(true),
            ],
        );
        let Value::Array(root) = decoded else {
            panic!("expected associative json array");
        };
        assert_eq!(
            root.get(&ArrayKey::String(PhpString::from_test_str("name"))),
            Some(&Value::string("pkg"))
        );
        assert!(matches!(
            root.get(&ArrayKey::String(PhpString::from_test_str("autoload"))),
            Some(Value::Array(_))
        ));

        let object = call_in_context(
            &mut context,
            "json_decode",
            vec![Value::string(r#"{"answer":42}"#)],
        );
        let Value::Object(object) = object else {
            panic!("expected stdClass object");
        };
        assert_eq!(object.class_name(), "stdClass");
        assert_eq!(object.get_property("answer"), Some(Value::Int(42)));

        let decoded_with_flag = call_in_context(
            &mut context,
            "json_decode",
            vec![
                Value::string(r#"{"answer":42}"#),
                Value::Null,
                Value::Int(512),
                Value::Int(JSON_OBJECT_AS_ARRAY),
            ],
        );
        assert!(matches!(decoded_with_flag, Value::Array(_)));

        let mut mixed = crate::PhpArray::new();
        mixed.insert(
            ArrayKey::String(PhpString::from_test_str("name")),
            Value::string("pkg"),
        );
        mixed.insert(
            ArrayKey::String(PhpString::from_test_str("versions")),
            Value::packed_array(vec![Value::string("1.0.0"), Value::string("1.1.0")]),
        );
        assert_eq!(
            call_in_context(&mut context, "json_encode", vec![Value::Array(mixed)]),
            Value::string(r#"{"name":"pkg","versions":["1.0.0","1.1.0"]}"#)
        );
        assert_eq!(
            call_in_context(&mut context, "json_encode", vec![Value::float(42.0)]),
            Value::string("42")
        );
        let flags = JSON_PRETTY_PRINT
            | JSON_UNESCAPED_SLASHES
            | JSON_UNESCAPED_UNICODE
            | JSON_PRESERVE_ZERO_FRACTION;
        let encoded_with_flags = call_in_context(
            &mut context,
            "json_encode",
            vec![
                Value::packed_array(vec![
                    Value::string("https://example.test/ü"),
                    Value::float(1.0),
                ]),
                Value::Int(flags),
            ],
        );
        let Value::String(encoded_with_flags) = encoded_with_flags else {
            panic!("expected encoded JSON string");
        };
        let encoded_with_flags = encoded_with_flags.to_string_lossy();
        assert!(encoded_with_flags.contains('\n'));
        assert!(encoded_with_flags.contains("https://example.test/ü"));
        assert!(encoded_with_flags.contains("1.0"));
        assert_eq!(
            call_in_context(&mut context, "json_last_error", Vec::new()),
            Value::Int(JSON_ERROR_NONE)
        );
        assert_eq!(
            call_in_context(
                &mut context,
                "json_validate",
                vec![Value::string("[1,2,3]")]
            ),
            Value::Bool(true)
        );
    }

    #[test]
    fn json_errors_are_recorded_and_throw_flag_errors() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        assert_eq!(
            call_in_context(&mut context, "json_decode", vec![Value::string("{")]),
            Value::Bool(false)
        );
        assert_eq!(
            call_in_context(&mut context, "json_last_error", Vec::new()),
            Value::Int(JSON_ERROR_SYNTAX)
        );
        assert_eq!(
            call_in_context(&mut context, "json_last_error_msg", Vec::new()),
            Value::string("Syntax error")
        );
        assert_eq!(
            call_in_context(&mut context, "json_validate", vec![Value::string("{")]),
            Value::Bool(false)
        );

        let entry = BuiltinRegistry::new()
            .get("json_decode")
            .expect("json_decode exists");
        let result = (entry.function())(
            &mut context,
            vec![
                Value::string("{"),
                Value::Null,
                Value::Int(512),
                Value::Int(JSON_THROW_ON_ERROR),
            ],
            RuntimeSourceSpan::default(),
        );
        assert!(matches!(
            result,
            Err(error) if error.diagnostic_id() == "E_PHP_RUNTIME_JSON_EXCEPTION"
        ));
    }

    #[test]
    fn symlink_stat_is_conditional_on_platform_support() {
        let root = std::env::temp_dir().join(format!("phrust-stdlib-lstat-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create temp root");
        let target = root.join("target.txt");
        let link = root.join("link.txt");
        std::fs::write(&target, b"target").expect("write target");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_file(&target, &link).is_err() {
                let _ = std::fs::remove_file(target);
                let _ = std::fs::remove_dir(root);
                return;
            }
        }

        let mut output = OutputBuffer::new();
        let capabilities = FilesystemCapabilities::none().with_allowed_roots(vec![root.clone()]);
        assert_eq!(
            call_with_fs(
                "is_link",
                vec![Value::string("link.txt")],
                &mut output,
                root.clone(),
                capabilities.clone()
            ),
            Value::Bool(true)
        );
        assert!(matches!(
            call_with_fs(
                "lstat",
                vec![Value::string("link.txt")],
                &mut output,
                root.clone(),
                capabilities
            ),
            Value::Array(_)
        ));

        let _ = std::fs::remove_file(link);
        let _ = std::fs::remove_file(target);
        let _ = std::fs::remove_dir(root);
    }

    fn empty_class(name: &str) -> ClassEntry {
        ClassEntry {
            name: name.to_owned(),
            parent: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            constants: Vec::new(),
            enum_cases: Vec::new(),
            attributes: Vec::new(),
            enum_backing_type: None,
            constructor_id: None,
            flags: ClassFlags::default(),
        }
    }

    #[test]
    fn builtins_var_dump_is_stable_for_scalars_and_arrays() {
        let mut output = OutputBuffer::new();
        let result = call(
            "var_dump",
            vec![
                Value::Null,
                Value::Bool(true),
                Value::Int(7),
                Value::float(1.0),
                Value::float(f64::INFINITY),
                Value::float(f64::NAN),
                Value::float(9_223_372_036_854_776_000.0),
                Value::string("hi"),
                Value::packed_array(vec![Value::Int(1), Value::string("x")]),
            ],
            &mut output,
        );

        assert_eq!(result, Value::Null);
        assert_eq!(
            output.to_string_lossy(),
            "NULL\nbool(true)\nint(7)\nfloat(1)\nfloat(INF)\nfloat(NAN)\nfloat(9.223372036854776E+18)\nstring(2) \"hi\"\narray(2) {\n  [0]=>\n  int(1)\n  [1]=>\n  string(1) \"x\"\n}\n"
        );
    }

    #[test]
    fn debug_output_builtins_cover_return_modes_and_cycles() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call(
                "print_r",
                vec![Value::packed_array(vec![Value::Int(1)]), Value::Bool(true)],
                &mut output
            ),
            Value::string("Array\n(\n    [0] => 1\n)\n")
        );
        assert_eq!(
            call(
                "var_export",
                vec![
                    Value::packed_array(vec![Value::string("x")]),
                    Value::Bool(true)
                ],
                &mut output
            ),
            Value::string("array (\n  0 => 'x',\n)")
        );
        assert_eq!(
            call(
                "var_export",
                vec![
                    Value::packed_array(vec![Value::packed_array(vec![Value::Int(1)])]),
                    Value::Bool(true)
                ],
                &mut output
            ),
            Value::string("array (\n  0 => \n  array (\n    0 => 1,\n  ),\n)")
        );
        assert_eq!(
            call(
                "var_export",
                vec![Value::float(1.0), Value::Bool(true)],
                &mut output
            ),
            Value::string("1.0")
        );
        assert_eq!(
            call(
                "var_export",
                vec![Value::float(-0.0), Value::Bool(true)],
                &mut output
            ),
            Value::string("-0.0")
        );
        assert_eq!(
            call(
                "var_export",
                vec![Value::float(10_000_000_000_000_000.0), Value::Bool(true)],
                &mut output
            ),
            Value::string("10000000000000000.0")
        );

        let cell = ReferenceCell::new(Value::Null);
        let mut array = PhpArray::new();
        array.append(Value::Reference(cell.clone()));
        cell.set(Value::Array(array));

        let result = call("var_dump", vec![Value::Reference(cell)], &mut output);
        assert_eq!(result, Value::Null);
        assert!(output.to_string_lossy().contains("*RECURSION*"));
    }

    #[test]
    fn version_compare_covers_platform_check_semantics() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call(
                "version_compare",
                vec![Value::string("8.5.7"), Value::string("8.5.0")],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "version_compare",
                vec![
                    Value::string("8.5.7"),
                    Value::string("8.5.7"),
                    Value::string("eq")
                ],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "version_compare",
                vec![
                    Value::string("8.5.7-dev"),
                    Value::string("8.5.7"),
                    Value::string("<")
                ],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "version_compare",
                vec![
                    Value::string("8.5.7RC1"),
                    Value::string("8.5.7"),
                    Value::string("lt")
                ],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "version_compare",
                vec![
                    Value::string("8.5.7pl1"),
                    Value::string("8.5.7"),
                    Value::string("gt")
                ],
                &mut output
            ),
            Value::Bool(true)
        );
    }

    #[test]
    fn string_search_and_compare_builtins_are_binary_safe() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("strlen", vec![Value::string(b"a\0b".to_vec())], &mut output),
            Value::Int(3)
        );
        assert_eq!(
            call(
                "substr",
                vec![Value::string("abcdef"), Value::Int(-3), Value::Int(2)],
                &mut output
            ),
            Value::string("de")
        );
        assert_eq!(
            call(
                "strpos",
                vec![
                    Value::string(b"a\0b\0c".to_vec()),
                    Value::string(b"\0b".to_vec())
                ],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "stripos",
                vec![Value::string("AbCd"), Value::string("bc")],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "strrpos",
                vec![Value::string("abcabc"), Value::string("a"), Value::Int(-1)],
                &mut output
            ),
            Value::Int(3)
        );
        assert_eq!(
            call(
                "str_contains",
                vec![Value::string("abc"), Value::string("")],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "str_starts_with",
                vec![Value::string("abc"), Value::string("ab")],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "str_ends_with",
                vec![Value::string("abc"), Value::string("bc")],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "strcmp",
                vec![Value::string("a"), Value::string("b")],
                &mut output
            ),
            Value::Int(-1)
        );
        assert_eq!(
            call(
                "strncmp",
                vec![Value::string("abc"), Value::string("abd"), Value::Int(2)],
                &mut output
            ),
            Value::Int(0)
        );
        assert_eq!(
            call(
                "strcasecmp",
                vec![Value::string("ABC"), Value::string("abc")],
                &mut output
            ),
            Value::Int(0)
        );
        assert_eq!(
            call(
                "strncasecmp",
                vec![Value::string("ABx"), Value::string("aby"), Value::Int(2)],
                &mut output
            ),
            Value::Int(0)
        );
    }

    #[test]
    fn string_builtins_report_value_errors() {
        for (name, args) in [
            (
                "strpos",
                vec![Value::string("abc"), Value::string("a"), Value::Int(4)],
            ),
            (
                "strncmp",
                vec![Value::string("a"), Value::string("a"), Value::Int(-1)],
            ),
        ] {
            let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
            let mut output = OutputBuffer::new();
            let mut context = BuiltinContext::new(&mut output);
            let error = (entry.function())(&mut context, args, RuntimeSourceSpan::default())
                .expect_err("expected value error");
            assert_eq!(error.diagnostic_id(), "E_PHP_RUNTIME_BUILTIN_VALUE");
        }
    }

    #[test]
    fn string_split_replace_case_and_padding_builtins_work() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call(
                "explode",
                vec![Value::string(","), Value::string("a,b,c")],
                &mut output
            ),
            Value::packed_array(vec![
                Value::string("a"),
                Value::string("b"),
                Value::string("c")
            ])
        );
        assert_eq!(
            call(
                "implode",
                vec![
                    Value::string("|"),
                    Value::packed_array(vec![Value::string("a"), Value::string("b")]),
                ],
                &mut output,
            ),
            Value::string("a|b")
        );
        assert_eq!(
            call(
                "str_replace",
                vec![
                    Value::packed_array(vec![Value::string("a"), Value::string("b")]),
                    Value::packed_array(vec![Value::string("x"), Value::string("y")]),
                    Value::string("abca"),
                ],
                &mut output,
            ),
            Value::string("xycx")
        );
        assert_eq!(
            call(
                "strtr",
                vec![
                    Value::string("abc"),
                    Value::string("ab"),
                    Value::string("xy")
                ],
                &mut output
            ),
            Value::string("xyc")
        );
        assert_eq!(
            call("trim", vec![Value::string(" x ")], &mut output),
            Value::string("x")
        );
        assert_eq!(
            call("ltrim", vec![Value::string(" x ")], &mut output),
            Value::string("x ")
        );
        assert_eq!(
            call("rtrim", vec![Value::string(" x ")], &mut output),
            Value::string(" x")
        );
        assert_eq!(
            call("strtolower", vec![Value::string("AbC")], &mut output),
            Value::string("abc")
        );
        assert_eq!(
            call("strtoupper", vec![Value::string("AbC")], &mut output),
            Value::string("ABC")
        );
        assert_eq!(
            call("ucfirst", vec![Value::string("abc")], &mut output),
            Value::string("Abc")
        );
        assert_eq!(
            call("lcfirst", vec![Value::string("Abc")], &mut output),
            Value::string("abc")
        );
        assert_eq!(
            call("ucwords", vec![Value::string("a b")], &mut output),
            Value::string("A B")
        );
        assert_eq!(
            call(
                "str_repeat",
                vec![Value::string("ab"), Value::Int(3)],
                &mut output
            ),
            Value::string("ababab")
        );
        assert_eq!(
            call(
                "str_pad",
                vec![
                    Value::string("x"),
                    Value::Int(3),
                    Value::string("0"),
                    Value::Int(0)
                ],
                &mut output,
            ),
            Value::string("00x")
        );
        assert_eq!(
            call("strrev", vec![Value::string("abc")], &mut output),
            Value::string("cba")
        );
    }

    #[test]
    fn string_split_replace_reports_value_errors() {
        for (name, args) in [
            ("explode", vec![Value::string(""), Value::string("abc")]),
            ("str_repeat", vec![Value::string("x"), Value::Int(-1)]),
            (
                "str_pad",
                vec![Value::string("x"), Value::Int(3), Value::string("")],
            ),
        ] {
            let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
            let mut output = OutputBuffer::new();
            let mut context = BuiltinContext::new(&mut output);
            let error = (entry.function())(&mut context, args, RuntimeSourceSpan::default())
                .expect_err("expected value error");
            assert_eq!(error.diagnostic_id(), "E_PHP_RUNTIME_BUILTIN_VALUE");
        }
    }

    #[test]
    fn encoding_hash_html_and_url_builtins_cover_mvp_paths() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("bin2hex", vec![Value::string("Hi")], &mut output),
            Value::string("4869")
        );
        assert_eq!(
            call("hex2bin", vec![Value::string("4869")], &mut output),
            Value::string("Hi")
        );
        assert_eq!(
            call("hex2bin", vec![Value::string("f")], &mut output),
            Value::Bool(false)
        );
        assert_eq!(
            call("hex2bin", vec![Value::string("zz")], &mut output),
            Value::Bool(false)
        );
        assert_eq!(
            call("ord", vec![Value::string("A")], &mut output),
            Value::Int(65)
        );
        assert_eq!(
            call("chr", vec![Value::Int(321)], &mut output),
            Value::string("A")
        );
        assert_eq!(
            call("md5", vec![Value::string("abc")], &mut output),
            Value::string("900150983cd24fb0d6963f7d28e17f72")
        );
        assert_eq!(
            call("sha1", vec![Value::string("abc")], &mut output),
            Value::string("a9993e364706816aba3e25717850c26c9cd0d89d")
        );
        assert_eq!(
            call("crc32", vec![Value::string("abc")], &mut output),
            Value::Int(891_568_578)
        );
        assert_eq!(
            call("base64_encode", vec![Value::string("hi")], &mut output),
            Value::string("aGk=")
        );
        assert_eq!(
            call("base64_decode", vec![Value::string("aGk=")], &mut output),
            Value::string("hi")
        );
        assert_eq!(
            call(
                "base64_decode",
                vec![Value::string("a!Gk="), Value::Bool(false)],
                &mut output
            ),
            Value::string("hi")
        );
        assert_eq!(
            call(
                "base64_decode",
                vec![Value::string("a!Gk="), Value::Bool(true)],
                &mut output
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call(
                "htmlspecialchars",
                vec![Value::string("<a&\"'>")],
                &mut output
            ),
            Value::string("&lt;a&amp;&quot;&#039;&gt;")
        );
        assert_eq!(
            call(
                "htmlspecialchars_decode",
                vec![Value::string("&lt;a&amp;&quot;&#039;&gt;")],
                &mut output
            ),
            Value::string("<a&\"'>")
        );
        assert_eq!(
            call("htmlentities", vec![Value::string("<a&>")], &mut output),
            Value::string("&lt;a&amp;&gt;")
        );
        assert_eq!(
            call("urlencode", vec![Value::string("a b~")], &mut output),
            Value::string("a+b%7E")
        );
        assert_eq!(
            call("rawurlencode", vec![Value::string("a b~")], &mut output),
            Value::string("a%20b~")
        );
        assert_eq!(
            call("urldecode", vec![Value::string("a+b%7E")], &mut output),
            Value::string("a b~")
        );
        assert_eq!(
            call("rawurldecode", vec![Value::string("a%20b~")], &mut output),
            Value::string("a b~")
        );

        let mut query = PhpArray::new();
        query.insert(
            ArrayKey::String(PhpString::from_test_str("a")),
            Value::string("b"),
        );
        query.insert(
            ArrayKey::String(PhpString::from_test_str("c")),
            Value::Int(1),
        );
        assert_eq!(
            call("http_build_query", vec![Value::Array(query)], &mut output),
            Value::string("a=b&c=1")
        );
    }

    #[test]
    fn encoding_builtins_report_value_errors() {
        let entry = BuiltinRegistry::new().get("ord").expect("builtin exists");
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        let error = (entry.function())(
            &mut context,
            vec![Value::string("")],
            RuntimeSourceSpan::default(),
        )
        .expect_err("expected value error");
        assert_eq!(error.diagnostic_id(), "E_PHP_RUNTIME_BUILTIN_VALUE");
    }

    #[test]
    fn formatting_builtins_cover_common_printf_surface() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call(
                "sprintf",
                vec![
                    Value::string("%04d|%-5s|%.2f|%08x|%X|%o|%c|%%"),
                    Value::Int(7),
                    Value::string("x"),
                    Value::float(1.25),
                    Value::Int(255),
                    Value::Int(255),
                    Value::Int(8),
                    Value::Int(65),
                ],
                &mut output,
            ),
            Value::string("0007|x    |1.25|000000ff|FF|10|A|%")
        );
        assert_eq!(
            call(
                "sprintf",
                vec![
                    Value::string("%'_5s|%+d|% d"),
                    Value::string("x"),
                    Value::Int(7),
                    Value::Int(7)
                ],
                &mut output,
            ),
            Value::string("____x|+7|7")
        );

        assert_eq!(
            call(
                "printf",
                vec![Value::string("[%04d]"), Value::Int(7)],
                &mut output
            ),
            Value::Int(6)
        );
        assert_eq!(output.to_string_lossy(), "[0007]");

        let args = Value::packed_array(vec![Value::string("id"), Value::Int(9)]);
        assert_eq!(
            call(
                "vsprintf",
                vec![Value::string("%s:%d"), args.clone()],
                &mut output,
            ),
            Value::string("id:9")
        );
        assert_eq!(
            call("vprintf", vec![Value::string("%s:%d"), args], &mut output),
            Value::Int(4)
        );
        assert_eq!(output.to_string_lossy(), "[0007]id:9");
    }

    #[test]
    fn formatting_builtins_report_missing_args_and_stream_gap() {
        for (name, args, expected_id) in [
            (
                "sprintf",
                vec![Value::string("%s %s"), Value::string("only-one")],
                "E_PHP_RUNTIME_PRINTF_ARGUMENTS",
            ),
            (
                "fprintf",
                vec![Value::Null, Value::string("%s"), Value::string("x")],
                "E_PHP_RUNTIME_STREAM_GAP",
            ),
        ] {
            let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
            let mut output = OutputBuffer::new();
            let mut context = BuiltinContext::new(&mut output);
            let error = (entry.function())(&mut context, args, RuntimeSourceSpan::default())
                .expect_err("expected formatting error");
            assert_eq!(error.diagnostic_id(), expected_id);
        }
    }

    #[test]
    fn math_numeric_builtins_cover_common_paths() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("abs", vec![Value::Int(-7)], &mut output),
            Value::Int(7)
        );
        assert_eq!(
            call("abs", vec![Value::string("-2.5")], &mut output),
            Value::float(2.5)
        );
        assert_eq!(
            call(
                "min",
                vec![Value::packed_array(vec![
                    Value::Int(3),
                    Value::Int(1),
                    Value::Int(2)
                ])],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "max",
                vec![Value::Int(3), Value::Int(1), Value::Int(2)],
                &mut output
            ),
            Value::Int(3)
        );
        assert_eq!(
            call(
                "round",
                vec![Value::float(12.345), Value::Int(2)],
                &mut output
            ),
            Value::float(12.35)
        );
        assert_eq!(
            call("floor", vec![Value::float(3.9)], &mut output),
            Value::float(3.0)
        );
        assert_eq!(
            call("ceil", vec![Value::float(3.1)], &mut output),
            Value::float(4.0)
        );
        assert_eq!(
            call("sqrt", vec![Value::Int(9)], &mut output),
            Value::float(3.0)
        );
        assert_eq!(
            call("pow", vec![Value::Int(2), Value::Int(3)], &mut output),
            Value::Int(8)
        );
        assert_eq!(
            call("intdiv", vec![Value::Int(7), Value::Int(2)], &mut output),
            Value::Int(3)
        );
        assert_eq!(
            call("fmod", vec![Value::Int(7), Value::Int(2)], &mut output),
            Value::float(1.0)
        );
        assert_eq!(
            call("is_finite", vec![Value::float(1.5)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "is_infinite",
                vec![Value::float(f64::INFINITY)],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call("is_nan", vec![Value::float(f64::NAN)], &mut output),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "number_format",
                vec![Value::float(1234.567), Value::Int(2)],
                &mut output
            ),
            Value::string("1,234.57")
        );
        assert_eq!(
            call(
                "number_format",
                vec![
                    Value::float(1234.5),
                    Value::Int(1),
                    Value::string(","),
                    Value::string(".")
                ],
                &mut output
            ),
            Value::string("1.234,5")
        );
    }

    #[test]
    fn math_numeric_builtins_report_value_errors() {
        for (name, args) in [
            ("intdiv", vec![Value::Int(1), Value::Int(0)]),
            ("fmod", vec![Value::Int(1), Value::Int(0)]),
        ] {
            let entry = BuiltinRegistry::new().get(name).expect("builtin exists");
            let mut output = OutputBuffer::new();
            let mut context = BuiltinContext::new(&mut output);
            let error = (entry.function())(&mut context, args, RuntimeSourceSpan::default())
                .expect_err("expected value error");
            assert_eq!(error.diagnostic_id(), "E_PHP_RUNTIME_BUILTIN_VALUE");
        }
    }

    #[test]
    fn array_basic_builtins_cover_keys_values_and_list_checks() {
        let mut output = OutputBuffer::new();
        let mut mixed = PhpArray::new();
        mixed.insert(ArrayKey::Int(1), Value::string("one"));
        mixed.insert(
            ArrayKey::String(PhpString::from_test_str("01")),
            Value::string("zero-one"),
        );
        mixed.insert(
            ArrayKey::String(PhpString::from_test_str("name")),
            Value::string("n"),
        );
        let before = mixed.clone();

        assert_eq!(
            call("count", vec![Value::Array(mixed.clone())], &mut output),
            Value::Int(3)
        );
        assert_eq!(
            call("sizeof", vec![Value::packed_array(vec![])], &mut output),
            Value::Int(0)
        );
        assert_eq!(
            call(
                "array_key_exists",
                vec![Value::string("1"), Value::Array(mixed.clone())],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call("array_keys", vec![Value::Array(mixed.clone())], &mut output),
            Value::packed_array(vec![
                Value::Int(1),
                Value::string("01"),
                Value::string("name")
            ])
        );
        assert_eq!(
            call(
                "array_values",
                vec![Value::Array(mixed.clone())],
                &mut output
            ),
            Value::packed_array(vec![
                Value::string("one"),
                Value::string("zero-one"),
                Value::string("n")
            ])
        );
        assert_eq!(
            call(
                "array_is_list",
                vec![Value::packed_array(vec![Value::Int(1)])],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "array_is_list",
                vec![Value::Array(mixed.clone())],
                &mut output
            ),
            Value::Bool(false)
        );
        assert_eq!(
            call(
                "array_key_first",
                vec![Value::Array(mixed.clone())],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "array_key_last",
                vec![Value::Array(mixed.clone())],
                &mut output
            ),
            Value::string("name")
        );
        assert_eq!(mixed, before);
    }

    #[test]
    fn array_basic_builtins_cover_strict_search_and_columns() {
        let mut output = OutputBuffer::new();
        let haystack = Value::packed_array(vec![Value::Int(0), Value::string("7"), Value::Int(7)]);

        assert_eq!(
            call(
                "in_array",
                vec![Value::Int(7), haystack.clone(), Value::Bool(false)],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "in_array",
                vec![Value::string("7"), haystack.clone(), Value::Bool(true)],
                &mut output
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                "array_search",
                vec![Value::string("7"), haystack.clone(), Value::Bool(true)],
                &mut output
            ),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "array_search",
                vec![Value::string("missing"), haystack, Value::Bool(false)],
                &mut output
            ),
            Value::Bool(false)
        );

        let mut first = PhpArray::new();
        first.insert(
            ArrayKey::String(PhpString::from_test_str("id")),
            Value::Int(2),
        );
        first.insert(
            ArrayKey::String(PhpString::from_test_str("name")),
            Value::string("Ada"),
        );
        let mut second = PhpArray::new();
        second.insert(
            ArrayKey::String(PhpString::from_test_str("id")),
            Value::Int(3),
        );
        second.insert(
            ArrayKey::String(PhpString::from_test_str("name")),
            Value::string("Grace"),
        );
        let rows = Value::packed_array(vec![Value::Array(first), Value::Array(second)]);

        let mut expected = PhpArray::new();
        expected.insert(ArrayKey::Int(2), Value::string("Ada"));
        expected.insert(ArrayKey::Int(3), Value::string("Grace"));
        assert_eq!(
            call(
                "array_column",
                vec![rows, Value::string("name"), Value::string("id")],
                &mut output
            ),
            Value::Array(expected)
        );
    }

    #[test]
    fn array_stack_builtins_mutate_only_references() {
        let mut output = OutputBuffer::new();
        let cell = ReferenceCell::new(Value::packed_array(vec![Value::Int(1), Value::Int(2)]));

        assert_eq!(
            call(
                "array_push",
                vec![Value::Reference(cell.clone()), Value::Int(3)],
                &mut output
            ),
            Value::Int(3)
        );
        assert_eq!(
            cell.get(),
            Value::packed_array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(
            call(
                "array_pop",
                vec![Value::Reference(cell.clone())],
                &mut output
            ),
            Value::Int(3)
        );
        assert_eq!(
            call(
                "array_unshift",
                vec![Value::Reference(cell.clone()), Value::Int(0)],
                &mut output
            ),
            Value::Int(3)
        );
        assert_eq!(
            call(
                "array_shift",
                vec![Value::Reference(cell.clone())],
                &mut output
            ),
            Value::Int(0)
        );
        assert_eq!(
            cell.get(),
            Value::packed_array(vec![Value::Int(1), Value::Int(2)])
        );
    }

    #[test]
    fn array_slice_merge_and_transform_builtins_work() {
        let mut output = OutputBuffer::new();
        let mut keyed = PhpArray::new();
        keyed.insert(ArrayKey::Int(2), Value::string("two"));
        keyed.insert(
            ArrayKey::String(PhpString::from_test_str("a")),
            Value::Int(1),
        );
        keyed.insert(ArrayKey::Int(4), Value::string("four"));

        let mut expected_slice = PhpArray::new();
        expected_slice.insert(
            ArrayKey::String(PhpString::from_test_str("a")),
            Value::Int(1),
        );
        expected_slice.append(Value::string("four"));
        assert_eq!(
            call(
                "array_slice",
                vec![Value::Array(keyed.clone()), Value::Int(1), Value::Int(2)],
                &mut output
            ),
            Value::Array(expected_slice)
        );
        let mut expected_reverse = PhpArray::new();
        expected_reverse.append(Value::string("four"));
        expected_reverse.insert(
            ArrayKey::String(PhpString::from_test_str("a")),
            Value::Int(1),
        );
        expected_reverse.append(Value::string("two"));
        assert_eq!(
            call(
                "array_reverse",
                vec![Value::Array(keyed.clone()), Value::Bool(false)],
                &mut output
            ),
            Value::Array(expected_reverse)
        );
        assert_eq!(
            call(
                "array_pad",
                vec![
                    Value::packed_array(vec![Value::Int(1)]),
                    Value::Int(3),
                    Value::Int(0)
                ],
                &mut output
            ),
            Value::packed_array(vec![Value::Int(1), Value::Int(0), Value::Int(0)])
        );

        let mut left = PhpArray::new();
        left.insert(ArrayKey::Int(0), Value::string("x"));
        left.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(1),
        );
        let mut right = PhpArray::new();
        right.insert(ArrayKey::Int(7), Value::string("y"));
        right.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(2),
        );
        let mut expected_merge = PhpArray::new();
        expected_merge.append(Value::string("x"));
        expected_merge.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(2),
        );
        expected_merge.append(Value::string("y"));
        assert_eq!(
            call(
                "array_merge",
                vec![Value::Array(left.clone()), Value::Array(right.clone())],
                &mut output
            ),
            Value::Array(expected_merge)
        );

        let mut expected_replace = keyed.clone();
        expected_replace.insert(ArrayKey::Int(7), Value::string("y"));
        expected_replace.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(2),
        );
        assert_eq!(
            call(
                "array_replace",
                vec![Value::Array(keyed), Value::Array(right)],
                &mut output
            ),
            Value::Array(expected_replace)
        );
    }

    #[test]
    fn array_splice_chunk_flip_and_recursive_merge_work() {
        let mut output = OutputBuffer::new();
        let cell = ReferenceCell::new(Value::packed_array(vec![
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
        ]));
        assert_eq!(
            call(
                "array_splice",
                vec![
                    Value::Reference(cell.clone()),
                    Value::Int(1),
                    Value::Int(1),
                    Value::packed_array(vec![Value::string("x")])
                ],
                &mut output
            ),
            Value::packed_array(vec![Value::string("b")])
        );
        assert_eq!(
            cell.get(),
            Value::packed_array(vec![
                Value::string("a"),
                Value::string("x"),
                Value::string("c")
            ])
        );

        assert_eq!(
            call(
                "array_chunk",
                vec![
                    Value::packed_array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
                    Value::Int(2)
                ],
                &mut output
            ),
            Value::packed_array(vec![
                Value::packed_array(vec![Value::Int(1), Value::Int(2)]),
                Value::packed_array(vec![Value::Int(3)])
            ])
        );

        let mut flip_input = PhpArray::new();
        flip_input.insert(
            ArrayKey::String(PhpString::from_test_str("a")),
            Value::Int(1),
        );
        flip_input.insert(
            ArrayKey::String(PhpString::from_test_str("b")),
            Value::string("x"),
        );
        let mut expected_flip = PhpArray::new();
        expected_flip.insert(ArrayKey::Int(1), Value::string("a"));
        expected_flip.insert(
            ArrayKey::String(PhpString::from_test_str("x")),
            Value::string("b"),
        );
        assert_eq!(
            call("array_flip", vec![Value::Array(flip_input)], &mut output),
            Value::Array(expected_flip)
        );

        let mut first = PhpArray::new();
        first.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(1),
        );
        let mut second = PhpArray::new();
        second.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::Int(2),
        );
        let mut expected = PhpArray::new();
        expected.insert(
            ArrayKey::String(PhpString::from_test_str("k")),
            Value::packed_array(vec![Value::Int(1), Value::Int(2)]),
        );
        assert_eq!(
            call(
                "array_merge_recursive",
                vec![Value::Array(first), Value::Array(second)],
                &mut output
            ),
            Value::Array(expected)
        );
    }

    #[test]
    fn serialization_builtins_roundtrip_and_fail_closed() {
        let mut output = OutputBuffer::new();

        assert_eq!(
            call("serialize", vec![Value::Int(1)], &mut output),
            Value::string("i:1;")
        );
        assert_eq!(
            call("unserialize", vec![Value::string("i:1;")], &mut output),
            Value::Int(1)
        );
        assert_eq!(
            call(
                "unserialize",
                vec![Value::string("bad payload")],
                &mut output
            ),
            Value::Bool(false)
        );
    }
}
