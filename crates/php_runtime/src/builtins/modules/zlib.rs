//! Bounded zlib compression helpers and gzip file resources.

use super::core::{arity_error, int_arg, resolve_runtime_path, resource_arg, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::resource::{StreamFlags, StreamSeekWhence, decode_gzip_bytes};
use crate::{PhpArray, Value};
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
    BuiltinEntry::new("gzeof", builtin_gzeof, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzfile", builtin_gzfile, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzgetc", builtin_gzgetc, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzgets", builtin_gzgets, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzopen", builtin_gzopen, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzpassthru", builtin_gzpassthru, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzputs", builtin_gzwrite, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzread", builtin_gzread, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzrewind", builtin_gzrewind, BuiltinCompatibility::Php),
    BuiltinEntry::new("gzseek", builtin_gzseek, BuiltinCompatibility::Php),
    BuiltinEntry::new("gztell", builtin_gztell, BuiltinCompatibility::Php),
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
    BuiltinEntry::new("readgzfile", builtin_readgzfile, BuiltinCompatibility::Php),
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

fn builtin_gzgetc(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzgetc", args.len(), 1, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let bytes = resource.read_bytes(1).unwrap_or_default();
    if bytes.is_empty() {
        Ok(Value::Bool(false))
    } else {
        Ok(Value::string(bytes))
    }
}

fn builtin_gzgets(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gzgets", "one or two argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let mut line = resource.read_line().unwrap_or_default();
    if let Some(length) = args.get(1) {
        line.truncate(int_arg("gzgets", length)?.max(0) as usize);
    }
    if line.is_empty() {
        Ok(Value::Bool(false))
    } else {
        Ok(Value::string(line))
    }
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

fn builtin_gzpassthru(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzpassthru", args.len(), 1, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    let bytes = resource.read_to_end().unwrap_or_default();
    context.output().write_bytes(&bytes);
    Ok(Value::Int(bytes.len() as i64))
}

fn builtin_gzrewind(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzrewind", args.len(), 1, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(resource.rewind().is_ok()))
}

fn builtin_gzseek(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 3 {
        return Err(arity_error("gzseek", "two or three argument(s)"));
    }
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Int(-1));
    };
    let offset = int_arg("gzseek", &args[1])?;
    let whence = match args
        .get(2)
        .map(|value| int_arg("gzseek", value))
        .transpose()?
    {
        Some(1) => StreamSeekWhence::Current,
        Some(2) => StreamSeekWhence::End,
        Some(0) | None => StreamSeekWhence::Set,
        Some(_) => return Ok(Value::Int(-1)),
    };
    Ok(if resource.seek_from(offset, whence).is_ok() {
        Value::Int(0)
    } else {
        Value::Int(-1)
    })
}

fn builtin_gztell(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gztell", args.len(), 1, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(false));
    };
    Ok(resource
        .tell()
        .map_or(Value::Bool(false), |offset| Value::Int(offset as i64)))
}

fn builtin_gzeof(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzeof", args.len(), 1, 1)?;
    let Some(resource) = resource_arg(&args[0]) else {
        return Ok(Value::Bool(true));
    };
    Ok(Value::Bool(resource.eof().unwrap_or(true)))
}

fn builtin_gzclose(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_zlib_arity("gzclose", args.len(), 1, 1)?;
    Ok(resource_arg(&args[0]).map_or(Value::Bool(false), |resource| Value::Bool(resource.close())))
}

fn builtin_gzfile(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("gzfile", "one or two argument(s)"));
    }
    let path = string_arg("gzfile", &args[0])?.to_string_lossy();
    let Some(bytes) = decode_gzip_path(context, &path) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Array(PhpArray::from_packed(gzip_lines(&bytes))))
}

fn builtin_readgzfile(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("readgzfile", "one or two argument(s)"));
    }
    let path = string_arg("readgzfile", &args[0])?.to_string_lossy();
    let Some(bytes) = decode_gzip_path(context, &path) else {
        return Ok(Value::Bool(false));
    };
    context.output().write_bytes(&bytes);
    Ok(Value::Int(bytes.len() as i64))
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
    decode_with(
        GzDecoder::new(input.as_bytes()),
        max_length("gzdecode", args.get(1))?,
    )
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
    decode_with(
        ZlibDecoder::new(input.as_bytes()),
        max_length("gzuncompress", args.get(1))?,
    )
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
    decode_with(
        DeflateDecoder::new(input.as_bytes()),
        max_length("gzinflate", args.get(1))?,
    )
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
    let max_length = max_length("zlib_decode", args.get(1))?;
    let bytes = input.as_bytes();
    let gzip = decode_with(GzDecoder::new(bytes), max_length);
    if !matches!(gzip, Ok(Value::Bool(false))) {
        return gzip;
    }
    let zlib = decode_with(ZlibDecoder::new(bytes), max_length);
    if !matches!(zlib, Ok(Value::Bool(false))) {
        return zlib;
    }
    decode_with(DeflateDecoder::new(bytes), max_length)
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

fn max_length(
    name: &str,
    value: Option<&Value>,
) -> Result<Option<usize>, crate::builtins::BuiltinError> {
    Ok(value
        .map(|value| int_arg(name, value))
        .transpose()?
        .filter(|length| *length > 0)
        .map(|length| length as usize))
}

fn decode_with(mut decoder: impl Read, max_length: Option<usize>) -> BuiltinResult {
    let mut output = Vec::new();
    Ok(
        if decoder.read_to_end(&mut output).is_ok()
            && max_length.is_none_or(|max_length| output.len() <= max_length)
        {
            Value::string(output)
        } else {
            Value::Bool(false)
        },
    )
}

fn decode_gzip_path(context: &mut BuiltinContext<'_>, path_arg: &str) -> Option<Vec<u8>> {
    let path = resolve_runtime_path(context, path_arg);
    if !context.filesystem_capabilities().allows_path(&path) {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    decode_gzip_bytes(&bytes).ok()
}

fn gzip_lines(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(|line| Value::string(line.to_vec()))
        .collect()
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
