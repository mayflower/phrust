//! Bounded filter extension MVP for common validation and sanitization.

use super::core::{arity_error, conversion_error, deref_value, int_arg, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::{ArrayKey, Value, to_string};
use std::net::IpAddr;

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "filter_input",
        builtin_filter_input,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("filter_var", builtin_filter_var, BuiltinCompatibility::Php),
];

const FILTER_DEFAULT: i64 = 516;
const FILTER_VALIDATE_BOOL: i64 = 258;
const FILTER_VALIDATE_INT: i64 = 257;
const FILTER_VALIDATE_FLOAT: i64 = 259;
const FILTER_VALIDATE_URL: i64 = 273;
const FILTER_VALIDATE_EMAIL: i64 = 274;
const FILTER_VALIDATE_IP: i64 = 275;
const FILTER_SANITIZE_EMAIL: i64 = 517;
const FILTER_SANITIZE_URL: i64 = 518;
const FILTER_SANITIZE_NUMBER_INT: i64 = 519;
const FILTER_NULL_ON_FAILURE: i64 = 134_217_728;
const FILTER_FLAG_IPV4: i64 = 1_048_576;
const FILTER_FLAG_IPV6: i64 = 2_097_152;
const FILTER_FLAG_PATH_REQUIRED: i64 = 262_144;
const FILTER_FLAG_QUERY_REQUIRED: i64 = 524_288;

fn builtin_filter_var(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("filter_var", "one to three argument(s)"));
    }
    let filter = args
        .get(1)
        .map(|value| int_arg("filter_var", value))
        .transpose()?
        .unwrap_or(FILTER_DEFAULT);
    let flags = args
        .get(2)
        .map(|value| filter_options_flags("filter_var", value))
        .transpose()?
        .unwrap_or(0);
    apply_filter("filter_var", &args[0], filter, flags)
}

fn apply_filter(name: &str, value: &Value, filter: i64, flags: i64) -> BuiltinResult {
    let failure = if flags & FILTER_NULL_ON_FAILURE != 0 {
        Value::Null
    } else {
        Value::Bool(false)
    };
    match filter {
        FILTER_DEFAULT => Ok(value.clone()),
        FILTER_VALIDATE_EMAIL => validate_email(name, value, failure),
        FILTER_VALIDATE_INT => validate_int(name, value, failure),
        FILTER_VALIDATE_FLOAT => validate_float(name, value, failure),
        FILTER_VALIDATE_URL => validate_url(name, value, flags, failure),
        FILTER_VALIDATE_IP => validate_ip(name, value, flags, failure),
        FILTER_VALIDATE_BOOL => validate_bool(name, value, flags, failure),
        FILTER_SANITIZE_EMAIL => sanitize(name, value, is_email_sanitize_byte),
        FILTER_SANITIZE_URL => sanitize(name, value, is_url_sanitize_byte),
        FILTER_SANITIZE_NUMBER_INT => sanitize(name, value, |byte| {
            byte.is_ascii_digit() || byte == b'+' || byte == b'-'
        }),
        _ => Ok(failure),
    }
}

fn validate_int(name: &str, value: &Value, failure: Value) -> BuiltinResult {
    let input = string_arg(name, value)?;
    let text = input.to_string_lossy();
    let trimmed = text.trim();
    if trimmed.parse::<i64>().is_ok()
        && trimmed.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_digit() || (index == 0 && matches!(byte, b'+' | b'-'))
        })
    {
        Ok(Value::Int(trimmed.parse::<i64>().unwrap_or_default()))
    } else {
        Ok(failure)
    }
}

fn validate_float(name: &str, value: &Value, failure: Value) -> BuiltinResult {
    let input = string_arg(name, value)?;
    let text = input.to_string_lossy();
    let trimmed = text.trim();
    match trimmed.parse::<f64>() {
        Ok(number) if number.is_finite() => Ok(Value::float(number)),
        _ => Ok(failure),
    }
}

