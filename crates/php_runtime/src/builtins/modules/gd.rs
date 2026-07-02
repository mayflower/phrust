//! Bounded GD-compatible image helpers for WordPress media flows.

use super::core::{
    argument_type_error, arity_error, int_arg, read_file_value, resolve_runtime_path, string_arg,
    string_array_key,
};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use crate::{ClassEntry, ClassFlags, ObjectRef, PhpArray, Value, normalize_class_name};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use std::fs;
use std::io::Cursor;

const IMG_JPG: i64 = 2;
const IMG_PNG: i64 = 4;
const SUPPORTED_IMAGE_TYPES: i64 = IMG_JPG | IMG_PNG;

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("gd_info", builtin_gd_info, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "imagecopyresampled",
        builtin_imagecopyresampled,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "imagecreatefromjpeg",
        builtin_imagecreatefromjpeg,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "imagecreatefrompng",
        builtin_imagecreatefrompng,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "imagecreatefromstring",
        builtin_imagecreatefromstring,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "imagecreatetruecolor",
        builtin_imagecreatetruecolor,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("imagetypes", builtin_imagetypes, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "imagedestroy",
        builtin_imagedestroy,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("imagejpeg", builtin_imagejpeg, BuiltinCompatibility::Php),
    BuiltinEntry::new("imagepng", builtin_imagepng, BuiltinCompatibility::Php),
    BuiltinEntry::new("imagesx", builtin_imagesx, BuiltinCompatibility::Php),
    BuiltinEntry::new("imagesy", builtin_imagesy, BuiltinCompatibility::Php),
];

fn builtin_gd_info(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !args.is_empty() {
        return Err(arity_error("gd_info", "no arguments"));
    }
    let mut array = PhpArray::new();
    insert(&mut array, "GD Version", Value::string("phrust bounded-gd"));
    insert(&mut array, "FreeType Support", Value::Bool(false));
    insert(&mut array, "GIF Read Support", Value::Bool(false));
    insert(&mut array, "GIF Create Support", Value::Bool(false));
    insert(&mut array, "JPEG Support", Value::Bool(true));
    insert(&mut array, "PNG Support", Value::Bool(true));
    insert(&mut array, "WebP Support", Value::Bool(false));
    insert(&mut array, "AVIF Support", Value::Bool(false));
    Ok(Value::Array(array))
}

fn builtin_imagetypes(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !args.is_empty() {
        return Err(arity_error("imagetypes", "no arguments"));
    }
    Ok(Value::Int(SUPPORTED_IMAGE_TYPES))
}

fn builtin_imagecreatefromstring(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("imagecreatefromstring", "one argument"));
    }
    let bytes = string_arg("imagecreatefromstring", &args[0])?;
    Ok(decode_image(bytes.as_bytes()).map_or(Value::Bool(false), gd_object_value))
}

fn builtin_imagecreatefromjpeg(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    imagecreatefrom_file(
        context,
        args,
        span,
        "imagecreatefromjpeg",
        ImageFormat::Jpeg,
    )
}

fn builtin_imagecreatefrompng(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
) -> BuiltinResult {
    imagecreatefrom_file(context, args, span, "imagecreatefrompng", ImageFormat::Png)
}

fn builtin_imagecreatetruecolor(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 2 {
        return Err(arity_error("imagecreatetruecolor", "two arguments"));
    }
    let width = int_arg("imagecreatetruecolor", &args[0])?;
    let height = int_arg("imagecreatetruecolor", &args[1])?;
    if width <= 0 || height <= 0 {
        return Ok(Value::Bool(false));
    }
    let image = DynamicImage::ImageRgba8(RgbaImage::new(width as u32, height as u32));
    Ok(gd_object_value(image))
}

fn builtin_imagesx(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("imagesx", "one argument"));
    }
    Ok(Value::Int(gd_object_arg("imagesx", &args[0])?.0 as i64))
}

fn builtin_imagesy(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("imagesy", "one argument"));
    }
    Ok(Value::Int(gd_object_arg("imagesy", &args[0])?.1 as i64))
}

