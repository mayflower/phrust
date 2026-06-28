//! Session builtin registry slice.

use super::core::*;
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use crate::{PHP_SESSION_NONE, Value};

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new(
        "session_destroy",
        builtin_session_destroy,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("session_id", builtin_session_id, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "session_name",
        builtin_session_name,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "session_start",
        builtin_session_start,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "session_status",
        builtin_session_status,
        BuiltinCompatibility::Php,
    ),
];

pub(in crate::builtins::modules) fn builtin_session_status(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("session_status", &args, 0)?;
    let status = context
        .session_state()
        .map_or(PHP_SESSION_NONE, |state| state.status());
    Ok(Value::Int(status))
}

pub(in crate::builtins::modules) fn builtin_session_name(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("session_name", "zero or one argument(s)"));
    }
    let Some(state) = context.session_state() else {
        return Err(session_context_error("session_name"));
    };
    let previous = state.name().to_owned();
    if let Some(name) = args.first()
        && !matches!(deref_value(name), Value::Null)
    {
        state.replace_name(string_arg("session_name", name)?.to_string_lossy());
    }
    Ok(Value::string(previous))
}

pub(in crate::builtins::modules) fn builtin_session_id(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("session_id", "zero or one argument(s)"));
    }
    let Some(state) = context.session_state() else {
        return Err(session_context_error("session_id"));
    };
    let previous = state.id().to_owned();
    if let Some(id) = args.first()
        && !matches!(deref_value(id), Value::Null)
    {
        state.replace_id(string_arg("session_id", id)?.to_string_lossy());
    }
    Ok(Value::string(previous))
}

pub(in crate::builtins::modules) fn builtin_session_start(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    if args.len() > 1 {
        return Err(arity_error("session_start", "zero or one argument(s)"));
    }
    if let Some(options) = args.first()
        && !matches!(deref_value(options), Value::Array(_))
    {
        return Err(type_error("session_start", "array", options));
    }
    let Some(state) = context.session_state() else {
        return Err(session_context_error("session_start"));
    };
    state.start();
    context.sync_session_global_from_state();
    Ok(Value::Bool(true))
}

pub(in crate::builtins::modules) fn builtin_session_destroy(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_arity("session_destroy", &args, 0)?;
    let Some(state) = context.session_state() else {
        return Err(session_context_error("session_destroy"));
    };
    let destroyed = state.destroy();
    context.sync_session_global_from_state();
    Ok(Value::Bool(destroyed))
}

fn session_context_error(function: &str) -> BuiltinError {
    BuiltinError::new(
        "E_PHP_RUNTIME_SESSION_CONTEXT_REQUIRED",
        format!("{function}() requires VM request-local session state"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayKey, OutputBuffer, PhpArray, PhpString, ReferenceCell, SessionState};

    fn context_with_session<'a>(
        output: &'a mut OutputBuffer,
        state: &'a mut SessionState,
        global: ReferenceCell,
    ) -> BuiltinContext<'a> {
        let mut context = BuiltinContext::new(output);
        context.set_session_state(state, global);
        context
    }

    #[test]
    fn session_builtins_track_cli_state() {
        let mut output = OutputBuffer::new();
        let mut state = SessionState::default();
        let global = ReferenceCell::new(Value::Array(PhpArray::new()));
        let mut context = context_with_session(&mut output, &mut state, global.clone());

        assert_eq!(
            builtin_session_status(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("status"),
            Value::Int(PHP_SESSION_NONE)
        );
        assert_eq!(
            builtin_session_name(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("name"),
            Value::string("PHPSESSID")
        );
        assert_eq!(
            builtin_session_id(
                &mut context,
                vec![Value::string("local")],
                RuntimeSourceSpan::default()
            )
            .expect("id"),
            Value::string("")
        );
        assert_eq!(
            builtin_session_start(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("start"),
            Value::Bool(true)
        );
        assert_eq!(global.get(), Value::Array(crate::PhpArray::new()));
        assert_eq!(
            builtin_session_id(&mut context, Vec::new(), RuntimeSourceSpan::default()).expect("id"),
            Value::string("local")
        );
        assert_eq!(
            builtin_session_destroy(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("destroy"),
            Value::Bool(true)
        );
    }

    #[test]
    fn session_builtins_use_seeded_web_state() {
        let mut seeded = PhpArray::new();
        seeded.insert(ArrayKey::String(PhpString::from("n")), Value::Int(7));
        let mut output = OutputBuffer::new();
        let mut state = SessionState::seeded(
            "APPSESSID".to_string(),
            "incoming123".to_string(),
            seeded.clone(),
            Some("generated456".to_string()),
        );
        let global = ReferenceCell::new(Value::Array(seeded));
        let mut context = context_with_session(&mut output, &mut state, global.clone());

        assert_eq!(
            builtin_session_name(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("name"),
            Value::string("APPSESSID")
        );
        assert_eq!(
            builtin_session_start(&mut context, Vec::new(), RuntimeSourceSpan::default())
                .expect("start"),
            Value::Bool(true)
        );
        assert_eq!(
            builtin_session_id(&mut context, Vec::new(), RuntimeSourceSpan::default()).expect("id"),
            Value::string("incoming123")
        );
        assert_eq!(
            global.get(),
            Value::Array({
                let mut array = PhpArray::new();
                array.insert(ArrayKey::String(PhpString::from("n")), Value::Int(7));
                array
            })
        );
        assert!(!state.newly_created());
    }
}
