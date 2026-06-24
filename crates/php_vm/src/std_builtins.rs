//! Minimal VM bridge for the standard-library `php_std` builtin ABI.

use php_runtime::{OutputBuffer, RuntimeSourceSpan as VmRuntimeSourceSpan, Value};
use php_std::abi::{
    BuiltinFunction, CallArgument, CallContext, RequestContext, ReturnValue, call_builtin,
};

use crate::vm::VmResult;

/// Calls a `php_std` builtin through the standard-library ABI.
///
/// This is intentionally not wired into all VM builtin dispatch yet. It proves
/// the ABI boundary and `VmResult` conversion while existing runtime-semantics builtins
/// continue to use their current runtime path.
pub fn call_php_std_builtin(
    builtin: &impl BuiltinFunction,
    args: Vec<Value>,
    source_span: VmRuntimeSourceSpan,
) -> VmResult {
    let mut output = OutputBuffer::new();
    let request = RequestContext::cli(".", source_span.file.clone());
    let call_args = args.into_iter().map(CallArgument::by_value).collect();
    let mut context = CallContext::new(
        builtin.name(),
        call_args,
        source_span,
        &mut output,
        &request,
    );

    match call_builtin(builtin, &mut context) {
        Ok(ReturnValue::Value(value)) => VmResult::success(context.output().clone(), Some(value)),
        Ok(ReturnValue::Void) => VmResult::success(context.output().clone(), Some(Value::Null)),
        Err(error) => {
            let diagnostic = error.diagnostic().clone();
            let message = format!("{}: {}", diagnostic.id(), diagnostic.message());
            VmResult::runtime_error_with_diagnostic(context.output().clone(), message, diagnostic)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_runtime::PhpString;
    use php_std::abi::{BuiltinMetadata, BuiltinResult, RegisteredBuiltin};

    fn test_builtin_echo_like(context: &mut CallContext<'_>) -> BuiltinResult {
        let mut bytes = 0;
        for arg in context.args().to_vec() {
            if let Value::String(text) = arg.value() {
                bytes += text.as_bytes().len();
                context.output().write_php_string(text);
            } else {
                return Err(context.fatal("E_PHP_STD_TEST_EXPECTED_STRING", "expected string"));
            }
        }
        Ok(ReturnValue::Value(Value::Int(bytes as i64)))
    }

    fn test_builtin_fails_with_span(context: &mut CallContext<'_>) -> BuiltinResult {
        Err(context.fatal("E_PHP_STD_TEST_FAILURE", "test failure"))
    }

    #[test]
    fn std_bridge_executes_test_builtin_with_output() {
        let builtin = RegisteredBuiltin::new(
            "__php_std_test_echo_like",
            test_builtin_echo_like,
            BuiltinMetadata {
                variadic: true,
                by_ref_params: &[],
            },
        );
        let result = call_php_std_builtin(
            &builtin,
            vec![Value::String(PhpString::from_test_str("hello"))],
            VmRuntimeSourceSpan {
                file: Some("fixture.php".to_owned()),
                start: 4,
                end: 12,
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"hello");
        assert_eq!(result.return_value, Some(Value::Int(5)));
    }

    #[test]
    fn std_bridge_preserves_diagnostic_source_span() {
        let builtin = RegisteredBuiltin::new(
            "__php_std_test_fails",
            test_builtin_fails_with_span,
            BuiltinMetadata::default(),
        );
        let result = call_php_std_builtin(
            &builtin,
            Vec::new(),
            VmRuntimeSourceSpan {
                file: Some("fixture.php".to_owned()),
                start: 10,
                end: 20,
            },
        );

        assert!(!result.status.is_success(), "{:?}", result.status);
        let span = result
            .diagnostics
            .first()
            .expect("bridge diagnostic")
            .source_span();
        assert_eq!(span.file.as_deref(), Some("fixture.php"));
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
    }
}
