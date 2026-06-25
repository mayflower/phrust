//! Pcre builtin registry slice.

use super::core;
use crate::builtins::{BuiltinCompatibility, BuiltinEntry};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "preg_grep",
        core::builtin_preg_grep,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_last_error",
        core::builtin_preg_last_error,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_last_error_msg",
        core::builtin_preg_last_error_msg,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_match",
        core::builtin_preg_match,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_match_all",
        core::builtin_preg_match_all,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_quote",
        core::builtin_preg_quote,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_replace",
        core::builtin_preg_replace,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_replace_callback",
        core::builtin_preg_replace_callback,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "preg_split",
        core::builtin_preg_split,
        BuiltinCompatibility::Php,
    ),
];
