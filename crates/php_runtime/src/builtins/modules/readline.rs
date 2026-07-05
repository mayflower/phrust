//! Noninteractive readline compatibility slice.

use super::core::{argument_type_error, arity_error, deref_value, string_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use crate::{ArrayKey, CallableValue, PhpArray, PhpString, Value};
use std::fs;
use std::path::PathBuf;

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("readline", builtin_readline, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "readline_add_history",
        builtin_readline_add_history,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_callback_handler_install",
        builtin_readline_callback_handler_install,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_callback_handler_remove",
        builtin_readline_callback_handler_remove,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_callback_read_char",
        builtin_readline_callback_read_char,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_clear_history",
        builtin_readline_clear_history,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_completion_function",
        builtin_readline_completion_function,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_info",
        builtin_readline_info,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_list_history",
        builtin_readline_list_history,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_on_new_line",
        builtin_readline_on_new_line,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_read_history",
        builtin_readline_read_history,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_redisplay",
        builtin_readline_redisplay,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "readline_write_history",
        builtin_readline_write_history,
        BuiltinCompatibility::Php,
    ),
];

fn builtin_readline(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("readline", "zero or one argument"));
    }
    if let Some(prompt) = args.first()
        && !matches!(prompt, Value::Null)
    {
        let _ = string_arg("readline", prompt)?;
    }
    Ok(Value::Bool(false))
}

fn builtin_readline_info(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 2 {
        return Err(arity_error("readline_info", "zero, one, or two arguments"));
    }
    if args.is_empty() || matches!(args[0], Value::Null) {
        let mut array = PhpArray::new();
        for (key, value) in context.readline_state().info() {
            array.insert(string_key(key), value.clone());
        }
        return Ok(Value::Array(array));
    }
    let name = string_arg("readline_info", &args[0])?.to_string_lossy();
    if let Some(value) = args.get(1) {
        let previous = context
            .readline_state()
            .set_info_value(name.to_string(), value.clone())
            .unwrap_or(Value::Null);
        return Ok(previous);
    }
    Ok(context
        .readline_state()
        .info_value(&name)
        .unwrap_or(Value::Null))
}

fn builtin_readline_add_history(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_add_history", &args, 1)?;
    let entry = string_arg("readline_add_history", &args[0])?.to_string_lossy();
    context.readline_state().add_history(entry.to_string());
    Ok(Value::Bool(true))
}

fn builtin_readline_clear_history(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_clear_history", &args, 0)?;
    context.readline_state().clear_history();
    Ok(Value::Bool(true))
}

fn builtin_readline_list_history(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_list_history", &args, 0)?;
    Ok(Value::Array(PhpArray::from_packed(
        context
            .readline_state()
            .history()
            .iter()
            .map(|entry| Value::string(entry.as_str()))
            .collect(),
    )))
}

fn builtin_readline_read_history(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("readline_read_history", "zero or one argument"));
    }
    let Some(path) = optional_path("readline_read_history", args.first())? else {
        return Ok(Value::Bool(false));
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(Value::Bool(false));
    };
    context
        .readline_state()
        .set_history(contents.lines().map(ToOwned::to_owned).collect());
    Ok(Value::Bool(true))
}

fn builtin_readline_write_history(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error(
            "readline_write_history",
            "zero or one argument",
        ));
    }
    let Some(path) = optional_path("readline_write_history", args.first())? else {
        return Ok(Value::Bool(false));
    };
    let mut contents = String::new();
    for entry in context.readline_state().history() {
        contents.push_str(entry);
        contents.push('\n');
    }
    Ok(Value::Bool(fs::write(path, contents).is_ok()))
}

fn builtin_readline_completion_function(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_completion_function", &args, 1)?;
    let callback = callback_arg("readline_completion_function", "#1 ($callback)", &args[0])?;
    context.readline_state().set_completion_callback(callback);
    Ok(Value::Bool(true))
}