fn builtin_filter_input(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=5).contains(&args.len()) {
        return Err(arity_error("filter_input", "two to five argument(s)"));
    }
    let source = int_arg("filter_input", &args[0])?;
    let name = string_arg("filter_input", &args[1])?.to_string_lossy();
    let Some(value) = context.filter_input_value(source, &name) else {
        return Ok(Value::Null);
    };
    let filter = args
        .get(2)
        .map(|value| int_arg("filter_input", value))
        .transpose()?
        .unwrap_or(FILTER_DEFAULT);
    let flags = args
        .get(3)
        .map(|value| filter_options_flags("filter_input", value))
        .transpose()?
        .unwrap_or(0);
    apply_filter("filter_input", &value, filter, flags)
}

fn filter_options_flags(name: &str, value: &Value) -> Result<i64, crate::builtins::BuiltinError> {
    match deref_value(value) {
        Value::Array(array) => {
            let key = ArrayKey::String(crate::PhpString::from_test_str("flags"));
            match array.get(&key) {
                Some(value) => int_arg(name, value),
                None => Ok(0),
            }
        }
        other => int_arg(name, &other),
    }
}

fn validate_email(name: &str, value: &Value, failure: Value) -> BuiltinResult {
    let input = string_arg(name, value)?;
    let string = input.to_string_lossy();
    let mut parts = string.split('@');
    let Some(local) = parts.next() else {
        return Ok(failure);
    };
    let Some(domain) = parts.next() else {
        return Ok(failure);
    };
    if parts.next().is_none()
        && !local.is_empty()
        && domain.contains('.')
        && !php_source::byte_kernel::contains_ascii_whitespace(input.as_bytes())
    {
        Ok(Value::String(input))
    } else {
        Ok(failure)
    }
}

fn validate_url(name: &str, value: &Value, flags: i64, failure: Value) -> BuiltinResult {
    let input = string_arg(name, value)?;
    let string = input.to_string_lossy();
    let has_scheme = string.starts_with("http://") || string.starts_with("https://");
    let after_scheme = string.split_once("://").map(|(_, tail)| tail).unwrap_or("");
    let has_host = !after_scheme.is_empty()
        && !after_scheme.starts_with('/')
        && !php_source::byte_kernel::contains_ascii_whitespace(after_scheme.as_bytes());
    let path_ok = flags & FILTER_FLAG_PATH_REQUIRED == 0 || after_scheme.contains('/');
    let query_ok = flags & FILTER_FLAG_QUERY_REQUIRED == 0 || after_scheme.contains('?');
    if has_scheme && has_host && path_ok && query_ok {
        Ok(Value::String(input))
    } else {
        Ok(failure)
    }
}

fn validate_ip(name: &str, value: &Value, flags: i64, failure: Value) -> BuiltinResult {
    let input = string_arg(name, value)?;
    let string = input.to_string_lossy();
    match string.parse::<IpAddr>() {
        Ok(IpAddr::V4(_)) if flags & FILTER_FLAG_IPV6 == 0 => Ok(Value::String(input)),
        Ok(IpAddr::V6(_)) if flags & FILTER_FLAG_IPV4 == 0 => Ok(Value::String(input)),
        Ok(_) => Ok(failure),
        Err(_) => Ok(failure),
    }
}

fn validate_bool(name: &str, value: &Value, flags: i64, failure: Value) -> BuiltinResult {
    let string = to_string(value)
        .map_err(|message| conversion_error(name, message))?
        .to_string_lossy()
        .to_ascii_lowercase();
    match string.as_str() {
        "1" | "true" | "on" | "yes" => Ok(Value::Bool(true)),
        "0" | "false" | "off" | "no" | "" => Ok(Value::Bool(false)),
        _ if flags & FILTER_NULL_ON_FAILURE != 0 => Ok(Value::Null),
        _ => Ok(failure),
    }
}

fn sanitize(name: &str, value: &Value, keep: impl Fn(u8) -> bool) -> BuiltinResult {
    let input = string_arg(name, value)?;
    Ok(Value::string(
        input
            .as_bytes()
            .iter()
            .copied()
            .filter(|byte| keep(*byte))
            .collect::<Vec<_>>(),
    ))
}

fn is_email_sanitize_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-=?^_`{|}~@.[]".contains(&byte)
}

fn is_url_sanitize_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"$-_.+!*'(),{}|\\^~[]`<>#%\";/?:@&=".contains(&byte)
}
