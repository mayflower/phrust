//! Streams builtin registry slice.

use super::core;
use crate::builtins::{BuiltinCompatibility, BuiltinEntry};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "closedir",
        core::builtin_closedir,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("fclose", core::builtin_fclose, BuiltinCompatibility::Php),
    BuiltinEntry::new("feof", core::builtin_feof, BuiltinCompatibility::Php),
    BuiltinEntry::new("fflush", core::builtin_fflush, BuiltinCompatibility::Php),
    BuiltinEntry::new("fgetc", core::builtin_fgetc, BuiltinCompatibility::Php),
    BuiltinEntry::new("fgets", core::builtin_fgets, BuiltinCompatibility::Php),
    BuiltinEntry::new("fopen", core::builtin_fopen, BuiltinCompatibility::Php),
    BuiltinEntry::new("fprintf", core::builtin_fprintf, BuiltinCompatibility::Php),
    BuiltinEntry::new("fread", core::builtin_fread, BuiltinCompatibility::Php),
    BuiltinEntry::new("fseek", core::builtin_fseek, BuiltinCompatibility::Php),
    BuiltinEntry::new("ftell", core::builtin_ftell, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "ftruncate",
        core::builtin_ftruncate,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("fwrite", core::builtin_fwrite, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "vfprintf",
        core::builtin_vfprintf,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("opendir", core::builtin_opendir, BuiltinCompatibility::Php),
    BuiltinEntry::new("readdir", core::builtin_readdir, BuiltinCompatibility::Php),
    BuiltinEntry::new("rewind", core::builtin_rewind, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "rewinddir",
        core::builtin_rewinddir,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("scandir", core::builtin_scandir, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "stream_context_create",
        core::builtin_stream_context_create,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_context_get_options",
        core::builtin_stream_context_get_options,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_context_set_option",
        core::builtin_stream_context_set_option,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_copy_to_stream",
        core::builtin_stream_copy_to_stream,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_get_contents",
        core::builtin_stream_get_contents,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_get_meta_data",
        core::builtin_stream_get_meta_data,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_get_wrappers",
        core::builtin_stream_get_wrappers,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_is_local",
        core::builtin_stream_is_local,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_isatty",
        core::builtin_stream_isatty,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "stream_resolve_include_path",
        core::builtin_stream_resolve_include_path,
        BuiltinCompatibility::Php,
    ),
];