fn builtin_readline_callback_handler_install(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_callback_handler_install", &args, 2)?;
    let prompt = string_arg("readline_callback_handler_install", &args[0])?.to_string_lossy();
    let callback = callback_arg(
        "readline_callback_handler_install",
        "#2 ($callback)",
        &args[1],
    )?;
    context
        .readline_state()
        .install_callback_handler(prompt.to_string(), callback);
    Ok(Value::Bool(true))
}

fn builtin_readline_callback_read_char(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_callback_read_char", &args, 0)?;
    Ok(Value::Null)
}

fn builtin_readline_callback_handler_remove(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_callback_handler_remove", &args, 0)?;
    Ok(Value::Bool(
        context.readline_state().remove_callback_handler(),
    ))
}

fn builtin_readline_redisplay(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_redisplay", &args, 0)?;
    Ok(Value::Null)
}

fn builtin_readline_on_new_line(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("readline_on_new_line", &args, 0)?;
    Ok(Value::Null)
}

fn optional_path(name: &str, value: Option<&Value>) -> Result<Option<PathBuf>, BuiltinError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(PathBuf::from(
            string_arg(name, value)?.to_string_lossy(),
        ))),
    }
}

fn callback_arg(name: &str, argument: &str, value: &Value) -> Result<String, BuiltinError> {
    let value = deref_value(value);
    match &value {
        Value::String(callback) => Ok(callback.to_string_lossy()),
        other => match other.as_callable() {
            Some(CallableValue::UserFunction { name })
            | Some(CallableValue::InternalBuiltin { name }) => Ok(name.clone()),
            Some(callable) => Ok(format!("{callable:?}")),
            None => Err(argument_type_error(name, argument, "callable", &value)),
        },
    }
}

fn expect_exact(name: &str, args: &[Value], expected: usize) -> Result<(), BuiltinError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(arity_error(name, &format!("exactly {expected} arguments")))
}

fn string_key(key: &str) -> ArrayKey {
    ArrayKey::String(PhpString::from(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputBuffer;

    fn call(context: &mut BuiltinContext<'_>, name: &str, args: Vec<Value>) -> Value {
        ENTRIES
            .iter()
            .find(|entry| entry.name() == name)
            .expect("readline entry")
            .function()(context, args, RuntimeSourceSpan::default())
        .expect("readline succeeds")
    }

    #[test]
    fn readline_history_info_and_callbacks_are_request_local() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        assert_eq!(call(&mut context, "readline", vec![]), Value::Bool(false));
        assert_eq!(
            call(
                &mut context,
                "readline_add_history",
                vec![Value::string("foo")]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                &mut context,
                "readline_add_history",
                vec![Value::string("")]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(&mut context, "readline_list_history", vec![]),
            Value::Array(PhpArray::from_packed(vec![
                Value::string("foo"),
                Value::string("")
            ]))
        );
        assert_eq!(
            call(
                &mut context,
                "readline_info",
                vec![Value::string("line_buffer"), Value::string("abc")]
            ),
            Value::string("")
        );
        assert_eq!(
            call(
                &mut context,
                "readline_info",
                vec![Value::string("line_buffer")]
            ),
            Value::string("abc")
        );
        assert_eq!(
            call(
                &mut context,
                "readline_completion_function",
                vec![Value::string("complete")]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                &mut context,
                "readline_callback_handler_install",
                vec![Value::string("> "), Value::string("handler")]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(&mut context, "readline_callback_handler_remove", vec![]),
            Value::Bool(true)
        );
        assert_eq!(
            call(&mut context, "readline_callback_handler_remove", vec![]),
            Value::Bool(false)
        );
        assert_eq!(
            call(&mut context, "readline_clear_history", vec![]),
            Value::Bool(true)
        );
        assert_eq!(
            call(&mut context, "readline_list_history", vec![]),
            Value::Array(PhpArray::new())
        );
    }
}
