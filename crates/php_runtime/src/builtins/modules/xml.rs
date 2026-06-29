//! XML extension builtins for the bounded runtime slice.

use super::core::{arity_error, int_arg, php_argument_type_name, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use crate::{Value, normalize_class_name, xml};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "xml_parser_create",
        builtin_xml_parser_create,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("xml_parse", builtin_xml_parse, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "xml_get_error_code",
        builtin_xml_get_error_code,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "xml_error_string",
        builtin_xml_error_string,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "xml_parser_create_ns",
        builtin_xml_parser_create_ns,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "xml_parser_get_option",
        builtin_xml_parser_get_option,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "xml_parser_set_option",
        builtin_xml_parser_set_option,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "xml_parser_free",
        builtin_xml_parser_free,
        BuiltinCompatibility::Php,
    ),
];

const XML_ERROR_NONE: i64 = 0;
const XML_ERROR_MISMATCHED_TAG: i64 = 76;
const XML_PARSER_ERROR_CODE: &str = "__phrust_xml_error_code";
const XML_OPTION_CASE_FOLDING: i64 = 1;
const XML_OPTION_TARGET_ENCODING: i64 = 2;
const XML_OPTION_SKIP_TAGSTART: i64 = 3;
const XML_OPTION_SKIP_WHITE: i64 = 4;
const XML_PARSER_CASE_FOLDING: &str = "__phrust_xml_case_folding";
const XML_PARSER_TARGET_ENCODING: &str = "__phrust_xml_target_encoding";
const XML_PARSER_SKIP_TAGSTART: &str = "__phrust_xml_skip_tagstart";
const XML_PARSER_SKIP_WHITE: &str = "__phrust_xml_skip_white";

fn builtin_xml_parser_create(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("xml_parser_create", "zero or one argument(s)"));
    }
    Ok(new_xml_parser())
}

fn builtin_xml_parser_create_ns(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error(
            "xml_parser_create_ns",
            "zero, one, or two argument(s)",
        ));
    }
    Ok(new_xml_parser())
}

fn new_xml_parser() -> Value {
    let object = xml::new_xml_parser();
    object.set_property(XML_PARSER_ERROR_CODE, Value::Int(XML_ERROR_NONE));
    object.set_property(XML_PARSER_CASE_FOLDING, Value::Bool(true));
    object.set_property(XML_PARSER_TARGET_ENCODING, Value::string("UTF-8"));
    object.set_property(XML_PARSER_SKIP_TAGSTART, Value::Int(0));
    object.set_property(XML_PARSER_SKIP_WHITE, Value::Bool(false));
    Value::Object(object)
}

fn builtin_xml_parse(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if !(2..=3).contains(&args.len()) {
        return Err(arity_error("xml_parse", "two or three argument(s)"));
    }
    let parser = match &args[0] {
        Value::Object(object) if normalize_class_name(&object.class_name()) == "xmlparser" => {
            object.clone()
        }
        value => {
            return Err(BuiltinError::new(
                "E_PHP_RUNTIME_BUILTIN_TYPE",
                format!(
                    "xml_parse(): Argument #1 ($parser) must be of type XMLParser, {} given",
                    php_argument_type_name(value)
                ),
            ));
        }
    };
    let input = string_arg("xml_parse", &args[1])?;
    let input = std::str::from_utf8(input.as_bytes()).map_err(|_| {
        BuiltinError::new(
            "E_PHP_RUNTIME_XML_UTF8",
            "xml_parse(): input must be valid UTF-8",
        )
    })?;
    let ok = xml::parse_xml(input).is_ok();
    parser.set_property(
        XML_PARSER_ERROR_CODE,
        Value::Int(if ok {
            XML_ERROR_NONE
        } else {
            XML_ERROR_MISMATCHED_TAG
        }),
    );
    Ok(Value::Int(i64::from(ok)))
}

fn builtin_xml_get_error_code(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("xml_get_error_code", "one argument"));
    }
    let parser = match &args[0] {
        Value::Object(object) if normalize_class_name(&object.class_name()) == "xmlparser" => {
            object
        }
        value => {
            return Err(BuiltinError::new(
                "E_PHP_RUNTIME_BUILTIN_TYPE",
                format!(
                    "xml_get_error_code(): Argument #1 ($parser) must be of type XMLParser, {} given",
                    php_argument_type_name(value)
                ),
            ));
        }
    };
    Ok(parser
        .get_property(XML_PARSER_ERROR_CODE)
        .unwrap_or(Value::Int(XML_ERROR_NONE)))
}

