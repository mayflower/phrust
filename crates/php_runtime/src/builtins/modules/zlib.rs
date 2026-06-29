//! Bounded zlib compression helpers and gzip file resources.

use super::core::{arity_error, int_arg, resolve_runtime_path, resource_arg, string_arg};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::resource::{StreamFlags, decode_gzip_bytes};
use flate2::Compression;
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};
use std::io::{Read, Write};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("gzclose", builtin_gzclose, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzdeflate", builtin_gzdeflate, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzcompress", builtin_gzcompress, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzdecode", builtin_gzdecode, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzencode", builtin_gzencode, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzopen", builtin_gzopen, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzread", builtin_gzread, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzwrite", builtin_gzwrite, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzinflate", builtin_gzinflate, BuiltinCompatibility::Php),
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
    BuiltinEntry::new(
        "zlib_encode",
        builtin_zlib_encode,
        BuiltinCompatibility::Php,
    ),
];

pub(in crate::builtins::modules) const ZLIB_ENCODING_RAW: i64 = -15;
pub(in crate::builtins::modules) const ZLIB_ENCODING_GZIP: i64 = 31;
pub(in crate::builtins::modules) const ZLIB_ENCODING_DEFLATE: i64 = 15;

fn builtin_gzopen(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("gzopen", "two or three argument(s)"));
    }
    let path_arg = string_arg("gzopen", &args[0])?.to_string_lossy();
    let mode = string_arg("gzopen", &args[1])?.to_string_lossy();
    let path = resolve_runtime_path(context, &path_arg);
    if !context.filesystem_capabilities().allows_path(&path) {
        return Ok(Value::Bool(false));
    }
    let readable = mode.starts_with('r');
    let writable = matches!(mode.as_bytes().first().copied(), Some(b'w' | b'a'));
    if !readable && !writable {
        return Ok(Value::Bool(false));
    }
    let buffer = if readable || mode.starts_with('a') {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) if writable => Vec::new(),
            Err(_) => return Ok(Value::Bool(false)),
        };
        if bytes.is_empty() {
            Vec::new()
        } else {
            match decode_gzip_bytes(&bytes) {
                Ok(bytes) => bytes,
                Err(_) => return Ok(Value::Bool(false)),
            }
        }
    } else {
        Vec::new()
    };
    let cursor = if mode.starts_with('a') {
        buffer.len()
    } else {
        0
    };
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    let flags = StreamFlags::new(
        readable || mode.contains('+'),
        writable || mode.contains('+'),
        true,
    );
    Ok(Value::Resource(
        resources.register_gzip_file(path, mode, flags, buffer, cursor),
    ))
}

fn builtin_gzread(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzread", args.len(), 2, 2)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let length = int_arg("gzread", &args[1])?.max(0) as usize;
    Ok(resource
        .read_bytes(length)
        .map_or(Value::Bool(false), Value::string))
}

fn builtin_gzwrite(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("gzwrite", "two or three argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let mut bytes = string_arg("gzwrite", &args[1])?.as_bytes().to_vec();
    if let Some(length) = args.get(2) {
        bytes.truncate(int_arg("gzwrite", length)?.max(0) as usize);
    }
    Ok(resource
        .write_bytes(&bytes)
        .map_or(Value::Bool(false), |written| Value::Int(written as i64)))
}

fn builtin_gzclose(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzclose", args.len(), 1, 1)?;
    Ok(resource_arg(&args[0]).map_or(Value::Bool(false), |resource| Value::Bool(resource.close())))
}

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

fn builtin_gzdeflate(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("gzdeflate", "one to three argument(s)"));
    }
    let input = string_arg("gzdeflate", &args[0])?;
    let level = compression_level("gzdeflate", args.get(1))?;
    let mut encoder = DeflateEncoder::new(Vec::new(), level);
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

fn builtin_gzinflate(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gzinflate", "one or two argument(s)"));
    }
    let input = string_arg("gzinflate", &args[0])?;
    decode_with(DeflateDecoder::new(input.as_bytes()))
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

fn builtin_zlib_encode(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("zlib_encode", "two or three argument(s)"));
    }
    let input = string_arg("zlib_encode", &args[0])?;
    let encoding = int_arg("zlib_encode", &args[1])?;
    let level = compression_level("zlib_encode", args.get(2))?;
    match encoding {
        ZLIB_ENCODING_RAW => {
            let mut encoder = DeflateEncoder::new(Vec::new(), level);
            if encoder.write_all(input.as_bytes()).is_err() {
                return Ok(Value::Bool(false));
            }
            Ok(encoder.finish().map_or(Value::Bool(false), Value::string))
        }
        ZLIB_ENCODING_GZIP => {
            let mut encoder = GzEncoder::new(Vec::new(), level);
            if encoder.write_all(input.as_bytes()).is_err() {
                return Ok(Value::Bool(false));
            }
            Ok(encoder.finish().map_or(Value::Bool(false), Value::string))
        }
        ZLIB_ENCODING_DEFLATE => {
            let mut encoder = ZlibEncoder::new(Vec::new(), level);
            if encoder.write_all(input.as_bytes()).is_err() {
                return Ok(Value::Bool(false));
            }
            Ok(encoder.finish().map_or(Value::Bool(false), Value::string))
        }
        _ => Ok(Value::Bool(false)),
    }
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

fn expect_zlib_arity(
    name: &str,
    actual: usize,
    min: usize,
    max: usize,
) -> Result<(), crate::builtins::BuiltinError> {
    if actual < min || actual > max {
        return Err(arity_error(name, "the expected number of argument(s)"));
    }
    Ok(())
}
