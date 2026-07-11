//! Array sorting argument, reference, and value helpers.

use super::builtin_adapter::builtin_source_span;
use super::prelude::*;

pub(super) fn array_callback_key_value(key: &ArrayKey) -> Value {
    match key {
        ArrayKey::Int(index) => Value::Int(*index),
        ArrayKey::String(value) => Value::String(value.clone()),
    }
}

pub(super) fn sort_reference_cell(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    function: &str,
    arg: CallArgument,
    stack: &mut CallStack,
) -> Result<ReferenceCell, ArrayCallbackError> {
    sort_reference_cell_at(compiled, state, function, arg, stack, 1)
}

fn sort_reference_cell_at(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    function: &str,
    arg: CallArgument,
    stack: &mut CallStack,
    position: usize,
) -> Result<ReferenceCell, ArrayCallbackError> {
    if let Some(cell) = call_argument_reference_cell(compiled, Some(state), &arg, stack)
        .map_err(ArrayCallbackError::Message)?
    {
        return Ok(cell);
    }
    match arg.value {
        Value::Reference(cell) => Ok(cell),
        other => Err(ArrayCallbackError::Message(format!(
            "E_PHP_VM_SORT_BY_REF_ARG: {function} argument #{position} must be a mutable array variable, {} given",
            value_type_name(&other)
        ))),
    }
}

pub(super) fn sort_callback_args(
    name: &str,
    left: &(ArrayKey, Value),
    right: &(ArrayKey, Value),
) -> Vec<Value> {
    if name == "uksort" {
        vec![
            array_callback_key_value(&left.0),
            array_callback_key_value(&right.0),
        ]
    } else {
        vec![left.1.clone(), right.1.clone()]
    }
}

pub(super) fn sort_callback_ordering(
    name: &str,
    result: Value,
    reverse: bool,
) -> Result<std::cmp::Ordering, ArrayCallbackError> {
    let int = to_int(&result)
        .map_err(|message| ArrayCallbackError::Message(format!("{name}: {message}")))?;
    let ordering = int.cmp(&0);
    Ok(if reverse {
        ordering.reverse()
    } else {
        ordering
    })
}

pub(super) fn emit_sort_bool_compare_deprecation(
    compiled: &CompiledUnit,
    name: &str,
    output: &mut OutputBuffer,
    stack: &CallStack,
    state: &mut ExecutionState,
    emitted: &mut bool,
) {
    if *emitted {
        return;
    }
    *emitted = true;
    let diagnostic = RuntimeDiagnostic::new(
        "E_PHP_VM_SORT_BOOL_COMPARE_DEPRECATED",
        RuntimeSeverity::Deprecation,
        format!(
            "{name}(): Returning bool from comparison function is deprecated, return an integer less than, equal to, or greater than zero"
        ),
        builtin_source_span(compiled, None),
        stack_trace(compiled, stack),
        None,
    );
    emit_vm_diagnostic(
        output,
        state,
        &diagnostic,
        php_runtime::PhpDiagnosticChannel::Deprecated,
        php_runtime::PHP_E_DEPRECATED,
    );
    state.diagnostics.push(diagnostic);
}

pub(super) fn multisort_reference_cell_at(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    _function: &str,
    arg: CallArgument,
    stack: &mut CallStack,
    _position: usize,
) -> Result<ReferenceCell, ArrayCallbackError> {
    if let Some(cell) = call_argument_reference_cell(compiled, Some(state), &arg, stack)
        .map_err(ArrayCallbackError::Message)?
    {
        return Ok(cell);
    }
    match arg.value {
        Value::Reference(cell) => Ok(cell),
        other => Ok(ReferenceCell::new(other)),
    }
}

pub(super) fn sort_argument_is_array(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    arg: &CallArgument,
    stack: &mut CallStack,
) -> Result<bool, ArrayCallbackError> {
    if let Some(cell) = call_argument_reference_cell(compiled, Some(state), arg, stack)
        .map_err(ArrayCallbackError::Message)?
    {
        return Ok(effective_is_array(&Value::Reference(cell)));
    }
    Ok(effective_is_array(&arg.value))
}

pub(super) fn multisort_array_entries(
    function: &str,
    position: usize,
    value: &Value,
) -> Result<Vec<(ArrayKey, Value)>, ArrayCallbackError> {
    match value {
        Value::Array(array) => Ok(array
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()),
        Value::Int(flag) if matches!(*flag, SORT_REGULAR | SORT_NUMERIC) => {
            Err(ArrayCallbackError::Message(format!(
                "E_PHP_RUNTIME_BUILTIN_VALUE: {function}(): Argument #{position} ($array) must be an array or a sort flag that has not already been specified"
            )))
        }
        Value::Int(_) => Err(ArrayCallbackError::Message(format!(
            "E_PHP_RUNTIME_BUILTIN_VALUE: {function}(): Argument #{position} ($array) must be a valid sort flag"
        ))),
        _ => Err(ArrayCallbackError::BuiltinTypeMessage(format!(
            "{function}(): Argument #{position} ($array) must be an array or a sort flag"
        ))),
    }
}

pub(super) fn multisort_duplicate_flag_error(
    function: &str,
    position: usize,
) -> ArrayCallbackError {
    ArrayCallbackError::BuiltinTypeMessage(format!(
        "{function}(): Argument #{position} must be an array or a sort flag that has not already been specified"
    ))
}

pub(super) fn sort_numeric_float(
    value: &Value,
    output: &mut OutputBuffer,
    state: &mut ExecutionState,
    source_span: RuntimeSourceSpan,
) -> Result<f64, ArrayCallbackError> {
    match value {
        Value::Reference(cell) => sort_numeric_float(&cell.get(), output, state, source_span),
        Value::Object(object) => {
            write_object_numeric_cast_warning(output, state, object, "float", source_span);
            Ok(1.0)
        }
        other => to_float(other)
            .map_err(|message| ArrayCallbackError::Message(format!("array_multisort: {message}"))),
    }
}

pub(super) fn multisort_numeric_values(
    entries: &[(ArrayKey, Value)],
    output: &mut OutputBuffer,
    state: &mut ExecutionState,
    source_span: RuntimeSourceSpan,
) -> Result<Vec<f64>, ArrayCallbackError> {
    entries
        .iter()
        .map(|(_, value)| multisort_numeric_value(value, output, state, source_span.clone()))
        .collect()
}

fn multisort_numeric_value(
    value: &Value,
    output: &mut OutputBuffer,
    state: &mut ExecutionState,
    source_span: RuntimeSourceSpan,
) -> Result<f64, ArrayCallbackError> {
    match value {
        Value::Reference(cell) => multisort_numeric_value(&cell.get(), output, state, source_span),
        Value::Object(object) => {
            write_object_numeric_cast_warning(output, state, object, "float", source_span.clone());
            write_object_numeric_cast_warning(output, state, object, "float", source_span);
            Ok(1.0)
        }
        other => to_float(other)
            .map_err(|message| ArrayCallbackError::Message(format!("array_multisort: {message}"))),
    }
}

pub(super) fn multisort_reorder_entries(
    entries: &[(ArrayKey, Value)],
    order: &[usize],
) -> PhpArray {
    let mut sorted = PhpArray::new();
    for index in order {
        let (key, value) = &entries[*index];
        match key {
            ArrayKey::String(_) => {
                sorted.insert(key.clone(), value.clone());
            }
            ArrayKey::Int(_) => {
                sorted.append(value.clone());
            }
        }
    }
    sorted
}
