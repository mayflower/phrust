//! Bounded EXIF/media helpers for WordPress image checks.

use super::core::{arity_error, read_file_value, string_arg, string_array_key};
use super::fileinfo::{image_size, image_type, size_array};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::{PhpArray, Value};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "exif_imagetype",
        builtin_exif_imagetype,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "exif_read_data",
        builtin_exif_read_data,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "getimagesize",
        builtin_getimagesize,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "getimagesizefromstring",
        builtin_getimagesizefromstring,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_exif_imagetype(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("exif_imagetype", "one argument"));
    }
    let path = string_arg("exif_imagetype", &args[0])?.to_string_lossy();
    match read_file_value(context, "exif_imagetype", &path, span)? {
        Value::String(bytes) => {
            Ok(image_type(bytes.as_bytes()).map_or(Value::Bool(false), Value::Int))
        }
        _ => Ok(Value::Bool(false)),
    }
}

fn builtin_getimagesize(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 2 {
        return Err(arity_error("getimagesize", "one or two argument(s)"));
    }
    let path = string_arg("getimagesize", &args[0])?.to_string_lossy();
    match read_file_value(context, "getimagesize", &path, span)? {
        Value::String(bytes) => Ok(image_size(bytes.as_bytes())
            .map(|(width, height, image_type, mime)| {
                Value::Array(size_array(width, height, image_type, mime))
            })
            .unwrap_or(Value::Bool(false))),
        _ => Ok(Value::Bool(false)),
    }
}

fn builtin_getimagesizefromstring(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("getimagesizefromstring", "one argument"));
    }
    let bytes = string_arg("getimagesizefromstring", &args[0])?;
    Ok(image_size(bytes.as_bytes())
        .map(|(width, height, image_type, mime)| {
            Value::Array(size_array(width, height, image_type, mime))
        })
        .unwrap_or(Value::Bool(false)))
}

fn builtin_exif_read_data(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 4 {
        return Err(arity_error("exif_read_data", "one to four argument(s)"));
    }
    let path = string_arg("exif_read_data", &args[0])?.to_string_lossy();
    let Value::String(bytes) = read_file_value(context, "exif_read_data", &path, span)? else {
        return Ok(Value::Bool(false));
    };
    let bytes = bytes.as_bytes();
    let Some((width, height, _, _)) = image_size(bytes) else {
        return Ok(Value::Bool(false));
    };
    let mut array = PhpArray::new();
    insert_int(&mut array, "ImageWidth", width);
    insert_int(&mut array, "ImageLength", height);
    if let Some(fields) = parse_jpeg_exif(bytes) {
        if let Some(value) = fields.orientation {
            insert_int(&mut array, "Orientation", i64::from(value));
        }
        if let Some(value) = fields.date_time {
            insert_string(&mut array, "DateTime", value);
        }
        if let Some(value) = fields.make {
            insert_string(&mut array, "Make", value);
        }
        if let Some(value) = fields.model {
            insert_string(&mut array, "Model", value);
        }
    }
    Ok(Value::Array(array))
}

#[derive(Default)]
struct ExifFields {
    orientation: Option<u16>,
    date_time: Option<String>,
    make: Option<String>,
    model: Option<String>,
}

fn parse_jpeg_exif(bytes: &[u8]) -> Option<ExifFields> {
    if !bytes.starts_with(b"\xFF\xD8") {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xFF {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xD9 || marker == 0xDA || offset + 2 > bytes.len() {
            return None;
        }
        let len = u16::from_be_bytes(bytes[offset..offset + 2].try_into().ok()?) as usize;
        if len < 2 || offset + len > bytes.len() {
            return None;
        }
        let segment = &bytes[offset + 2..offset + len];
        if marker == 0xE1 && segment.starts_with(b"Exif\0\0") {
            return parse_tiff_exif(&segment[6..]);
        }
        offset += len;
    }
    None
}

fn parse_tiff_exif(bytes: &[u8]) -> Option<ExifFields> {
    if bytes.len() < 8 {
        return None;
    }
    let endian = match &bytes[0..2] {
        b"II" => Endian::Little,
        b"MM" => Endian::Big,
        _ => return None,
    };
    if read_u16(bytes, 2, endian)? != 42 {
        return None;
    }
    let ifd_offset = read_u32(bytes, 4, endian)? as usize;
    let count = read_u16(bytes, ifd_offset, endian)? as usize;
    let mut fields = ExifFields::default();
    for index in 0..count {
        let entry = ifd_offset + 2 + index * 12;
        if entry + 12 > bytes.len() {
            break;
        }
        let tag = read_u16(bytes, entry, endian)?;
        let ty = read_u16(bytes, entry + 2, endian)?;
        let count = read_u32(bytes, entry + 4, endian)? as usize;
        let value_field = entry + 8;
        match tag {
            0x0112 => fields.orientation = read_short_value(bytes, value_field, ty, count, endian),
            0x0132 => fields.date_time = read_ascii_value(bytes, value_field, ty, count, endian),
            0x010F => fields.make = read_ascii_value(bytes, value_field, ty, count, endian),
            0x0110 => fields.model = read_ascii_value(bytes, value_field, ty, count, endian),
            _ => {}
        }
    }
    Some(fields)
}

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

fn read_u16(bytes: &[u8], offset: usize, endian: Endian) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(match endian {
        Endian::Little => u16::from_le_bytes(raw),
        Endian::Big => u16::from_be_bytes(raw),
    })
}

fn read_u32(bytes: &[u8], offset: usize, endian: Endian) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(match endian {
        Endian::Little => u32::from_le_bytes(raw),
        Endian::Big => u32::from_be_bytes(raw),
    })
}

fn read_short_value(
    bytes: &[u8],
    value_field: usize,
    ty: u16,
    count: usize,
    endian: Endian,
) -> Option<u16> {
    if ty != 3 || count == 0 {
        return None;
    }
    read_u16(bytes, value_field, endian)
}

fn read_ascii_value(
    bytes: &[u8],
    value_field: usize,
    ty: u16,
    count: usize,
    endian: Endian,
) -> Option<String> {
    if ty != 2 || count == 0 {
        return None;
    }
    let data = if count <= 4 {
        bytes.get(value_field..value_field + count)?
    } else {
        let offset = read_u32(bytes, value_field, endian)? as usize;
        bytes.get(offset..offset + count)?
    };
    let trimmed = data
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();
    String::from_utf8(trimmed)
        .ok()
        .filter(|value| !value.is_empty())
}

fn insert_int(array: &mut PhpArray, key: &str, value: i64) {
    array.insert(string_array_key(key), Value::Int(value));
}

fn insert_string(array: &mut PhpArray, key: &str, value: String) {
    array.insert(string_array_key(key), Value::string(value.into_bytes()));
}
