use super::prelude::*;

impl Vm {
    pub(super) fn throw_exception_result(
        &self,
        compiled: &CompiledUnit,
        output: &mut OutputBuffer,
        stack: &CallStack,
        state: &mut ExecutionState,
        span: php_ir::IrSpan,
        message: String,
    ) -> VmResult {
        let message_value = Value::string(message.into_bytes());
        let throwable = match make_exception_object("Exception", &message_value) {
            Ok(object) => Value::Object(object),
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        tag_throwable_location(&throwable, compiled, span);
        state.pending_trace = Some(capture_backtrace_string(compiled, stack));
        state.pending_throw = Some(throwable);
        VmResult::propagating_exception(output.clone())
    }

    pub(super) fn throw_catchable_exception(
        &self,
        compiled: &CompiledUnit,
        output: &mut OutputBuffer,
        stack: &CallStack,
        state: &mut ExecutionState,
        message: String,
    ) -> VmResult {
        let message_value = Value::string(message.into_bytes());
        let throwable = match make_exception_object("Exception", &message_value) {
            Ok(object) => Value::Object(object),
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        state.pending_trace = Some(capture_backtrace_string(compiled, stack));
        state.pending_throw = Some(throwable);
        VmResult::propagating_exception(output.clone())
    }
}
