// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_unary_fallback(
    _runtime: *mut std::ffi::c_void,
    op: u32,
    src: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    let value = match op {
        0 => src,
        1 => match src.checked_neg() {
            Some(value) => value,
            None => return crate::JitCallStatus::RUNTIME_ERROR.0 as i32,
        },
        2 => i64::from(src == 0),
        3 => !src,
        _ => return crate::JitCallStatus::ABI_MISMATCH.0 as i32,
    };
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(value) };
    0
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_binary_fallback(
    _runtime: *mut std::ffi::c_void,
    op: u32,
    lhs: i64,
    rhs: i64,
    _function: i64,
    _continuation: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    let value = match op {
        0 => lhs.checked_add(rhs),
        1 => lhs.checked_sub(rhs),
        2 => lhs.checked_mul(rhs),
        3 if rhs != 0 && lhs % rhs == 0 => Some(lhs / rhs),
        4 if rhs != 0 => Some(lhs % rhs),
        _ => None,
    };
    let Some(value) = value else {
        return crate::JitCallStatus::RECOMPILE_REQUESTED.0 as i32;
    };
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(value) };
    0
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_compare_fallback(
    _runtime: *mut std::ffi::c_void,
    op: u32,
    lhs: i64,
    rhs: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    let value = match op {
        0 | 2 => i64::from(lhs == rhs),
        1 | 3 => i64::from(lhs != rhs),
        4 => i64::from(lhs < rhs),
        5 => i64::from(lhs <= rhs),
        6 => i64::from(lhs > rhs),
        7 => i64::from(lhs >= rhs),
        8 => match lhs.cmp(&rhs) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        _ => return crate::JitCallStatus::ABI_MISMATCH.0 as i32,
    };
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(value) };
    0
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_cast_fallback(
    _runtime: *mut std::ffi::c_void,
    op: u32,
    src: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    let value = match op {
        0 => i64::from(src != 0),
        1 => src,
        _ => return crate::JitCallStatus::RUNTIME_ERROR.0 as i32,
    };
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(value) };
    0
}

pub(super) extern "C" fn test_native_echo_fallback(
    _runtime: *mut std::ffi::c_void,
    _src: i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_local_fetch_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    value: i64,
    _function: i64,
    _local: i64,
    _file: i64,
    _start: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        // SAFETY: Cranelift owns this synchronous stack output slot.
        unsafe { out.write(value) };
        0
    }
}

pub(super) extern "C" fn test_native_exception_new_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    message: i64,
    _function: i64,
    _continuation: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        // SAFETY: Cranelift owns this synchronous stack output slot.
        unsafe { out.write(message) };
        0
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_local_store_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _current: i64,
    value: i64,
    _function: i64,
    _local: i64,
    out: *mut i64,
) -> i32 {
    if !out.is_null() {
        unsafe { out.write(value) };
        0
    } else {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_value_lifecycle_fallback(
    _runtime: *mut std::ffi::c_void,
    op: u32,
    value: i64,
    out: *mut i64,
) -> i32 {
    let op = if op & 0x8000_0000 != 0 { op & 1 } else { op };
    if op > 1 {
        return crate::JitCallStatus::ABI_MISMATCH.0 as i32;
    }
    if !out.is_null() {
        unsafe { out.write(value) };
        0
    } else {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_reference_bind_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    value: i64,
    _key: i64,
    _reserved: i64,
    out: *mut i64,
) -> i32 {
    if !out.is_null() {
        unsafe { out.write(value) };
        0
    } else {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_argument_check_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    value: i64,
    _target_function: i64,
    _parameter_flags: i64,
    _caller_function: i64,
    _continuation: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        unsafe { out.write(value) };
        0
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_return_check_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    value: i64,
    _function: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        unsafe { out.write(value) };
        0
    }
}

pub(super) extern "C" fn test_native_array_new_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_object_new_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_property_fetch_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _object: i64,
    _function: i64,
    _continuation: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_property_assign_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _object: i64,
    _value: i64,
    _function: i64,
    _continuation: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_object_clone_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    object: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        unsafe { out.write(object) };
        0
    }
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_object_clone_with_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    object: i64,
    _replacements: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        crate::JitCallStatus::RUNTIME_ERROR.0 as i32
    } else {
        unsafe { out.write(object) };
        0
    }
}

pub(super) extern "C" fn test_native_array_insert_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _array: i64,
    _key: i64,
    _value: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_array_fetch_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _array: i64,
    _key: i64,
) -> crate::JitNativeValueResult {
    crate::JitNativeValueResult {
        value: crate::jit_encode_constant(u32::MAX),
        status: crate::JitCallStatus::RUNTIME_ERROR.0 as i64,
    }
}

pub(super) extern "C" fn test_native_array_unset_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _array: i64,
    _key: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_array_spread_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _array: i64,
    _source: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_foreach_init_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _source: i64,
    _function: i64,
    _local: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_foreach_next_fallback(
    _runtime: *mut std::ffi::c_void,
    _iterator: i64,
    _key_out: *mut i64,
    _value_out: *mut i64,
    _has_out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_foreach_cleanup_fallback(
    _runtime: *mut std::ffi::c_void,
    _iterator: i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_constant_fetch_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _function: i64,
    _instruction: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_truthy_fallback(
    _runtime: *mut std::ffi::c_void,
    src: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(i64::from(src != 0)) };
    0
}

pub(super) extern "C" fn test_native_type_predicate_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _src: i64,
    out: *mut i64,
) -> i32 {
    if out.is_null() {
        return crate::JitCallStatus::RUNTIME_ERROR.0 as i32;
    }
    // SAFETY: Cranelift owns this synchronous stack output slot.
    unsafe { out.write(crate::jit_encode_constant(crate::JIT_VALUE_FALSE)) };
    0
}

