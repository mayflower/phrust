//! Deterministic System V shared variable compatibility slice.

use super::core::{argument_type_error, arity_error, int_arg};
use crate::builtins::{
    BuiltinCompatibility, BuiltinContext, BuiltinEntry, BuiltinError, BuiltinResult,
    RuntimeSourceSpan,
};
use crate::{ClassEntry, ClassFlags, ObjectRef, Value, normalize_class_name};

const SHM_CLASS: &str = "SysvSharedMemory";
const SHM_ID_PROPERTY: &str = "__sysvshm_id";

pub(in crate::builtins) const ENTRIES: &[BuiltinEntry] = &[
    BuiltinEntry::new("shm_attach", builtin_shm_attach, BuiltinCompatibility::Php),
    BuiltinEntry::new("shm_detach", builtin_shm_detach, BuiltinCompatibility::Php),
    BuiltinEntry::new(
        "shm_has_var",
        builtin_shm_has_var,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "shm_put_var",
        builtin_shm_put_var,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "shm_get_var",
        builtin_shm_get_var,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new(
        "shm_remove_var",
        builtin_shm_remove_var,
        BuiltinCompatibility::Php,
    ),
    BuiltinEntry::new("shm_remove", builtin_shm_remove, BuiltinCompatibility::Php),
];

fn builtin_shm_attach(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_between("shm_attach", &args, 1, 3)?;
    let key = int_arg("shm_attach", &args[0])?;
    let size = args
        .get(1)
        .filter(|value| !matches!(value, Value::Null))
        .map_or(Ok(10_000), |value| int_arg("shm_attach", value))?;
    let permissions = optional_int("shm_attach", &args, 2, 0o666)?;
    let id = context.sysvshm_state().attach(key, size, permissions);
    Ok(Value::Object(shm_object(id)))
}

fn builtin_shm_detach(
    _context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_detach", &args, 1)?;
    let _ = shm_id("shm_detach", &args[0])?;
    Ok(Value::Bool(true))
}

fn builtin_shm_has_var(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_has_var", &args, 2)?;
    let shm_id = shm_id("shm_has_var", &args[0])?;
    let key = int_arg("shm_has_var", &args[1])?;
    Ok(Value::Bool(
        context
            .sysvshm_state()
            .segment(shm_id)
            .is_some_and(|segment| segment.has(key)),
    ))
}

fn builtin_shm_put_var(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_put_var", &args, 3)?;
    let shm_id = shm_id("shm_put_var", &args[0])?;
    let key = int_arg("shm_put_var", &args[1])?;
    let Some(segment) = context.sysvshm_state().segment_mut(shm_id) else {
        return Ok(Value::Bool(false));
    };
    segment.put(key, args[2].clone());
    Ok(Value::Bool(true))
}

fn builtin_shm_get_var(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_get_var", &args, 2)?;
    let shm_id = shm_id("shm_get_var", &args[0])?;
    let key = int_arg("shm_get_var", &args[1])?;
    let Some(segment) = context.sysvshm_state().segment(shm_id) else {
        return Err(BuiltinError::new(
            "E_PHP_RUNTIME_SYSVSHM_INVALID",
            "shm_get_var(): SysvSharedMemory object is no longer valid",
        ));
    };
    segment.get(key).ok_or_else(|| {
        BuiltinError::new(
            "E_PHP_RUNTIME_SYSVSHM_KEY",
            format!("shm_get_var(): Variable key {key} does not exist"),
        )
    })
}

fn builtin_shm_remove_var(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_remove_var", &args, 2)?;
    let shm_id = shm_id("shm_remove_var", &args[0])?;
    let key = int_arg("shm_remove_var", &args[1])?;
    let Some(segment) = context.sysvshm_state().segment_mut(shm_id) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(segment.remove_var(key)))
}

fn builtin_shm_remove(
    context: &mut BuiltinContext<'_>,
    args: Vec<Value>,
    _span: RuntimeSourceSpan,
) -> BuiltinResult {
    expect_exact("shm_remove", &args, 1)?;
    let shm_id = shm_id("shm_remove", &args[0])?;
    Ok(Value::Bool(context.sysvshm_state().remove(shm_id)))
}

fn expect_exact(name: &str, args: &[Value], expected: usize) -> Result<(), BuiltinError> {
    expect_between(name, args, expected, expected)
}

fn expect_between(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), BuiltinError> {
    if (min..=max).contains(&args.len()) {
        Ok(())
    } else {
        Err(arity_error(
            name,
            &format!("between {min} and {max} arguments"),
        ))
    }
}

fn optional_int(
    name: &str,
    args: &[Value],
    index: usize,
    default: i64,
) -> Result<i64, BuiltinError> {
    args.get(index)
        .map_or(Ok(default), |value| int_arg(name, value))
}

fn shm_id(name: &str, value: &Value) -> Result<i64, BuiltinError> {
    let Value::Object(object) = value else {
        return Err(argument_type_error(name, "#1 ($shm)", SHM_CLASS, value));
    };
    if normalize_class_name(&object.class_name()) != "sysvsharedmemory" {
        return Err(argument_type_error(name, "#1 ($shm)", SHM_CLASS, value));
    }
    match object.get_property(SHM_ID_PROPERTY) {
        Some(Value::Int(id)) if id > 0 => Ok(id),
        _ => Err(BuiltinError::new(
            "E_PHP_RUNTIME_SYSVSHM_INVALID",
            format!("{name}(): SysvSharedMemory object is no longer valid"),
        )),
    }
}

fn shm_object(id: i64) -> ObjectRef {
    let object = ObjectRef::new_with_display_name(&runtime_class(SHM_CLASS), SHM_CLASS);
    object.set_property(SHM_ID_PROPERTY, Value::Int(id));
    object
}

fn runtime_class(name: &str) -> ClassEntry {
    ClassEntry {
        name: normalize_class_name(name),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OutputBuffer;

    #[test]
    fn shared_memory_stores_variables_by_key_and_removes_segment() {
        let mut output = OutputBuffer::new();
        let mut context = BuiltinContext::new(&mut output);
        let shm = builtin_shm_attach(
            &mut context,
            vec![Value::Int(456), Value::Int(1024), Value::Int(0o600)],
            RuntimeSourceSpan::default(),
        )
        .expect("attach");

        assert_eq!(
            builtin_shm_put_var(
                &mut context,
                vec![shm.clone(), Value::Int(1), Value::string("value")],
                RuntimeSourceSpan::default(),
            )
            .expect("put"),
            Value::Bool(true)
        );
        assert_eq!(
            builtin_shm_has_var(
                &mut context,
                vec![shm.clone(), Value::Int(1)],
                RuntimeSourceSpan::default(),
            )
            .expect("has"),
            Value::Bool(true)
        );
        assert_eq!(
            builtin_shm_get_var(
                &mut context,
                vec![shm.clone(), Value::Int(1)],
                RuntimeSourceSpan::default(),
            )
            .expect("get"),
            Value::string("value")
        );
        assert_eq!(
            builtin_shm_remove(&mut context, vec![shm], RuntimeSourceSpan::default(),)
                .expect("remove"),
            Value::Bool(true)
        );
    }
}
