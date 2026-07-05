//! Bounded SOAP platform facade.

use super::core::{arity_error, conversion_error};
use crate::Value;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinResult, RuntimeSourceSpan,
};
use crate::convert::to_bool;

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "is_soap_fault",
        builtin_is_soap_fault,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "use_soap_error_handler",
        builtin_use_soap_error_handler,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_is_soap_fault(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() != 1 {
        return Err(arity_error("is_soap_fault", "one argument"));
    }

    let is_fault = match &args[0] {
        Value::Object(object) => is_soap_fault_class(&object.class_name()),
        _ => false,
    };
    Ok(Value::Bool(is_fault))
}

fn builtin_use_soap_error_handler(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error(
            "use_soap_error_handler",
            "zero or one argument",
        ));
    }

    let enabled = match args.first() {
        Some(value) => {
            to_bool(value).map_err(|message| conversion_error("use_soap_error_handler", message))?
        }
        None => true,
    };
    let previous = context.soap_state().set_error_handler_enabled(enabled);
    Ok(Value::Bool(previous))
}

fn is_soap_fault_class(class_name: &str) -> bool {
    let class_name = class_name.trim_start_matches('\\');
    class_name.eq_ignore_ascii_case("SoapFault")
        || class_name.eq_ignore_ascii_case("Soap\\SoapFault")
}
