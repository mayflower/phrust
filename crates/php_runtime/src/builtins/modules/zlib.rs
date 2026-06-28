//! Bounded zlib compression helpers.

use super::core::{arity_error, int_arg, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use flate2::Compression;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use std::io::{Read, Write};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("gzcompress", builtin_gzcompress, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzdecode", builtin_gzdecode, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzencode", builtin_gzencode, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "gzuncompress",
        builtin_gzuncompress,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "zlib_decode",
        builtin_zlib_decode,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_gzencode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("gzencode", "one to three argument(s)"));
    }
    let input = string_arg("gzencode", &args[0])?;
    let level = compression_level("gzencode", args.get(1))?;
    let mut encoder = GzEncoder::new(Vec::new(), level);
    if encoder.write_all(input.as_bytes()).is_err() {
        return Ok(Value::Bool(false));
    }
    Ok(encoder.finish().map_or(Value::Bool(false), Value::string))
}

fn builtin_gzcompress(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("gzcompress", "one to three argument(s)"));
    }
    let input = string_arg("gzcompress", &args[0])?;
    let level = compression_level("gzcompress", args.get(1))?;
    let mut encoder = ZlibEncoder::new(Vec::new(), level);
    if encoder.write_all(input.as_bytes()).is_err() {
        return Ok(Value::Bool(false));
    }
    Ok(encoder.finish().map_or(Value::Bool(false), Value::string))
}

fn builtin_gzdecode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gzdecode", "one or two argument(s)"));
    }
    let input = string_arg("gzdecode", &args[0])?;
    decode_with(GzDecoder::new(input.as_bytes()))
}

fn builtin_gzuncompress(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gzuncompress", "one or two argument(s)"));
    }
    let input = string_arg("gzuncompress", &args[0])?;
    decode_with(ZlibDecoder::new(input.as_bytes()))
}

fn builtin_zlib_decode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("zlib_decode", "one or two argument(s)"));
    }
    let input = string_arg("zlib_decode", &args[0])?;
    let bytes = input.as_bytes();
    let gzip = decode_with(GzDecoder::new(bytes));
    if !matches!(gzip, Ok(Value::Bool(false))) {
        return gzip;
    }
    let zlib = decode_with(ZlibDecoder::new(bytes));
    if !matches!(zlib, Ok(Value::Bool(false))) {
        return zlib;
    }
    decode_with(DeflateDecoder::new(bytes))
}

fn compression_level(
    name: &str,
    value: Option<&Value>,
) -> Result<Compression, crate::builtins::BuiltinError> {
    let level = value
        .map(|value| int_arg(name, value))
        .transpose()?
        .unwrap_or(-1);
    Ok(if level < 0 {
        Compression::default()
    } else {
        Compression::new(level.clamp(0, 9) as u32)
    })
}

fn decode_with(mut decoder: impl Read) -> BuiltinResult {
    let mut output = Vec::new();
    Ok(if decoder.read_to_end(&mut output).is_ok() {
        Value::string(output)
    } else {
        Value::Bool(false)
    })
}
