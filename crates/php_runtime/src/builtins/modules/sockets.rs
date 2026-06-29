//! Default-disabled sockets surface.

use super::core::{arity_error, int_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "socket_create",
        builtin_socket_create,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "socket_last_error",
        builtin_socket_last_error,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "socket_strerror",
        builtin_socket_strerror,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_socket_create(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 3 {
        return Err(arity_error("socket_create", "three arguments"));
    }
    let _ = int_arg("socket_create", &args[0])?;
    let _ = int_arg("socket_create", &args[1])?;
    let _ = int_arg("socket_create", &args[2])?;
    Ok(Value::Bool(false))
}

fn builtin_socket_last_error(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("socket_last_error", "zero or one argument"));
    }
    Ok(Value::Int(0))
}

fn builtin_socket_strerror(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("socket_strerror", "one argument"));
    }
    let code = int_arg("socket_strerror", &args[0])?;
    Ok(Value::string(format!("socket error {code}")))
}
