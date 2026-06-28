//! Fileinfo MVP for common WordPress media MIME checks.

use super::core::{arity_error, int_arg, read_file_value, resource_arg, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::{PhpArray, ResourceKind, Value};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "finfo_buffer",
        builtin_finfo_buffer,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("finfo_file", builtin_finfo_file, BuiltinCompatibility::Php),
    BuiltinEntry::new("finfo_open", builtin_finfo_open, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "mime_content_type",
        builtin_mime_content_type,
        BuiltinCompatibility::Php,
    ),
];

pub(in crate::builtins::modules) const FILEINFO_NONE: i64 = 0;
pub(in crate::builtins::modules) const FILEINFO_MIME_TYPE: i64 = 16;
pub(in crate::builtins::modules) const FILEINFO_MIME_ENCODING: i64 = 1024;
pub(in crate::builtins::modules) const FILEINFO_MIME: i64 = 1040;

pub(in crate::builtins::modules) const IMAGETYPE_GIF: i64 = 1;
pub(in crate::builtins::modules) const IMAGETYPE_JPEG: i64 = 2;
pub(in crate::builtins::modules) const IMAGETYPE_PNG: i64 = 3;
pub(in crate::builtins::modules) const IMAGETYPE_WEBP: i64 = 18;
pub(in crate::builtins::modules) const IMAGETYPE_AVIF: i64 = 19;

fn builtin_finfo_open(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error("finfo_open", "zero to two argument(s)"));
    }
    let Some(resources) = context.resources() else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Resource(resources.register_fileinfo()))
}

fn builtin_finfo_buffer(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 4 {
        return Err(arity_error("finfo_buffer", "two to four argument(s)"));
    }
    if !is_fileinfo_resource(&args[0]) {
        return Ok(Value::Bool(false));
    }
    let data = string_arg("finfo_buffer", &args[1])?;
    let flags = args
        .get(2)
        .map(|value| int_arg("finfo_buffer", value))
        .transpose()?
        .unwrap_or(FILEINFO_NONE);
    Ok(Value::string(format_mime(
        mime_for_bytes(data.as_bytes(), None),
        flags,
    )))
}

fn builtin_finfo_file(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() < 2 || args.len() > 4 {
        return Err(arity_error("finfo_file", "two to four argument(s)"));
    }
    if !is_fileinfo_resource(&args[0]) {
        return Ok(Value::Bool(false));
    }
    let path = string_arg("finfo_file", &args[1])?.to_string_lossy();
    let flags = args
        .get(2)
        .map(|value| int_arg("finfo_file", value))
        .transpose()?
        .unwrap_or(FILEINFO_NONE);
    mime_for_file(context, "finfo_file", &path, flags, span)
}

fn builtin_mime_content_type(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("mime_content_type", "one argument"));
    }
    let path = string_arg("mime_content_type", &args[0])?.to_string_lossy();
    mime_for_file(
        context,
        "mime_content_type",
        &path,
        FILEINFO_MIME_TYPE,
        span,
    )
}

fn is_fileinfo_resource(value: &Value) -> bool {
    resource_arg(value).is_some_and(|resource| resource.kind() == ResourceKind::FileInfo)
}

fn mime_for_file(
    context: &mut BuiltinContext<'_>,
    name: &str,
    path: &str,
    flags: i64,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    match read_file_value(context, name, path, span)? {
        Value::String(bytes) => Ok(Value::string(format_mime(
            mime_for_bytes(bytes.as_bytes(), Some(path)),
            flags,
        ))),
        _ => Ok(Value::Bool(false)),
    }
}

pub(in crate::builtins::modules) fn mime_for_bytes(
    bytes: &[u8],
    path: Option<&str>,
) -> &'static str {
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        "image/png"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.len() >= 12 && &bytes[4..12] == b"ftypavif" {
        "image/avif"
    } else if bytes.starts_with(b"%PDF-") {
        "application/pdf"
    } else {
        path.and_then(mime_from_extension)
            .unwrap_or("application/octet-stream")
    }
}

fn mime_from_extension(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "pdf" => Some("application/pdf"),
        "txt" => Some("text/plain"),
        "json" => Some("application/json"),
        "zip" => Some("application/zip"),
        _ => None,
    }
}

fn format_mime(mime: &str, flags: i64) -> String {
    match flags {
        FILEINFO_MIME => format!("{mime}; charset=binary"),
        FILEINFO_MIME_ENCODING => "binary".to_owned(),
        _ => mime.to_owned(),
    }
}

pub(in crate::builtins::modules) fn image_type(bytes: &[u8]) -> Option<i64> {
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some(IMAGETYPE_JPEG)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        Some(IMAGETYPE_PNG)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(IMAGETYPE_GIF)
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(IMAGETYPE_WEBP)
    } else if bytes.len() >= 12 && &bytes[4..12] == b"ftypavif" {
        Some(IMAGETYPE_AVIF)
    } else {
        None
    }
}

pub(in crate::builtins::modules) fn image_size(
    bytes: &[u8],
) -> Option<(i64, i64, i64, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") && bytes.len() >= 24 {
        let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?) as i64;
        let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?) as i64;
        return Some((width, height, IMAGETYPE_PNG, "image/png"));
    }
    if (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) && bytes.len() >= 10 {
        let width = u16::from_le_bytes(bytes[6..8].try_into().ok()?) as i64;
        let height = u16::from_le_bytes(bytes[8..10].try_into().ok()?) as i64;
        return Some((width, height, IMAGETYPE_GIF, "image/gif"));
    }
    if bytes.len() >= 30
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
        && &bytes[12..16] == b"VP8 "
    {
        let width = (u16::from_le_bytes(bytes[26..28].try_into().ok()?) & 0x3fff) as i64;
        let height = (u16::from_le_bytes(bytes[28..30].try_into().ok()?) & 0x3fff) as i64;
        return Some((width, height, IMAGETYPE_WEBP, "image/webp"));
    }
    jpeg_size(bytes).map(|(width, height)| (width, height, IMAGETYPE_JPEG, "image/jpeg"))
}

fn jpeg_size(bytes: &[u8]) -> Option<(i64, i64)> {
    if !bytes.starts_with(b"\xFF\xD8") {
        return None;
    }
    let mut offset = 2;
    while offset + 9 < bytes.len() {
        if bytes[offset] != 0xFF {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if offset + 2 > bytes.len() {
            break;
        }
        let len = u16::from_be_bytes(bytes[offset..offset + 2].try_into().ok()?) as usize;
        if (0xC0..=0xC3).contains(&marker) && offset + 7 < bytes.len() {
            let height = u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().ok()?) as i64;
            let width = u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().ok()?) as i64;
            return Some((width, height));
        }
        offset = offset.saturating_add(len);
    }
    None
}

pub(in crate::builtins::modules) fn size_array(
    width: i64,
    height: i64,
    image_type: i64,
    mime: &str,
) -> PhpArray {
    let mut array = PhpArray::new();
    array.insert(crate::ArrayKey::Int(0), Value::Int(width));
    array.insert(crate::ArrayKey::Int(1), Value::Int(height));
    array.insert(crate::ArrayKey::Int(2), Value::Int(image_type));
    array.insert(
        crate::ArrayKey::Int(3),
        Value::string(format!("width=\"{width}\" height=\"{height}\"")),
    );
    array.insert(
        crate::builtins::modules::core::string_array_key("mime"),
        Value::string(mime),
    );
    array
}