// SAFETY: audited native ABI pointer boundary; see the function-local safety notes.
#[allow(unsafe_code)]
pub(super) extern "C" fn test_native_stable_length_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _value: i64,
    _function: i64,
    _continuation: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_string_predicate_fallback(
    _runtime: *mut std::ffi::c_void,
    _op: u32,
    _haystack: i64,
    _needle: i64,
    _out: *mut i64,
) -> i32 {
    crate::JitCallStatus::ABI_MISMATCH.0 as i32
}

pub(super) extern "C" fn test_native_runtime_fatal_fallback(
    _runtime: *mut std::ffi::c_void,
    _function: u32,
    _instruction: u32,
) -> i32 {
    crate::JitCallStatus::RUNTIME_ERROR.0 as i32
}

pub(super) extern "C" fn test_native_execution_poll_fallback(
    _runtime: *mut std::ffi::c_void,
) -> i32 {
    0
}
macro_rules! register_value_fallback {
    ($wrapper:ident => $target:ident ($($name:ident: $ty:ty),* $(,)?)) => {
        pub(super) extern "C" fn $wrapper(
            runtime: *mut std::ffi::c_void,
            $($name: $ty),*
        ) -> crate::JitNativeValueResult {
            let mut value = crate::jit_encode_constant(u32::MAX);
            let status = $target(runtime, $($name,)* &raw mut value);
            crate::JitNativeValueResult {
                value,
                status: i64::from(status),
            }
        }
    };
}

register_value_fallback!(test_native_unary_register_fallback => test_native_unary_fallback(op: u32, src: i64));
register_value_fallback!(test_native_binary_register_fallback => test_native_binary_fallback(op: u32, lhs: i64, rhs: i64, function: i64, continuation: i64));
register_value_fallback!(test_native_compare_register_fallback => test_native_compare_fallback(op: u32, lhs: i64, rhs: i64));
register_value_fallback!(test_native_cast_register_fallback => test_native_cast_fallback(op: u32, src: i64));
register_value_fallback!(test_native_local_fetch_register_fallback => test_native_local_fetch_fallback(op: u32, value: i64, function: i64, local: i64, file: i64, start: i64));
register_value_fallback!(test_native_local_store_register_fallback => test_native_local_store_fallback(op: u32, current: i64, value: i64, function: i64, local: i64));
register_value_fallback!(test_native_value_lifecycle_register_fallback => test_native_value_lifecycle_fallback(op: u32, value: i64));
register_value_fallback!(test_native_reference_bind_register_fallback => test_native_reference_bind_fallback(op: u32, value: i64, key: i64, reserved: i64));
register_value_fallback!(test_native_argument_check_register_fallback => test_native_argument_check_fallback(op: u32, value: i64, target_function: i64, parameter_flags: i64, caller_function: i64, continuation: i64));
register_value_fallback!(test_native_return_check_register_fallback => test_native_return_check_fallback(op: u32, value: i64, function: i64));
register_value_fallback!(test_native_exception_new_register_fallback => test_native_exception_new_fallback(op: u32, message: i64, function: i64, continuation: i64));
register_value_fallback!(test_native_array_new_register_fallback => test_native_array_new_fallback(op: u32));
register_value_fallback!(test_native_object_new_register_fallback => test_native_object_new_fallback(op: u32));
register_value_fallback!(test_native_property_fetch_register_fallback => test_native_property_fetch_fallback(op: u32, object: i64, function: i64, continuation: i64));
register_value_fallback!(test_native_property_assign_register_fallback => test_native_property_assign_fallback(op: u32, object: i64, value: i64, function: i64, continuation: i64));
register_value_fallback!(test_native_object_clone_register_fallback => test_native_object_clone_fallback(op: u32, object: i64));
register_value_fallback!(test_native_object_clone_with_register_fallback => test_native_object_clone_with_fallback(op: u32, object: i64, replacements: i64));
register_value_fallback!(test_native_array_insert_register_fallback => test_native_array_insert_fallback(op: u32, array: i64, key: i64, value: i64));
register_value_fallback!(test_native_array_unset_register_fallback => test_native_array_unset_fallback(op: u32, array: i64, key: i64));
register_value_fallback!(test_native_array_spread_register_fallback => test_native_array_spread_fallback(op: u32, array: i64, source: i64));
register_value_fallback!(test_native_foreach_init_register_fallback => test_native_foreach_init_fallback(op: u32, source: i64, function: i64, local: i64));
register_value_fallback!(test_native_constant_fetch_register_fallback => test_native_constant_fetch_fallback(op: u32, function: i64, instruction: i64));
register_value_fallback!(test_native_type_predicate_register_fallback => test_native_type_predicate_fallback(op: u32, src: i64));
register_value_fallback!(test_native_stable_length_register_fallback => test_native_stable_length_fallback(op: u32, value: i64, function: i64, continuation: i64));
register_value_fallback!(test_native_string_predicate_register_fallback => test_native_string_predicate_fallback(op: u32, haystack: i64, needle: i64));
