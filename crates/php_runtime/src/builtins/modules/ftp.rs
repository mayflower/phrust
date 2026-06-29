//! Default-disabled FTP surface.

use super::core::{arity_error, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "ftp_connect",
        builtin_ftp_connect,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "ftp_ssl_connect",
        builtin_ftp_ssl_connect,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_ftp_connect(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    disabled_connect("ftp_connect", args)
}

fn builtin_ftp_ssl_connect(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    disabled_connect("ftp_ssl_connect", args)
}

fn disabled_connect(name: &str, args: Vec<Value>) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error(name, "one to three arguments"));
    }
    let _ = string_arg(name, &args[0])?;
    Ok(Value::Bool(false))
}
