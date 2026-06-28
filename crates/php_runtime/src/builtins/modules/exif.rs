//! Bounded EXIF/media helpers for WordPress image checks.

use super::core::{arity_error, read_file_value, string_arg};
use super::fileinfo::{image_size, image_type, size_array};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "exif_imagetype",
        builtin_exif_imagetype,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "getimagesize",
        builtin_getimagesize,
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