fn builtin_imagecopyresampled(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 10 {
        return Err(arity_error("imagecopyresampled", "ten arguments"));
    }
    let (_, _, dst) = gd_object_arg("imagecopyresampled", &args[0])?;
    let (_, _, src) = gd_object_arg("imagecopyresampled", &args[1])?;
    let dst_x = int_arg("imagecopyresampled", &args[2])?.max(0) as u32;
    let dst_y = int_arg("imagecopyresampled", &args[3])?.max(0) as u32;
    let src_x = int_arg("imagecopyresampled", &args[4])?.max(0) as u32;
    let src_y = int_arg("imagecopyresampled", &args[5])?.max(0) as u32;
    let dst_w = int_arg("imagecopyresampled", &args[6])?;
    let dst_h = int_arg("imagecopyresampled", &args[7])?;
    let src_w = int_arg("imagecopyresampled", &args[8])?;
    let src_h = int_arg("imagecopyresampled", &args[9])?;
    if dst_w <= 0 || dst_h <= 0 || src_w <= 0 || src_h <= 0 {
        return Ok(Value::Bool(false));
    }
    let mut dst_image = decode_gd_image(&dst)?.to_rgba8();
    let src_image = decode_gd_image(&src)?.to_rgba8();
    if src_x >= src_image.width() || src_y >= src_image.height() {
        return Ok(Value::Bool(false));
    }
    let crop_w = (src_w as u32).min(src_image.width() - src_x);
    let crop_h = (src_h as u32).min(src_image.height() - src_y);
    let cropped = imageops::crop_imm(&src_image, src_x, src_y, crop_w, crop_h).to_image();
    let resized = imageops::resize(&cropped, dst_w as u32, dst_h as u32, FilterType::Triangle);
    imageops::overlay(&mut dst_image, &resized, i64::from(dst_x), i64::from(dst_y));
    update_gd_object(&dst, DynamicImage::ImageRgba8(dst_image))?;
    Ok(Value::Bool(true))
}

fn builtin_imagejpeg(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 3 {
        return Err(arity_error("imagejpeg", "one to three argument(s)"));
    }
    let (_, _, object) = gd_object_arg("imagejpeg", &args[0])?;
    let quality = args
        .get(2)
        .map(|value| int_arg("imagejpeg", value))
        .transpose()?
        .unwrap_or(75)
        .clamp(0, 100) as u8;
    let mut bytes = Vec::new();
    let image = decode_gd_image(&object)?;
    JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode_image(&image)
        .map_err(|error| BuiltinError::new("E_PHP_RUNTIME_GD_ENCODE", error.to_string()))?;
    write_or_output_image(context, args.get(1), bytes)
}

fn builtin_imagepng(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.is_empty() || args.len() > 4 {
        return Err(arity_error("imagepng", "one to four argument(s)"));
    }
    let (_, _, object) = gd_object_arg("imagepng", &args[0])?;
    let bytes = gd_bytes(&object)?;
    write_or_output_image(context, args.get(1), bytes)
}

fn builtin_imagedestroy(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("imagedestroy", "one argument"));
    }
    let (_, _, object) = gd_object_arg("imagedestroy", &args[0])?;
    object.set_property("__gd_destroyed", Value::Bool(true));
    Ok(Value::Bool(true))
}

fn imagecreatefrom_file(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    span: RuntimeSourceSpan,
    name: &str,
    format: ImageFormat,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error(name, "one argument"));
    }
    let path = string_arg(name, &args[0])?.to_string_lossy();
    let Value::String(bytes) = read_file_value(context, name, &path, span)? else {
        return Ok(Value::Bool(false));
    };
    Ok(
        image::load_from_memory_with_format(bytes.as_bytes(), format)
            .ok()
            .map_or(Value::Bool(false), gd_object_value),
    )
}

fn decode_image(bytes: &[u8]) -> Option<DynamicImage> {
    image::load_from_memory(bytes).ok()
}

fn gd_object_value(image: DynamicImage) -> Value {
    Value::Object(gd_object(image))
}

fn gd_object(image: DynamicImage) -> ObjectRef {
    let object = ObjectRef::new_with_display_name(&gd_runtime_class(), "GdImage");
    let _ = update_gd_object(&object, image);
    object
}

