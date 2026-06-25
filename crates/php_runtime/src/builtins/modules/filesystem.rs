//! Filesystem builtin registry slice.

use super::core;
use crate::builtins::{BuiltinCompatibility, BuiltinEntry};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "basename",
        core::builtin_basename,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("chdir", core::builtin_chdir, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "clearstatcache",
        core::builtin_clearstatcache,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("copy", core::builtin_copy, BuiltinCompatibility::Php),
    BuiltinEntry::new("dirname", core::builtin_dirname, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "file_exists",
        core::builtin_file_exists,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "file_get_contents",
        core::builtin_file_get_contents,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "file_put_contents",
        core::builtin_file_put_contents,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "filemtime",
        core::builtin_filemtime,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "filesize",
        core::builtin_filesize,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "filetype",
        core::builtin_filetype,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("getcwd", core::builtin_getcwd, BuiltinCompatibility::Php),
    BuiltinEntry::new("glob", core::builtin_glob, BuiltinCompatibility::Php),
    BuiltinEntry::new("is_dir", core::builtin_is_dir, BuiltinCompatibility::Php),
    BuiltinEntry::new("is_file", core::builtin_is_file, BuiltinCompatibility::Php),
    BuiltinEntry::new("is_link", core::builtin_is_link, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "is_readable",
        core::builtin_is_readable,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "is_writable",
        core::builtin_is_writable,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("lstat", core::builtin_lstat, BuiltinCompatibility::Php),
    BuiltinEntry::new("mkdir", core::builtin_mkdir, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "pathinfo",
        core::builtin_pathinfo,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readfile",
        core::builtin_readfile,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "realpath",
        core::builtin_realpath,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("rename", core::builtin_rename, BuiltinCompatibility::Php),
    BuiltinEntry::new("rmdir", core::builtin_rmdir, BuiltinCompatibility::Php),
    BuiltinEntry::new("stat", core::builtin_stat, BuiltinCompatibility::Php),
    BuiltinEntry::new("tempnam", core::builtin_tempnam, BuiltinCompatibility::Php),
    BuiltinEntry::new("tmpfile", core::builtin_tmpfile, BuiltinCompatibility::Php),
    BuiltinEntry::new("touch", core::builtin_touch, BuiltinCompatibility::Php),
    BuiltinEntry::new("unlink", core::builtin_unlink, BuiltinCompatibility::Php),
];
