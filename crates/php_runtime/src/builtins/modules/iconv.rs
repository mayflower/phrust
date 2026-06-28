//! Bounded iconv UTF-8/ASCII MVP.

use super::core::{argument_value_error, arity_error, int_arg, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("iconv", builtin_iconv, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "iconv_strlen",
        builtin_iconv_strlen,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "iconv_strpos",
        builtin_iconv_strpos,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "iconv_substr",
        builtin_iconv_substr,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_iconv(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 3 {
        return Err(arity_error("iconv", "three argument(s)"));
    }
    let from = encoding_arg("iconv", &args[0])?;
    let to = encoding_arg("iconv", &args[1])?;
    let input = string_arg("iconv", &args[2])?;
    convert_encoding("iconv", input.as_bytes(), from, to)
}

fn builtin_iconv_strlen(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("iconv_strlen", "one or two argument(s)"));
    }
    let input = string_arg("iconv_strlen", &args[0])?;
    let encoding = args
        .get(1)
        .map(|value| encoding_arg("iconv_strlen", value))
        .transpose()?
        .unwrap_or("UTF-8");
    let chars = chars_for_encoding("iconv_strlen", input.as_bytes(), encoding)?;
    Ok(Value::Int(chars.len() as i64))
}

fn builtin_iconv_substr(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=4).contains(&args.len()) {
        return Err(arity_error("iconv_substr", "two to four argument(s)"));
    }
    let input = string_arg("iconv_substr", &args[0])?;
    let offset = int_arg("iconv_substr", &args[1])?;
    let length = args
        .get(2)
        .map(|value| int_arg("iconv_substr", value))
        .transpose()?;
    let encoding = args
        .get(3)
        .map(|value| encoding_arg("iconv_substr", value))
        .transpose()?
        .unwrap_or("UTF-8");
    let chars = chars_for_encoding("iconv_substr", input.as_bytes(), encoding)?;
    let start = normalize_offset(chars.len(), offset);
    let end = length.map_or(chars.len(), |value| {
        if value < 0 {
            chars.len().saturating_sub(value.unsigned_abs() as usize)
        } else {
            start.saturating_add(value as usize).min(chars.len())
        }
    });
    Ok(Value::string(
        chars[start.min(chars.len())..end.min(chars.len())]
            .iter()
            .collect::<String>(),
    ))
}

fn builtin_iconv_strpos(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=4).contains(&args.len()) {
        return Err(arity_error("iconv_strpos", "two to four argument(s)"));
    }
    let haystack = string_arg("iconv_strpos", &args[0])?;
    let needle = string_arg("iconv_strpos", &args[1])?;
    let offset = args
        .get(2)
        .map(|value| int_arg("iconv_strpos", value))
        .transpose()?
        .unwrap_or(0);
    let encoding = args
        .get(3)
        .map(|value| encoding_arg("iconv_strpos", value))
        .transpose()?
        .unwrap_or("UTF-8");
    let haystack_chars = chars_for_encoding("iconv_strpos", haystack.as_bytes(), encoding)?;
    let needle_string = chars_for_encoding("iconv_strpos", needle.as_bytes(), encoding)?
        .iter()
        .collect::<String>();
    let start = normalize_offset(haystack_chars.len(), offset);
    let tail = haystack_chars[start..].iter().collect::<String>();
    Ok(tail
        .find(&needle_string)
        .map_or(Value::Bool(false), |byte_offset| {
            Value::Int((start + tail[..byte_offset].chars().count()) as i64)
        }))
}

fn encoding_arg<'a>(
    name: &str,
    value: &'a Value,
) -> Result<&'a str, crate::builtins::BuiltinError> {
    let raw = string_arg(name, value)?;
    canonical_encoding(&raw.to_string_lossy())
        .ok_or_else(|| argument_value_error(name, "encoding", "must be UTF-8, ASCII, or US-ASCII"))
}

fn canonical_encoding(encoding: &str) -> Option<&'static str> {
    let base = encoding.split("//").next().unwrap_or(encoding);
    match base.trim().to_ascii_uppercase().replace('_', "-").as_str() {
        "UTF-8" | "UTF8" => Some("UTF-8"),
        "ASCII" | "US-ASCII" => Some("ASCII"),
        _ => None,
    }
}

fn convert_encoding(name: &str, input: &[u8], from: &str, to: &str) -> BuiltinResult {
    match (from, to) {
        ("UTF-8", "UTF-8") => {
            std::str::from_utf8(input)
                .map_err(|_| argument_value_error(name, "#3 ($string)", "must be valid UTF-8"))?;
            Ok(Value::string(input.to_vec()))
        }
        ("ASCII", "ASCII") => {
            if input.is_ascii() {
                Ok(Value::string(input.to_vec()))
            } else {
                Err(argument_value_error(name, "#3 ($string)", "must be ASCII"))
            }
        }
        ("ASCII", "UTF-8") => {
            if input.is_ascii() {
                Ok(Value::string(input.to_vec()))
            } else {
                Err(argument_value_error(name, "#3 ($string)", "must be ASCII"))
            }
        }
        ("UTF-8", "ASCII") => {
            let value = std::str::from_utf8(input)
                .map_err(|_| argument_value_error(name, "#3 ($string)", "must be valid UTF-8"))?;
            if value.is_ascii() {
                Ok(Value::string(input.to_vec()))
            } else {
                Ok(Value::Bool(false))
            }
        }
        _ => Ok(Value::Bool(false)),
    }
}

fn chars_for_encoding(
    name: &str,
    input: &[u8],
    encoding: &str,
) -> Result<Vec<char>, crate::builtins::BuiltinError> {
    match encoding {
        "UTF-8" => Ok(std::str::from_utf8(input)
            .map_err(|_| argument_value_error(name, "#1 ($string)", "must be valid UTF-8"))?
            .chars()
            .collect()),
        "ASCII" => {
            if input.is_ascii() {
                Ok(input.iter().map(|byte| char::from(*byte)).collect())
            } else {
                Err(argument_value_error(name, "#1 ($string)", "must be ASCII"))
            }
        }
        _ => Err(argument_value_error(
            name,
            "encoding",
            "must be UTF-8 or ASCII",
        )),
    }
}

fn normalize_offset(len: usize, offset: i64) -> usize {
    if offset < 0 {
        len.saturating_sub(offset.unsigned_abs() as usize)
    } else {
        (offset as usize).min(len)
    }
}