fn update_gd_object(object: &ObjectRef, image: DynamicImage) -> Result<(), BuiltinError> {
    let (width, height) = image.dimensions();
    let bytes = encode_png(&image)?;
    object.set_property("__gd_width", Value::Int(i64::from(width)));
    object.set_property("__gd_height", Value::Int(i64::from(height)));
    object.set_property("__gd_format", Value::string("png"));
    object.set_property("__gd_bytes", Value::string(bytes));
    object.set_property("__gd_destroyed", Value::Bool(false));
    Ok(())
}

fn gd_object_arg(name: &str, value: &Value) -> Result<(u32, u32, ObjectRef), BuiltinError> {
    let Value::Object(object) = value else {
        return Err(argument_type_error(name, "1", "GdImage", value));
    };
    if normalize_class_name(&object.class_name()) != "gdimage" {
        return Err(argument_type_error(name, "1", "GdImage", value));
    }
    if matches!(
        object.get_property("__gd_destroyed"),
        Some(Value::Bool(true))
    ) {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_GD_DESTROYED",
            format!("{name}(): GdImage object has been destroyed"),
        ));
    }
    let width = match object.get_property("__gd_width") {
        Some(Value::Int(value)) if value > 0 => value as u32,
        _ => 0,
    };
    let height = match object.get_property("__gd_height") {
        Some(Value::Int(value)) if value > 0 => value as u32,
        _ => 0,
    };
    Ok((width, height, object.clone()))
}

fn decode_gd_image(object: &ObjectRef) -> Result<DynamicImage, BuiltinError> {
    image::load_from_memory(&gd_bytes(object)?)
        .map_err(|error| BuiltinError::new("E_PHP_RUNTIME_GD_DECODE", error.to_string()))
}

fn gd_bytes(object: &ObjectRef) -> Result<Vec<u8>, BuiltinError> {
    match object.get_property("__gd_bytes") {
        Some(Value::String(bytes)) => Ok(bytes.as_bytes().to_vec()),
        _ => Err(BuiltinError::new(
            "E_PHP_RUNTIME_GD_STATE",
            "GdImage object is missing image data",
        )),
    }
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, BuiltinError> {
    let mut cursor = Cursor::new(Vec::new());
    image
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| BuiltinError::new("E_PHP_RUNTIME_GD_ENCODE", error.to_string()))?;
    Ok(cursor.into_inner())
}

fn write_or_output_image(
    context: &mut BuiltinContext<'_>,
    path_arg: Option<&Value>,
    bytes: Vec<u8>,
) -> BuiltinResult {
    match path_arg {
        None | Some(Value::Null) => {
            context.output().write_bytes(&bytes);
            Ok(Value::Bool(true))
        }
        Some(value) => {
            let path = string_arg("image output", value)?.to_string_lossy();
            let resolved = resolve_runtime_path(context, &path);
            if !context.filesystem_capabilities().allows_path(&resolved) {
                return Ok(Value::Bool(false));
            }
            Ok(Value::Bool(fs::write(resolved, bytes).is_ok()))
        }
    }
}

fn gd_runtime_class() -> ClassEntry {
    ClassEntry {
        name: "gdimage".to_owned(),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: ClassFlags::default(),
    }
}

fn insert(array: &mut PhpArray, key: &str, value: Value) {
    array.insert(string_array_key(key), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputBuffer;

    #[test]
    fn imagetypes_reports_bounded_jpeg_and_png_support() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);

        assert_eq!(
            builtin_imagetypes(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("imagetypes succeeds"),
            Value::Int(IMG_JPG | IMG_PNG)
        );
    }

    #[test]
    fn gd_info_matches_bounded_image_type_support() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        let Value::Array(info) =
            builtin_gd_info(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("gd_info succeeds")
        else {
            panic!("expected GD info array");
        };

        assert_eq!(
            info.get(&string_array_key("JPEG Support")),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            info.get(&string_array_key("PNG Support")),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            info.get(&string_array_key("GIF Read Support")),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            info.get(&string_array_key("WebP Support")),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            info.get(&string_array_key("AVIF Support")),
            Some(&Value::Bool(false))
        );
    }
}
