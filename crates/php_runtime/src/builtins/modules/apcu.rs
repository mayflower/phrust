//! Request-local APCu MVP.

use super::core::{arity_error, assign_reference_arg, int_arg, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("apcu_add", builtin_apcu_add, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "apcu_clear_cache",
        builtin_apcu_clear_cache,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "apcu_delete",
        builtin_apcu_delete,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "apcu_enabled",
        builtin_apcu_enabled,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "apcu_exists",
        builtin_apcu_exists,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("apcu_fetch", builtin_apcu_fetch, BuiltinCompatibility::Php),
    BuiltinEntry::new("apcu_store", builtin_apcu_store, BuiltinCompatibility::Php),
];

fn builtin_apcu_enabled(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !args.is_empty() {
        return Err(arity_error("apcu_enabled", "zero arguments"));
    }
    Ok(Value::Bool(true))
}

fn builtin_apcu_store(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("apcu_store", "two or three arguments"));
    }
    let key = string_arg("apcu_store", &args[0])?.as_bytes().to_vec();
    let ttl = args
        .get(2)
        .map(|value| int_arg("apcu_store", value))
        .transpose()?
        .unwrap_or(0);
    context.apcu_state().store(key, args[1].clone(), ttl);
    Ok(Value::Bool(true))
}

fn builtin_apcu_add(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("apcu_add", "two or three arguments"));
    }
    let key = string_arg("apcu_add", &args[0])?.as_bytes().to_vec();
    let ttl = args
        .get(2)
        .map(|value| int_arg("apcu_add", value))
        .transpose()?
        .unwrap_or(0);
    Ok(Value::Bool(context.apcu_state().add(
        key,
        args[1].clone(),
        ttl,
    )))
}

fn builtin_apcu_fetch(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("apcu_fetch", "one or two arguments"));
    }
    let key = string_arg("apcu_fetch", &args[0])?;
    let value = context.apcu_state().fetch(key.as_bytes());
    assign_reference_arg(args.get(1), Value::Bool(value.is_some()));
    Ok(value.unwrap_or(Value::Bool(false)))
}

fn builtin_apcu_exists(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("apcu_exists", "one argument"));
    }
    let key = string_arg("apcu_exists", &args[0])?;
    Ok(Value::Bool(context.apcu_state().exists(key.as_bytes())))
}

fn builtin_apcu_delete(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("apcu_delete", "one argument"));
    }
    let key = string_arg("apcu_delete", &args[0])?;
    Ok(Value::Bool(context.apcu_state().delete(key.as_bytes())))
}

fn builtin_apcu_clear_cache(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !args.is_empty() {
        return Err(arity_error("apcu_clear_cache", "zero arguments"));
    }
    context.apcu_state().clear();
    Ok(Value::Bool(true))
}
