//! Json builtin registry slice.

use super::core;
use crate::builtins::{BuiltinCompatibility, BuiltinEntry};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "json_decode",
        core::builtin_json_decode,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "json_encode",
        core::builtin_json_encode,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "json_last_error",
        core::builtin_json_last_error,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "json_last_error_msg",
        core::builtin_json_last_error_msg,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "json_validate",
        core::builtin_json_validate,
        BuiltinCompatibility::Php,
    ),
];