fn builtin_xml_error_string(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("xml_error_string", "one argument"));
    }
    let code = int_arg("xml_error_string", &args[0])?;
    let message = match code {
        XML_ERROR_NONE => "No error",
        XML_ERROR_MISMATCHED_TAG => "Mismatched tag",
        _ => "syntax error",
    };
    Ok(Value::string(message.as_bytes().to_vec()))
}

fn builtin_xml_parser_get_option(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 2 {
        return Err(arity_error("xml_parser_get_option", "two arguments"));
    }
    let parser = xml_parser_arg("xml_parser_get_option", &args[0])?;
    let option = int_arg("xml_parser_get_option", &args[1])?;
    Ok(match option {
        XML_OPTION_CASE_FOLDING => parser
            .get_property(XML_PARSER_CASE_FOLDING)
            .unwrap_or(Value::Bool(true)),
        XML_OPTION_TARGET_ENCODING => parser
            .get_property(XML_PARSER_TARGET_ENCODING)
            .unwrap_or_else(|| Value::string("UTF-8")),
        XML_OPTION_SKIP_TAGSTART => parser
            .get_property(XML_PARSER_SKIP_TAGSTART)
            .unwrap_or(Value::Int(0)),
        XML_OPTION_SKIP_WHITE => parser
            .get_property(XML_PARSER_SKIP_WHITE)
            .unwrap_or(Value::Bool(false)),
        _ => Value::Bool(false),
    })
}

fn builtin_xml_parser_set_option(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 3 {
        return Err(arity_error("xml_parser_set_option", "three arguments"));
    }
    let parser = xml_parser_arg("xml_parser_set_option", &args[0])?;
    let option = int_arg("xml_parser_set_option", &args[1])?;
    match option {
        XML_OPTION_CASE_FOLDING => {
            let enabled = crate::convert::to_bool(&args[2])
                .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_BUILTIN_TYPE", message))?;
            parser.set_property(XML_PARSER_CASE_FOLDING, Value::Bool(enabled));
        }
        XML_OPTION_TARGET_ENCODING => {
            let encoding = string_arg("xml_parser_set_option", &args[2])?;
            parser.set_property(XML_PARSER_TARGET_ENCODING, Value::String(encoding));
        }
        XML_OPTION_SKIP_TAGSTART => {
            let offset = int_arg("xml_parser_set_option", &args[2])?;
            if offset < 0 {
                return Ok(Value::Bool(false));
            }
            parser.set_property(XML_PARSER_SKIP_TAGSTART, Value::Int(offset));
        }
        XML_OPTION_SKIP_WHITE => {
            let enabled = crate::convert::to_bool(&args[2])
                .map_err(|message| BuiltinError::new("E_PHP_RUNTIME_BUILTIN_TYPE", message))?;
            parser.set_property(XML_PARSER_SKIP_WHITE, Value::Bool(enabled));
        }
        _ => return Ok(Value::Bool(false)),
    }
    Ok(Value::Bool(true))
}

fn builtin_xml_parser_free(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("xml_parser_free", "one argument"));
    }
    match &args[0] {
        Value::Object(object) if normalize_class_name(&object.class_name()) == "xmlparser" => {
            Ok(Value::Bool(true))
        }
        value => Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!(
                "xml_parser_free(): Argument #1 ($parser) must be of type XMLParser, {} given",
                php_argument_type_name(value)
            ),
        )),
    }
}

fn xml_parser_arg(name: &str, value: &Value) -> Result<crate::ObjectRef, BuiltinError> {
    match value {
        Value::Object(object) if normalize_class_name(&object.class_name()) == "xmlparser" => {
            Ok(object.clone())
        }
        value => Err(BuiltinError::new(
            "E_PHP_RUNTIME_BUILTIN_TYPE",
            format!(
                "{name}(): Argument #1 ($parser) must be of type XMLParser, {} given",
                php_argument_type_name(value)
            ),
        )),
    }
}
