//! Spl builtin registry slice.

use super::core;
use crate::builtins::{BuiltinCompatibility, BuiltinEntry};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "spl_autoload_call",
        core::builtin_spl_autoload_requires_vm,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "spl_autoload_functions",
        core::builtin_spl_autoload_requires_vm,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "spl_autoload_register",
        core::builtin_spl_autoload_requires_vm,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "spl_autoload_unregister",
        core::builtin_spl_autoload_requires_vm,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "spl_object_hash",
        core::builtin_spl_object_hash,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "spl_object_id",
        core::builtin_spl_object_id,
        BuiltinCompatibility::Php,
    ),
];
