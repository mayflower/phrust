//! First minimal VM dispatch loop.

use crate::compiled_unit::CompiledUnit;
use crate::frame::{CallStack, Frame};
use crate::include::IncludeLoader;
use php_ir::constants::IrConstant;
use php_ir::function::{IrFunction, IrParam, IrReturnType};
use php_ir::ids::{BlockId, ConstId, FunctionId, LocalId};
use php_ir::instruction::{
    BinaryOp, CallableKind, CastKind, ClosureCaptureArg, CompareOp, IncludeKind, Instruction,
    InstructionKind, TerminatorKind, UnaryOp,
};
use php_ir::module::IrUnit;
use php_ir::operand::Operand;
use php_ir::verify::verify_unit;
use php_runtime::{
    ArrayKey, BuiltinContext, BuiltinRegistry, CallableValue, ClassEntry as RuntimeClassEntry,
    ClassFlags as RuntimeClassFlags, ClassMethodEntry as RuntimeClassMethodEntry,
    ClassMethodFlags as RuntimeClassMethodFlags, ClassPropertyEntry as RuntimeClassPropertyEntry,
    ClassPropertyFlags as RuntimeClassPropertyFlags, ClosureCaptureValue, ExecutionStatus,
    NumericValue, ObjectRef, OutputBuffer, PhpArray, PhpString, RuntimeContext, RuntimeDiagnostic,
    RuntimeSeverity, RuntimeSourceSpan, RuntimeStackFrame, RuntimeType, Value, compare,
    division_by_zero_mvp, equal, identical, to_bool, to_number, to_string, undefined_function,
    undefined_variable_warning, unsupported_feature,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ForeachSnapshot {
    entries: Vec<(ArrayKey, Value)>,
    position: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExceptionHandler {
    catch: Option<BlockId>,
    finally: Option<BlockId>,
    after: BlockId,
    exception_local: Option<LocalId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingControl {
    Return(Option<Value>),
    Throw(Value),
}

/// VM execution options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmOptions {
    /// Verify IR before dispatching it.
    pub verify_ir: bool,
    /// Maximum instruction dispatches before reporting a runtime error.
    pub max_steps: usize,
    /// Optional local include loader. When absent, include/require are disabled
    /// with deterministic runtime diagnostics.
    pub include_loader: Option<IncludeLoader>,
    /// Deterministic runtime context used to seed CLI globals and superglobals.
    pub runtime_context: RuntimeContext,
    /// Capture deterministic instruction trace events.
    pub trace: bool,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            verify_ir: true,
            max_steps: 100_000,
            include_loader: None,
            runtime_context: RuntimeContext::default(),
            trace: false,
        }
    }
}

/// Execution result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VmResult {
    /// Final execution status.
    pub status: ExecutionStatus,
    /// Captured stdout bytes.
    pub output: OutputBuffer,
    /// Structured runtime diagnostics emitted during execution.
    pub diagnostics: Vec<RuntimeDiagnostic>,
    /// Return value when execution returned successfully.
    pub return_value: Option<Value>,
    /// Deterministic trace events captured when `VmOptions::trace` is enabled.
    pub trace: Vec<String>,
}

/// VM control-flow signal, kept separate from runtime diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VmControlFlow {
    /// Function return.
    Return(Option<Value>),
    /// Future exception throw signal.
    Throw(Value),
    /// Loop break signal.
    Break,
    /// Loop continue signal.
    Continue,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ExecutionState {
    included_once: Vec<PathBuf>,
}

struct FunctionCall<'a> {
    args: Vec<Value>,
    captures: Vec<ClosureCaptureValue>,
    this_value: Option<ObjectRef>,
    shared_top_level_locals: Option<&'a mut HashMap<String, Value>>,
}

impl FunctionCall<'_> {
    fn new(args: Vec<Value>, captures: Vec<ClosureCaptureValue>) -> Self {
        Self {
            args,
            captures,
            this_value: None,
            shared_top_level_locals: None,
        }
    }

    fn with_this(mut self, this_value: ObjectRef) -> Self {
        self.this_value = Some(this_value);
        self
    }
}

impl VmResult {
    fn success(output: OutputBuffer, return_value: Option<Value>) -> Self {
        Self {
            status: ExecutionStatus::success(),
            output,
            diagnostics: Vec::new(),
            return_value,
            trace: Vec::new(),
        }
    }

    fn success_with_diagnostics(
        output: OutputBuffer,
        return_value: Option<Value>,
        diagnostics: Vec<RuntimeDiagnostic>,
    ) -> Self {
        Self {
            status: ExecutionStatus::success(),
            output,
            diagnostics,
            return_value,
            trace: Vec::new(),
        }
    }

    fn runtime_error_with_diagnostic(
        output: OutputBuffer,
        message: impl Into<String>,
        diagnostic: RuntimeDiagnostic,
    ) -> Self {
        Self {
            status: ExecutionStatus::runtime_error(message),
            output,
            diagnostics: vec![diagnostic],
            return_value: None,
            trace: Vec::new(),
        }
    }

    fn compile_error(output: OutputBuffer, message: impl Into<String>) -> Self {
        Self {
            status: ExecutionStatus::compile_error(message),
            output,
            diagnostics: Vec::new(),
            return_value: None,
            trace: Vec::new(),
        }
    }
}

/// Minimal interpreter VM.
#[derive(Clone, Debug)]
pub struct Vm {
    options: VmOptions,
    trace: RefCell<Vec<String>>,
}

impl Vm {
    /// Creates a VM with default options.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(VmOptions::default())
    }

    /// Creates a VM with explicit options.
    #[must_use]
    pub fn with_options(options: VmOptions) -> Self {
        Self {
            options,
            trace: RefCell::new(Vec::new()),
        }
    }

    /// Executes a compiled unit from its entry function.
    #[must_use]
    pub fn execute(&self, unit: impl Into<CompiledUnit>) -> VmResult {
        let unit = unit.into();
        let mut output = OutputBuffer::new();
        self.trace.borrow_mut().clear();

        if self.options.verify_ir
            && let Err(errors) = verify_unit(unit.unit())
        {
            return VmResult::compile_error(
                output,
                format!("IR verifier failed with {} error(s)", errors.len()),
            );
        }

        let entry = unit.unit().entry;
        if unit.unit().functions.get(entry.index()).is_none() {
            return VmResult::compile_error(output, "entry function is missing");
        }

        let mut stack = CallStack::new();
        let mut state = ExecutionState::default();
        let mut result = self.execute_function(
            &unit,
            entry,
            FunctionCall::new(Vec::new(), Vec::new()),
            &mut output,
            &mut stack,
            &mut state,
        );
        if self.options.trace {
            result.trace = self.trace.borrow().clone();
        }
        result
    }

    fn record_trace_event(
        &self,
        function_id: FunctionId,
        function: &IrFunction,
        stack: &CallStack,
        block_id: BlockId,
        instruction: &Instruction,
        output_len: usize,
    ) {
        let mut trace = self.trace.borrow_mut();
        let step = trace.len() + 1;
        trace.push(format!(
            "step={step} function={}({}) block={} instr={} kind={} stack_depth={} output_len={} locals=[{}] registers=[{}]",
            function.name,
            function_id.raw(),
            block_id.raw(),
            instruction.id.raw(),
            format_instruction_kind(&instruction.kind),
            stack.len(),
            output_len,
            format_locals(function, stack),
            format_registers(stack),
        ));
    }

    fn execute_function(
        &self,
        compiled: &CompiledUnit,
        function_id: FunctionId,
        mut call: FunctionCall<'_>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let unit = compiled.unit();
        let Some(function) = unit.functions.get(function_id.index()) else {
            return self.runtime_error(output, compiled, stack, "called function is missing");
        };
        let mut diagnostics = Vec::new();
        let args = match prepare_arguments(function, call.args) {
            Ok(args) => args,
            Err(message) => {
                return self.runtime_error(output, compiled, stack, message);
            }
        };
        if function.params.iter().any(|param| param.by_ref) {
            return self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_UNSUPPORTED_BY_REF_PARAM: function {} uses by-reference parameters",
                    function.name
                ),
            );
        }
        stack.push(Frame::new(
            function_id,
            function.register_count,
            function.local_count,
        ));
        if let Err(message) = initialize_captures(function, call.captures, stack) {
            let result = self.runtime_error(output, compiled, stack, message);
            stack.pop();
            return result;
        }
        if let Some(this_value) = call.this_value
            && let Err(message) = initialize_this(function, this_value, stack)
        {
            let result = self.runtime_error(output, compiled, stack, message);
            stack.pop();
            return result;
        }
        if let Some(shared) = call.shared_top_level_locals.as_deref_mut() {
            import_shared_locals(function, stack, shared);
        } else if function.flags.is_top_level {
            initialize_runtime_context_locals(function, stack, &self.options.runtime_context);
        }
        for (param, value) in function.params.iter().zip(args) {
            if let Err(message) = check_param_type(function, param, &value) {
                let result = self.runtime_error(output, compiled, stack, message);
                stack.pop();
                return result;
            }
            if let Err(message) = stack
                .current_mut()
                .expect("frame was pushed")
                .locals
                .set(param.local, value)
            {
                let result = self.runtime_error(output, compiled, stack, message);
                stack.pop();
                return result;
            }
        }
        let mut block_id = BlockId::new(0);
        let mut steps = 0usize;
        let mut foreach_iterators = HashMap::new();
        let mut exception_handlers: Vec<ExceptionHandler> = Vec::new();
        let mut pending_control: Option<PendingControl> = None;

        'dispatch: loop {
            steps += 1;
            if steps > self.options.max_steps {
                return self.runtime_error(output, compiled, stack, "VM step limit exceeded");
            }

            let Some(block) = function.blocks.get(block_id.index()) else {
                return self.runtime_error(
                    output,
                    compiled,
                    stack,
                    format!("invalid block block:{}", block_id.raw()),
                );
            };

            for instruction in &block.instructions {
                if self.options.trace {
                    self.record_trace_event(
                        function_id,
                        function,
                        stack,
                        block_id,
                        instruction,
                        output.len(),
                    );
                }
                match &instruction.kind {
                    InstructionKind::Nop => {}
                    InstructionKind::LoadConst { dst, constant } => {
                        let value = match constant_value(unit, *constant) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::FetchConst { dst, name } => {
                        let value = if let Some(constant) = compiled.lookup_constant(name) {
                            inline_constant_value(constant)
                        } else if name == "PHP_VERSION" {
                            Value::String(PhpString::from_test_str(
                                php_source::reference_php_version(),
                            ))
                        } else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_RUNTIME_UNDEFINED_CONSTANT: undefined constant {name}"
                                ),
                            );
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Move { dst, src } => {
                        let value = match read_operand(unit, stack, *src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Binary { dst, op, lhs, rhs } => {
                        let lhs = match read_operand(unit, stack, *lhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let rhs = match read_operand(unit, stack, *rhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match execute_binary(*op, &lhs, &rhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Compare { dst, op, lhs, rhs } => {
                        let lhs = match read_operand(unit, stack, *lhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let rhs = match read_operand(unit, stack, *rhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match execute_compare(*op, &lhs, &rhs) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Unary { dst, op, src } => {
                        let src = match read_operand(unit, stack, *src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match execute_unary(*op, &src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Cast { dst, kind, src } => {
                        let src = match read_operand(unit, stack, *src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match execute_cast(*kind, &src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Discard { src } => {
                        if let Err(message) = read_operand(unit, stack, *src) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::LoadLocal { dst, local } => {
                        let value = match stack
                            .current()
                            .expect("frame was pushed")
                            .locals
                            .get(*local)
                        {
                            Some(Value::Uninitialized) if is_this_local(function, *local) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    "E_PHP_VM_THIS_OUTSIDE_METHOD: $this is not available outside an instance method",
                                );
                            }
                            Some(Value::Uninitialized) => {
                                diagnostics.push(undefined_variable_warning(
                                    format!("local:{}", local.raw()),
                                    RuntimeSourceSpan::default(),
                                    stack_trace(compiled, stack),
                                ));
                                Value::Null
                            }
                            Some(value) => value.clone(),
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!("invalid local local:{}", local.raw()),
                                );
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::LoadLocalQuiet { dst, local } => {
                        let value = match stack
                            .current()
                            .expect("frame was pushed")
                            .locals
                            .get(*local)
                        {
                            Some(Value::Uninitialized) => Value::Null,
                            Some(value) => value.clone(),
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!("invalid local local:{}", local.raw()),
                                );
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::StoreLocal { local, src } => {
                        let value = match read_operand(unit, stack, *src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .set(*local, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::BindReference { target, source } => {
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .bind_reference(*target, *source)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::EnterTry {
                        catch,
                        finally,
                        after,
                        exception_local,
                    } => {
                        exception_handlers.push(ExceptionHandler {
                            catch: *catch,
                            finally: *finally,
                            after: *after,
                            exception_local: *exception_local,
                        });
                    }
                    InstructionKind::LeaveTry => {
                        let _ = exception_handlers.pop();
                    }
                    InstructionKind::EndFinally { after } => match pending_control.take() {
                        Some(PendingControl::Return(value)) => {
                            if let Err(message) = check_return_type(function, value.as_ref()) {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            if let Some(shared) = call.shared_top_level_locals.as_deref_mut() {
                                export_shared_locals(function, stack, shared);
                            }
                            stack.pop();
                            return VmResult::success_with_diagnostics(
                                output.clone(),
                                value,
                                diagnostics,
                            );
                        }
                        Some(PendingControl::Throw(value)) => {
                            if let Some(target) = handle_throw(
                                value.clone(),
                                &mut exception_handlers,
                                stack,
                                &mut pending_control,
                            ) {
                                block_id = target;
                                continue 'dispatch;
                            }
                            return uncaught_exception(output, compiled, stack, value);
                        }
                        None => {
                            block_id = *after;
                            continue 'dispatch;
                        }
                    },
                    InstructionKind::Throw { value } => {
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Some(target) = handle_throw(
                            value.clone(),
                            &mut exception_handlers,
                            stack,
                            &mut pending_control,
                        ) {
                            block_id = target;
                            continue 'dispatch;
                        }
                        return uncaught_exception(output, compiled, stack, value);
                    }
                    InstructionKind::MakeException { dst, message } => {
                        let message = match read_operand(unit, stack, *message) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let object = match make_exception_object(&message) {
                            Ok(object) => object,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Object(object))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::NewArray { dst } => {
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Array(PhpArray::new()))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::NewObject {
                        dst,
                        class_name,
                        args,
                    } => {
                        let class = match compiled.lookup_class(class_name) {
                            Some(class) => class.clone(),
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {class_name} is not defined"
                                    ),
                                );
                            }
                        };
                        let runtime_class = match runtime_class_entry(unit, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let mut values = Vec::with_capacity(args.len());
                        for arg in args {
                            let value = match read_operand(unit, stack, *arg) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            values.push(value);
                        }
                        let object = ObjectRef::new(&runtime_class);
                        if let Some(constructor) = class.constructor {
                            let result = self.execute_function(
                                compiled,
                                constructor,
                                FunctionCall::new(values, Vec::new()).with_this(object.clone()),
                                output,
                                stack,
                                state,
                            );
                            if !result.status.is_success() {
                                return result;
                            }
                            diagnostics.extend(result.diagnostics);
                        } else if !values.is_empty() {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_TOO_MANY_ARGS: constructor for class {class_name} does not accept arguments"
                                ),
                            );
                        }
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, Value::Object(object))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CloneObject { dst, object } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_CLONE_NON_OBJECT: cannot clone {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let class = match compiled.lookup_class(&object.class_name()) {
                            Some(class) => class.clone(),
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                                        object.class_name()
                                    ),
                                );
                            }
                        };
                        let runtime_class = match runtime_class_entry(unit, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Object(object.clone_shallow()))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CloneWith {
                        dst,
                        object,
                        replacements,
                    } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_CLONE_NON_OBJECT: cannot clone {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let replacements = match read_operand(unit, stack, *replacements) {
                            Ok(Value::Array(replacements)) => replacements,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_CLONE_WITH_REPLACEMENTS: clone-with replacements must be array, got {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let class = match compiled.lookup_class(&object.class_name()) {
                            Some(class) => class.clone(),
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                                        object.class_name()
                                    ),
                                );
                            }
                        };
                        let runtime_class = match runtime_class_entry(unit, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let copy = object.clone_shallow();
                        for (key, value) in replacements.iter() {
                            let property = match clone_with_property_name(key) {
                                Ok(property) => property,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            let Some(entry) = runtime_class
                                .properties
                                .iter()
                                .find(|entry| entry.name == property)
                            else {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_PROPERTY: property {}::${property} is not declared",
                                        object.class_name()
                                    ),
                                );
                            };
                            if entry.flags.is_static
                                || entry.flags.is_private
                                || entry.flags.is_protected
                                || entry.flags.is_readonly
                            {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNSUPPORTED_PROPERTY_MODIFIER: property {}::${property} uses modifiers outside the Prompt 28 clone-with MVP",
                                        object.class_name()
                                    ),
                                );
                            }
                            if let Err(message) = check_property_type(
                                object.class_name().as_str(),
                                &property,
                                &entry.type_,
                                value,
                            ) {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            copy.set_property(property, value.clone());
                        }
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Object(copy))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::FetchProperty {
                        dst,
                        object,
                        property,
                    } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_PROPERTY_FETCH_NON_OBJECT: cannot fetch property {property} from {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match object.get_property(property) {
                            Some(value) => value,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_PROPERTY: property {}::${property} is not declared",
                                        object.class_name()
                                    ),
                                );
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::AssignProperty {
                        dst,
                        object,
                        property,
                        value,
                    } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_PROPERTY_ASSIGN_NON_OBJECT: cannot assign property {property} on {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let class = match compiled.lookup_class(&object.class_name()) {
                            Some(class) => class,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                                        object.class_name()
                                    ),
                                );
                            }
                        };
                        let Some(entry) = class
                            .properties
                            .iter()
                            .find(|entry| entry.name == *property)
                        else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_UNKNOWN_PROPERTY: property {}::${property} is not declared",
                                    object.class_name()
                                ),
                            );
                        };
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let property_type = ir_runtime_type(entry.type_.as_ref());
                        if let Err(message) = check_property_type(
                            object.class_name().as_str(),
                            property,
                            &property_type,
                            &value,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        object.set_property(property, value.clone());
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::ArrayInsert { array, key, value } => {
                        let key = match key {
                            Some(key) => match read_operand(unit, stack, *key)
                                .and_then(|value| array_key_from_value(&value))
                            {
                                Ok(key) => Some(key),
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            },
                            None => None,
                        };
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let Some(Value::Array(array_value)) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .get_mut(*array)
                        else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                "E_PHP_VM_ARRAY_INSERT_TARGET: target is not an array register",
                            );
                        };
                        if let Some(key) = key {
                            array_value.insert(key, value);
                        } else {
                            array_value.append(value);
                        }
                    }
                    InstructionKind::FetchDim {
                        dst,
                        array,
                        key,
                        quiet,
                    } => {
                        let array = match read_operand(unit, stack, *array) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let key = match read_operand(unit, stack, *key)
                            .and_then(|value| array_key_from_value(&value))
                        {
                            Ok(key) => key,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match fetch_dim_value(&array, &key) {
                            Ok(Some(value)) => value,
                            Ok(None) if *quiet => Value::Null,
                            Ok(None) => {
                                diagnostics.push(undefined_array_key_warning(
                                    &key,
                                    stack_trace(compiled, stack),
                                ));
                                Value::Null
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::AssignDim {
                        dst,
                        local,
                        dims,
                        value,
                    } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) =
                            assign_dim_local(stack, *local, &dims, value.clone(), false)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::AppendDim {
                        dst,
                        local,
                        dims,
                        value,
                    } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) =
                            assign_dim_local(stack, *local, &dims, value.clone(), true)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::IssetLocal { dst, local } => {
                        let value = read_local_value(stack, *local).unwrap_or(Value::Uninitialized);
                        let result = !matches!(value, Value::Uninitialized | Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Bool(result))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::EmptyLocal { dst, local } => {
                        let value = read_local_value(stack, *local).unwrap_or(Value::Uninitialized);
                        let result = match php_empty(&value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Bool(result))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::UnsetLocal { local } => {
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .set(*local, Value::Uninitialized)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::IssetDim { dst, local, dims } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = read_local_value(stack, *local).and_then(|value| {
                            fetch_dim_path(&value, &dims)
                                .ok()
                                .and_then(|value| value.cloned())
                        });
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(
                                *dst,
                                Value::Bool(!matches!(value, None | Some(Value::Null))),
                            )
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::EmptyDim { dst, local, dims } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = read_local_value(stack, *local)
                            .and_then(|value| {
                                fetch_dim_path(&value, &dims)
                                    .ok()
                                    .and_then(|value| value.cloned())
                            })
                            .unwrap_or(Value::Uninitialized);
                        let result = match php_empty(&value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Bool(result))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::UnsetDim { local, dims } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = unset_dim_local(stack, *local, &dims) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::ForeachInit { iterator, source } => {
                        let source = match read_operand(unit, stack, *source) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let Value::Array(array) = source else {
                            let diagnostic = unsupported_feature(
                                "E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE",
                                format!(
                                    "foreach over {} is not implemented; Phase 4 supports arrays only",
                                    value_type_name(&source)
                                ),
                                RuntimeSourceSpan::default(),
                                stack_trace(compiled, stack),
                            );
                            return VmResult {
                                status: ExecutionStatus::unsupported(
                                    diagnostic.message().to_owned(),
                                ),
                                output: output.clone(),
                                diagnostics: vec![diagnostic],
                                return_value: None,
                                trace: Vec::new(),
                            };
                        };
                        foreach_iterators.insert(
                            *iterator,
                            ForeachSnapshot {
                                entries: array
                                    .iter()
                                    .map(|(key, value)| (key.clone(), value.clone()))
                                    .collect(),
                                position: 0,
                            },
                        );
                    }
                    InstructionKind::ForeachNext {
                        has_value,
                        iterator,
                        key,
                        value,
                    } => {
                        let Some(snapshot) = foreach_iterators.get_mut(iterator) else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_FOREACH_ITERATOR_MISSING: iterator r{} is not initialized",
                                    iterator.raw()
                                ),
                            );
                        };
                        let Some((entry_key, entry_value)) =
                            snapshot.entries.get(snapshot.position).cloned()
                        else {
                            if let Err(message) = stack
                                .current_mut()
                                .expect("frame was pushed")
                                .registers
                                .set(*has_value, Value::Bool(false))
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        };
                        snapshot.position += 1;
                        let frame = stack.current_mut().expect("frame was pushed");
                        if let Err(message) = frame.registers.set(*has_value, Value::Bool(true)) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Some(key) = key
                            && let Err(message) =
                                frame.registers.set(*key, array_key_to_value(entry_key))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) = frame.registers.set(*value, entry_value) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Echo { src } => {
                        let value = match read_operand(unit, stack, *src) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = write_echo(output, &value) {
                            let diagnostic = unsupported_feature(
                                "E_PHP_RUNTIME_UNSUPPORTED_ECHO_VALUE",
                                message,
                                RuntimeSourceSpan::default(),
                                stack_trace(compiled, stack),
                            );
                            return VmResult {
                                status: ExecutionStatus::unsupported(
                                    diagnostic.message().to_owned(),
                                ),
                                output: output.clone(),
                                diagnostics: vec![diagnostic],
                                return_value: None,
                                trace: Vec::new(),
                            };
                        }
                    }
                    InstructionKind::CallFunction { dst, name, args } => {
                        let mut values = Vec::with_capacity(args.len());
                        for arg in args {
                            let value = match read_operand(unit, stack, *arg) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            values.push(value);
                        }
                        let result = if let Some(callee) = compiled.lookup_function(name) {
                            self.execute_function(
                                compiled,
                                callee,
                                FunctionCall::new(values, Vec::new()),
                                output,
                                stack,
                                state,
                            )
                        } else if BuiltinRegistry::new().contains(name) {
                            execute_builtin(name, values, output)
                        } else {
                            let diagnostic = undefined_function(
                                name,
                                RuntimeSourceSpan::default(),
                                stack_trace(compiled, stack),
                            );
                            return VmResult::runtime_error_with_diagnostic(
                                output.clone(),
                                diagnostic.message().to_owned(),
                                diagnostic,
                            );
                        };
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CallMethod {
                        dst,
                        object,
                        method,
                        args,
                    } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_METHOD_CALL_NON_OBJECT: cannot call method {method} on {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let class = match compiled.lookup_class(&object.class_name()) {
                            Some(class) => class,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                                        object.class_name()
                                    ),
                                );
                            }
                        };
                        let method_entry = match lookup_method(class, method) {
                            Some(method) => method,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                                        object.class_name(),
                                        method
                                    ),
                                );
                            }
                        };
                        if method_entry.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_STATIC_METHOD_AS_INSTANCE: method {}::{} is static",
                                    object.class_name(),
                                    method_entry.name
                                ),
                            );
                        }
                        if let Err(message) = validate_method_callable(class, method_entry) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = self.execute_function(
                            compiled,
                            method_entry.function,
                            FunctionCall::new(values, Vec::new()).with_this(object),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CallStaticMethod {
                        dst,
                        class_name,
                        method,
                        args,
                    } => {
                        let class = match compiled.lookup_class(class_name) {
                            Some(class) => class,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {class_name} is not defined"
                                    ),
                                );
                            }
                        };
                        let method_entry = match lookup_method(class, method) {
                            Some(method) => method,
                            None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_METHOD: method {class_name}::{method} is not defined"
                                    ),
                                );
                            }
                        };
                        if !method_entry.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_NON_STATIC_METHOD_CALL: method {}::{} is not static",
                                    class.name, method_entry.name
                                ),
                            );
                        }
                        if let Err(message) = validate_method_callable(class, method_entry) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = self.execute_function(
                            compiled,
                            method_entry.function,
                            FunctionCall::new(values, Vec::new()),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::MakeClosure {
                        dst,
                        function,
                        captures,
                    } => {
                        let captured = match evaluate_closure_captures(unit, stack, captures) {
                            Ok(captures) => captures,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = Value::closure(function.raw(), captured);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CallClosure { dst, callee, args } => {
                        let callee = match read_operand(unit, stack, *callee) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let Some((function, captures)) = callee.as_closure() else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                "E_PHP_VM_CALL_NON_CLOSURE: value is not a closure",
                            );
                        };
                        let mut values = Vec::with_capacity(args.len());
                        for arg in args {
                            let value = match read_operand(unit, stack, *arg) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            values.push(value);
                        }
                        let result = self.execute_function(
                            compiled,
                            FunctionId::new(function),
                            FunctionCall::new(values, captures.clone()),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::ResolveCallable { dst, callable } => {
                        let value = resolve_callable(compiled, callable);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::CallCallable { dst, callee, args } => {
                        let callee = match read_operand(unit, stack, *callee) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let mut values = Vec::with_capacity(args.len());
                        for arg in args {
                            let value = match read_operand(unit, stack, *arg) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            values.push(value);
                        }
                        let result =
                            self.call_callable(compiled, callee, values, output, stack, state);
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Pipe {
                        dst,
                        input,
                        callable,
                    } => {
                        let input = match read_operand(unit, stack, *input) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let callable = match read_operand(unit, stack, *callable) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = self.call_callable(
                            compiled,
                            callable,
                            vec![input],
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Include { dst, kind, path } => {
                        let path = match read_operand(unit, stack, *path) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result =
                            self.execute_include(compiled, *kind, &path, output, stack, state);
                        if !result.status.is_success() {
                            if matches!(kind, IncludeKind::Include | IncludeKind::IncludeOnce) {
                                diagnostics.extend(result.diagnostics);
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("caller frame is active")
                                    .registers
                                    .set(*dst, Value::Bool(false))
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                continue;
                            }
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let return_value = result.return_value.unwrap_or(Value::Int(1));
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .registers
                            .set(*dst, return_value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::ArrayGet { dst, array, index } => {
                        let array = match read_operand(unit, stack, *array) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let index = match read_operand(unit, stack, *index) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match packed_array_get(&array, &index) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::Unsupported { diagnostic_id } => {
                        let diagnostic = unsupported_feature(
                            diagnostic_id.clone(),
                            format!("unsupported IR instruction {diagnostic_id}"),
                            RuntimeSourceSpan::default(),
                            stack_trace(compiled, stack),
                        );
                        return VmResult {
                            status: ExecutionStatus::unsupported(diagnostic.message().to_owned()),
                            output: output.clone(),
                            diagnostics: vec![diagnostic],
                            return_value: None,
                            trace: Vec::new(),
                        };
                    }
                    InstructionKind::RuntimeError {
                        diagnostic_id,
                        message,
                    } => {
                        return self.runtime_error(
                            output,
                            compiled,
                            stack,
                            format!("{diagnostic_id}: {message}"),
                        );
                    }
                }
            }

            let Some(terminator) = &block.terminator else {
                return self.runtime_error(output, compiled, stack, "block has no terminator");
            };
            match &terminator.kind {
                TerminatorKind::Return { value } => {
                    let value = match value {
                        Some(value) => match read_operand(unit, stack, *value) {
                            Ok(value) => Some(value),
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        },
                        None => None,
                    };
                    if let Some(handler) = exception_handlers.pop()
                        && let Some(finally) = handler.finally
                    {
                        pending_control = Some(PendingControl::Return(value));
                        block_id = finally;
                        continue 'dispatch;
                    }
                    if let Err(message) = check_return_type(function, value.as_ref()) {
                        return self.runtime_error(output, compiled, stack, message);
                    }
                    if let Some(shared) = call.shared_top_level_locals.as_deref_mut() {
                        export_shared_locals(function, stack, shared);
                    }
                    stack.pop();
                    return VmResult::success_with_diagnostics(output.clone(), value, diagnostics);
                }
                TerminatorKind::Jump { target } => {
                    block_id = *target;
                }
                TerminatorKind::JumpIfFalse { condition, target } => {
                    let value = match read_operand(unit, stack, *condition) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    let truthy = match to_bool(&value) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    if truthy {
                        block_id = match next_block_id(function, block_id) {
                            Ok(block_id) => block_id,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                    } else {
                        block_id = *target;
                    }
                }
                TerminatorKind::JumpIfTrue { condition, target } => {
                    let value = match read_operand(unit, stack, *condition) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    let truthy = match to_bool(&value) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    if truthy {
                        block_id = *target;
                    } else {
                        block_id = match next_block_id(function, block_id) {
                            Ok(block_id) => block_id,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                    }
                }
                TerminatorKind::JumpIf {
                    condition,
                    if_true,
                    if_false,
                } => {
                    let value = match read_operand(unit, stack, *condition) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    let truthy = match to_bool(&value) {
                        Ok(value) => value,
                        Err(message) => {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    };
                    block_id = if truthy { *if_true } else { *if_false };
                }
            }
        }
    }

    fn call_callable(
        &self,
        compiled: &CompiledUnit,
        callee: Value,
        args: Vec<Value>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        match callee {
            Value::Callable(CallableValue::UserFunction { name }) => {
                let Some(function) = compiled.lookup_function(&name) else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!("E_PHP_VM_UNRESOLVED_CALLABLE: function {name} is not defined"),
                    );
                };
                self.execute_function(
                    compiled,
                    function,
                    FunctionCall::new(args, Vec::new()),
                    output,
                    stack,
                    state,
                )
            }
            Value::Callable(CallableValue::Closure { function, captures }) => {
                self.execute_function(
                    compiled,
                    FunctionId::new(function),
                    FunctionCall::new(args, captures),
                    output,
                    stack,
                    state,
                )
            }
            Value::Callable(CallableValue::InternalBuiltin { name }) => {
                execute_builtin(&name, args, output)
            }
            Value::Callable(CallableValue::MethodPlaceholder { target }) => self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_UNSUPPORTED_METHOD_CALLABLE: method callable {target} is not implemented"
                ),
            ),
            Value::Callable(CallableValue::UnresolvedDynamic { target }) => self.runtime_error(
                output,
                compiled,
                stack,
                format!("E_PHP_VM_UNRESOLVED_CALLABLE: callable {target} could not be resolved"),
            ),
            other => self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_PIPE_RHS_NOT_CALLABLE: {} is not callable",
                    value_type_name(&other)
                ),
            ),
        }
    }

    fn execute_include(
        &self,
        compiled: &CompiledUnit,
        kind: IncludeKind,
        path: &Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let path = match to_string(path) {
            Ok(path) => path.to_string_lossy(),
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        let Some(loader) = &self.options.include_loader else {
            return include_failure(
                output,
                kind,
                "E_PHP_VM_INCLUDE_DISABLED: include/require loader is not configured",
                stack_trace(compiled, stack),
            );
        };
        let including_file = current_source_path(compiled, stack);
        let loaded = match loader.load(including_file.as_deref(), &path) {
            Ok(loaded) => loaded,
            Err(message) => {
                return include_failure(output, kind, message, stack_trace(compiled, stack));
            }
        };
        if matches!(kind, IncludeKind::IncludeOnce | IncludeKind::RequireOnce) {
            if state.included_once.contains(&loaded.canonical_path) {
                return VmResult::success(output.clone(), Some(Value::Bool(true)));
            }
            state.included_once.push(loaded.canonical_path.clone());
        }

        let frontend = php_semantics::analyze_source(&loaded.source);
        if frontend.has_errors() {
            return include_failure(
                output,
                kind,
                format!(
                    "E_PHP_VM_INCLUDE_COMPILE_ERROR: {} failed frontend analysis",
                    loaded.canonical_path.display()
                ),
                stack_trace(compiled, stack),
            );
        }
        let lowering = php_ir::lower_frontend_result(
            &frontend,
            php_ir::LoweringOptions {
                source_path: loaded.canonical_path.to_string_lossy().into_owned(),
                source_text: Some(loaded.source.clone()),
                ..php_ir::LoweringOptions::default()
            },
        );
        if !lowering.diagnostics.is_empty() || lowering.verification.is_err() {
            return include_failure(
                output,
                kind,
                format!(
                    "E_PHP_VM_INCLUDE_COMPILE_ERROR: {} failed IR lowering",
                    loaded.canonical_path.display()
                ),
                stack_trace(compiled, stack),
            );
        }
        let included = CompiledUnit::new(lowering.unit);
        let mut shared = shared_locals_from_current_frame(compiled, stack);
        let call = FunctionCall {
            args: Vec::new(),
            captures: Vec::new(),
            this_value: None,
            shared_top_level_locals: Some(&mut shared),
        };
        let result =
            self.execute_function(&included, included.unit().entry, call, output, stack, state);
        if result.status.is_success() {
            write_shared_locals_to_current_frame(compiled, stack, &shared);
        }
        result
    }

    fn runtime_error(
        &self,
        output: &OutputBuffer,
        compiled: &CompiledUnit,
        stack: &CallStack,
        message: impl Into<String>,
    ) -> VmResult {
        let mut message = message.into();
        let diagnostic_message = message.clone();
        if stack.len() > 1 {
            message.push_str("\ncall_stack:");
            for frame in stack.frames().iter().rev() {
                let name = compiled
                    .unit()
                    .functions
                    .get(frame.function.index())
                    .map(|function| function.name.as_str())
                    .unwrap_or("<missing>");
                message.push_str("\n  at ");
                message.push_str(name);
            }
        }
        let diagnostic = runtime_diagnostic_for_message(&diagnostic_message, compiled, stack);
        VmResult::runtime_error_with_diagnostic(output.clone(), message, diagnostic)
    }
}

fn format_instruction_kind(kind: &InstructionKind) -> String {
    format!("{kind:?}")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_locals(function: &IrFunction, stack: &CallStack) -> String {
    let Some(frame) = stack.current() else {
        return String::new();
    };
    frame
        .locals
        .iter()
        .filter_map(|(index, slot)| {
            let value = slot.read();
            if value.is_uninitialized() {
                return None;
            }
            let name = function
                .locals
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("local:{index}"));
            Some(format!("{name}={}", trace_value(&value)))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_registers(stack: &CallStack) -> String {
    let Some(frame) = stack.current() else {
        return String::new();
    };
    frame
        .registers
        .iter()
        .filter_map(|(index, value)| {
            if value.is_uninitialized() {
                None
            } else {
                Some(format!("r{index}={}", trace_value(value)))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn trace_value(value: &Value) -> String {
    format!("{value:?}")
}

fn runtime_diagnostic_for_message(
    message: &str,
    compiled: &CompiledUnit,
    stack: &CallStack,
) -> RuntimeDiagnostic {
    if message == "division by zero" {
        return division_by_zero_mvp(RuntimeSourceSpan::default(), stack_trace(compiled, stack));
    }
    let id = message
        .split_once(':')
        .and_then(|(id, _)| id.starts_with("E_").then_some(id))
        .unwrap_or("E_PHP_RUNTIME_ERROR");
    RuntimeDiagnostic::new(
        id,
        RuntimeSeverity::FatalError,
        message.to_owned(),
        RuntimeSourceSpan::default(),
        stack_trace(compiled, stack),
        None,
    )
}

fn make_exception_object(message: &Value) -> Result<ObjectRef, String> {
    let message = to_string(message)?.to_string_lossy();
    let class = RuntimeClassEntry {
        name: "Exception".to_owned(),
        methods: Vec::new(),
        properties: vec![RuntimeClassPropertyEntry {
            name: "message".to_owned(),
            default: Value::String(PhpString::from_test_str(&message)),
            type_: Some(RuntimeType::String),
            flags: RuntimeClassPropertyFlags::default(),
        }],
        constructor_id: None,
        flags: RuntimeClassFlags::default(),
    };
    Ok(ObjectRef::new(&class))
}

fn handle_throw(
    value: Value,
    handlers: &mut Vec<ExceptionHandler>,
    stack: &mut CallStack,
    pending_control: &mut Option<PendingControl>,
) -> Option<BlockId> {
    let handler = handlers.pop()?;
    if let Some(catch) = handler.catch {
        if let Some(local) = handler.exception_local
            && let Some(frame) = stack.current_mut()
        {
            let _ = frame.locals.set(local, value);
        }
        return Some(catch);
    }
    if let Some(finally) = handler.finally {
        *pending_control = Some(PendingControl::Throw(value));
        return Some(finally);
    }
    None
}

fn uncaught_exception(
    output: &OutputBuffer,
    compiled: &CompiledUnit,
    stack: &CallStack,
    value: Value,
) -> VmResult {
    let message = match &value {
        Value::Object(object) if object.class_name().eq_ignore_ascii_case("Exception") => object
            .get_property("message")
            .and_then(|value| to_string(&value).ok())
            .map(|value| value.to_string_lossy())
            .unwrap_or_default(),
        Value::Object(object) => format!("uncaught object {}", object.class_name()),
        other => format!("uncaught {}", value_type_name(other)),
    };
    let full = if message.is_empty() {
        "E_PHP_VM_UNCAUGHT_EXCEPTION: Uncaught Exception".to_owned()
    } else {
        format!("E_PHP_VM_UNCAUGHT_EXCEPTION: Uncaught Exception: {message}")
    };
    VmResult::runtime_error_with_diagnostic(
        output.clone(),
        full.clone(),
        RuntimeDiagnostic::new(
            "E_PHP_VM_UNCAUGHT_EXCEPTION",
            RuntimeSeverity::FatalError,
            full,
            RuntimeSourceSpan::default(),
            stack_trace(compiled, stack),
            None,
        ),
    )
}

fn runtime_class_entry(
    unit: &IrUnit,
    class: &php_ir::module::ClassEntry,
) -> Result<RuntimeClassEntry, String> {
    let mut properties = Vec::with_capacity(class.properties.len());
    for property in &class.properties {
        let default = if let Some(default) = property.default {
            constant_value(unit, default)?
        } else {
            Value::Null
        };
        properties.push(RuntimeClassPropertyEntry {
            name: property.name.clone(),
            default,
            type_: ir_runtime_type(property.type_.as_ref()),
            flags: RuntimeClassPropertyFlags {
                is_static: property.flags.is_static,
                is_private: property.flags.is_private,
                is_protected: property.flags.is_protected,
                is_readonly: property.flags.is_readonly,
                is_typed: property.flags.is_typed,
            },
        });
    }
    Ok(RuntimeClassEntry {
        name: class.name.clone(),
        methods: class
            .methods
            .iter()
            .map(|method| RuntimeClassMethodEntry {
                name: method.name.clone(),
                function_id: method.function.raw(),
                flags: RuntimeClassMethodFlags {
                    is_static: method.flags.is_static,
                    is_private: method.flags.is_private,
                    is_protected: method.flags.is_protected,
                    is_abstract: method.flags.is_abstract,
                },
            })
            .collect(),
        properties,
        constructor_id: class.constructor.map(|function| function.raw()),
        flags: RuntimeClassFlags {
            is_abstract: class.flags.is_abstract,
            is_final: class.flags.is_final,
            is_readonly: class.flags.is_readonly,
        },
    })
}

fn validate_object_mvp(class: &RuntimeClassEntry) -> Result<(), String> {
    if class.flags.is_abstract {
        return Err(format!(
            "E_PHP_VM_UNSUPPORTED_CLASS_MODIFIER: class {} is abstract",
            class.name
        ));
    }
    if class.flags.is_readonly {
        return Err(format!(
            "E_PHP_VM_UNSUPPORTED_CLASS_MODIFIER: class {} is readonly",
            class.name
        ));
    }
    for method in &class.methods {
        if method.flags.is_private || method.flags.is_protected || method.flags.is_abstract {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_METHOD_MODIFIER: method {}::{} uses visibility or abstract modifiers outside the Prompt 27 method MVP",
                class.name, method.name
            ));
        }
    }
    for property in &class.properties {
        if property.flags.is_static
            || property.flags.is_private
            || property.flags.is_protected
            || property.flags.is_readonly
        {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_PROPERTY_MODIFIER: property {}::${} uses modifiers outside the Prompt 26 object MVP",
                class.name, property.name
            ));
        }
    }
    Ok(())
}

fn validate_method_callable(
    class: &php_ir::module::ClassEntry,
    method: &php_ir::module::ClassMethodEntry,
) -> Result<(), String> {
    if method.flags.is_private || method.flags.is_protected || method.flags.is_abstract {
        return Err(format!(
            "E_PHP_VM_UNSUPPORTED_METHOD_VISIBILITY: method {}::{} uses visibility or abstract modifiers outside the Prompt 27 method MVP",
            class.name, method.name
        ));
    }
    Ok(())
}

fn lookup_method<'a>(
    class: &'a php_ir::module::ClassEntry,
    method: &str,
) -> Option<&'a php_ir::module::ClassMethodEntry> {
    let normalized = normalize_method_name(method);
    class
        .methods
        .iter()
        .find(|entry| normalize_method_name(&entry.name) == normalized)
}

fn normalize_method_name(method: &str) -> String {
    method.to_ascii_lowercase()
}

fn include_failure(
    output: &OutputBuffer,
    kind: IncludeKind,
    message: impl Into<String>,
    stack_trace: Vec<RuntimeStackFrame>,
) -> VmResult {
    let message = message.into();
    let severity = if matches!(kind, IncludeKind::Include | IncludeKind::IncludeOnce) {
        RuntimeSeverity::Warning
    } else {
        RuntimeSeverity::FatalError
    };
    VmResult::runtime_error_with_diagnostic(
        output.clone(),
        message.clone(),
        RuntimeDiagnostic::new(
            include_failure_id(&message).to_owned(),
            severity,
            message,
            RuntimeSourceSpan::default(),
            stack_trace,
            None,
        ),
    )
}

fn include_failure_id(message: &str) -> &str {
    message
        .split_once(':')
        .and_then(|(id, _)| id.starts_with("E_").then_some(id))
        .unwrap_or("E_PHP_VM_INCLUDE_ERROR")
}

fn current_source_path(compiled: &CompiledUnit, stack: &CallStack) -> Option<PathBuf> {
    let frame = stack.current()?;
    let function = compiled.unit().functions.get(frame.function.index())?;
    let file = compiled.unit().files.get(function.span.file.index())?;
    Some(PathBuf::from(&file.path))
}

fn shared_locals_from_current_frame(
    compiled: &CompiledUnit,
    stack: &CallStack,
) -> HashMap<String, Value> {
    let Some(frame) = stack.current() else {
        return HashMap::new();
    };
    let Some(function) = compiled.unit().functions.get(frame.function.index()) else {
        return HashMap::new();
    };
    function
        .locals
        .iter()
        .enumerate()
        .filter_map(|(index, name)| {
            frame
                .locals
                .get(LocalId::new(index as u32))
                .map(|value| (name.clone(), value))
        })
        .collect()
}

fn import_shared_locals(
    function: &IrFunction,
    stack: &mut CallStack,
    shared: &HashMap<String, Value>,
) {
    let Some(frame) = stack.current_mut() else {
        return;
    };
    for (index, name) in function.locals.iter().enumerate() {
        if let Some(value) = shared.get(name) {
            let _ = frame.locals.set(LocalId::new(index as u32), value.clone());
        }
    }
}

fn initialize_runtime_context_locals(
    function: &IrFunction,
    stack: &mut CallStack,
    context: &RuntimeContext,
) {
    let Some(frame) = stack.current_mut() else {
        return;
    };
    for (index, name) in function.locals.iter().enumerate() {
        if let Some(value) = context.global_value(name) {
            let _ = frame.locals.set(LocalId::new(index as u32), value);
        }
    }
}

fn export_shared_locals(
    function: &IrFunction,
    stack: &CallStack,
    shared: &mut HashMap<String, Value>,
) {
    let Some(frame) = stack.current() else {
        return;
    };
    for (index, name) in function.locals.iter().enumerate() {
        if let Some(value) = frame.locals.get(LocalId::new(index as u32)) {
            shared.insert(name.clone(), value);
        }
    }
}

fn write_shared_locals_to_current_frame(
    compiled: &CompiledUnit,
    stack: &mut CallStack,
    shared: &HashMap<String, Value>,
) {
    let Some(frame) = stack.current_mut() else {
        return;
    };
    let Some(function) = compiled.unit().functions.get(frame.function.index()) else {
        return;
    };
    for (index, name) in function.locals.iter().enumerate() {
        if let Some(value) = shared.get(name) {
            let _ = frame.locals.set(LocalId::new(index as u32), value.clone());
        }
    }
}

fn stack_trace(compiled: &CompiledUnit, stack: &CallStack) -> Vec<RuntimeStackFrame> {
    stack
        .frames()
        .iter()
        .rev()
        .map(|frame| {
            let name = compiled
                .unit()
                .functions
                .get(frame.function.index())
                .map(|function| function.name.as_str())
                .unwrap_or("<missing>");
            RuntimeStackFrame::new(name)
        })
        .collect()
}

fn resolve_callable(compiled: &CompiledUnit, callable: &CallableKind) -> Value {
    match callable {
        CallableKind::FunctionName { name } => {
            if compiled.lookup_function(name).is_some() {
                Value::user_function_callable(name.clone())
            } else if is_supported_builtin(name) {
                Value::internal_builtin_callable(name.clone())
            } else {
                Value::unresolved_callable(format!("function {name}"))
            }
        }
        CallableKind::MethodPlaceholder { target } => {
            Value::method_callable_placeholder(target.clone())
        }
        CallableKind::UnresolvedDynamic { target } => Value::unresolved_callable(target.clone()),
    }
}

fn is_supported_builtin(name: &str) -> bool {
    BuiltinRegistry::new().contains(name)
}

fn execute_builtin(name: &str, args: Vec<Value>, output: &mut OutputBuffer) -> VmResult {
    let Some(entry) = BuiltinRegistry::new().get(name) else {
        let message = format!("E_PHP_VM_UNKNOWN_BUILTIN: builtin {name} is not implemented");
        return VmResult::runtime_error_with_diagnostic(
            output.clone(),
            message.clone(),
            RuntimeDiagnostic::new(
                "E_PHP_VM_UNKNOWN_BUILTIN",
                RuntimeSeverity::FatalError,
                message,
                RuntimeSourceSpan::default(),
                Vec::new(),
                None,
            ),
        );
    };
    let mut context = BuiltinContext::new(output);
    match (entry.function())(&mut context, args, RuntimeSourceSpan::default()) {
        Ok(value) => VmResult::success(context.output().clone(), Some(value)),
        Err(error) => VmResult::runtime_error_with_diagnostic(
            context.output().clone(),
            error.display_message(),
            RuntimeDiagnostic::new(
                error.diagnostic_id(),
                RuntimeSeverity::FatalError,
                error.message().to_owned(),
                RuntimeSourceSpan::default(),
                Vec::new(),
                None,
            ),
        ),
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

fn constant_value(unit: &IrUnit, constant: ConstId) -> Result<Value, String> {
    let Some(value) = unit.constants.get(constant.index()) else {
        return Err(format!("invalid constant const:{}", constant.raw()));
    };
    Ok(match value {
        IrConstant::Null => Value::Null,
        IrConstant::Bool(value) => Value::Bool(*value),
        IrConstant::Int(value) => Value::Int(*value),
        IrConstant::Float(value) => Value::float(*value),
        IrConstant::String(value) => Value::String(PhpString::from_test_str(value)),
    })
}

fn inline_constant_value(constant: &IrConstant) -> Value {
    match constant {
        IrConstant::Null => Value::Null,
        IrConstant::Bool(value) => Value::Bool(*value),
        IrConstant::Int(value) => Value::Int(*value),
        IrConstant::Float(value) => Value::float(*value),
        IrConstant::String(value) => Value::String(PhpString::from_test_str(value)),
    }
}

fn prepare_arguments(function: &IrFunction, args: Vec<Value>) -> Result<Vec<Value>, String> {
    let min = function
        .params
        .iter()
        .filter(|param| param.required)
        .count();
    let variadic_index = function.params.iter().position(|param| param.variadic);
    let max = variadic_index.unwrap_or(function.params.len());
    if args.len() < min {
        return Err(format!(
            "E_PHP_VM_TOO_FEW_ARGS: function {} expects at least {} argument(s), got {}",
            function.name,
            min,
            args.len()
        ));
    }
    if variadic_index.is_none() && args.len() > max {
        return Err(format!(
            "E_PHP_VM_TOO_MANY_ARGS: function {} expects at most {} argument(s), got {}",
            function.name,
            max,
            args.len()
        ));
    }

    let mut prepared = Vec::with_capacity(function.params.len());
    for (index, param) in function.params.iter().enumerate() {
        if param.variadic {
            let tail = if index <= args.len() {
                args[index..].to_vec()
            } else {
                Vec::new()
            };
            prepared.push(Value::packed_array(tail));
            break;
        }
        if let Some(value) = args.get(index) {
            prepared.push(value.clone());
        } else if let Some(default) = &param.default {
            prepared.push(inline_constant_value(default));
        } else if param.required {
            return Err(format!(
                "E_PHP_VM_TOO_FEW_ARGS: function {} is missing argument ${}",
                function.name, param.name
            ));
        } else {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_DEFAULT_ARG: function {} parameter ${} has no folded default",
                function.name, param.name
            ));
        }
    }
    Ok(prepared)
}

fn check_param_type(function: &IrFunction, param: &IrParam, value: &Value) -> Result<(), String> {
    if param.variadic {
        return Ok(());
    }
    let Some(runtime_type) = ir_runtime_type(param.type_.as_ref()) else {
        return Ok(());
    };
    if value_matches_runtime_type(value, &runtime_type) {
        Ok(())
    } else {
        Err(format!(
            "E_PHP_VM_PARAM_TYPE_MISMATCH: function {} argument ${} got {}, expected {}",
            function.name,
            param.name,
            value_type_name(value),
            runtime_type_name(&runtime_type)
        ))
    }
}

fn evaluate_closure_captures(
    unit: &IrUnit,
    stack: &CallStack,
    captures: &[ClosureCaptureArg],
) -> Result<Vec<ClosureCaptureValue>, String> {
    let mut values = Vec::with_capacity(captures.len());
    for capture in captures {
        if capture.by_ref {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_BY_REF_CAPTURE: closure captures ${} by reference",
                capture.name
            ));
        }
        values.push(ClosureCaptureValue {
            name: capture.name.clone(),
            value: read_operand(unit, stack, capture.src)?,
        });
    }
    Ok(values)
}

fn initialize_captures(
    function: &IrFunction,
    captures: Vec<ClosureCaptureValue>,
    stack: &mut CallStack,
) -> Result<(), String> {
    if function.captures.is_empty() {
        return Ok(());
    }
    for metadata in &function.captures {
        if metadata.by_ref {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_BY_REF_CAPTURE: closure captures ${} by reference",
                metadata.name
            ));
        }
        let value = captures
            .iter()
            .find(|capture| capture.name == metadata.name)
            .map(|capture| capture.value.clone())
            .unwrap_or(Value::Null);
        stack
            .current_mut()
            .expect("frame was pushed")
            .locals
            .set(metadata.local, value)?;
    }
    Ok(())
}

fn initialize_this(
    function: &IrFunction,
    this_value: ObjectRef,
    stack: &mut CallStack,
) -> Result<(), String> {
    let Some(index) = function.locals.iter().position(|name| name == "this") else {
        return Err(format!(
            "E_PHP_VM_MISSING_THIS_LOCAL: method {} has no $this local",
            function.name
        ));
    };
    stack
        .current_mut()
        .expect("frame was pushed")
        .locals
        .set(LocalId::new(index as u32), Value::Object(this_value))
}

fn check_return_type(function: &IrFunction, value: Option<&Value>) -> Result<(), String> {
    let Some(return_type) = ir_runtime_type(function.return_type.as_ref()) else {
        return Ok(());
    };
    if matches!(return_type, RuntimeType::Void) {
        return match value {
            None => Ok(()),
            Some(value) => Err(format!(
                "E_PHP_VM_RETURN_TYPE_MISMATCH: function {} returned {}, expected void",
                function.name,
                value_type_name(value)
            )),
        };
    };
    let value = value.unwrap_or(&Value::Null);
    if value_matches_runtime_type(value, &return_type) {
        Ok(())
    } else {
        Err(format!(
            "E_PHP_VM_RETURN_TYPE_MISMATCH: function {} returned {}, expected {}",
            function.name,
            value_type_name(value),
            runtime_type_name(&return_type)
        ))
    }
}

fn check_property_type(
    class_name: &str,
    property: &str,
    runtime_type: &Option<RuntimeType>,
    value: &Value,
) -> Result<(), String> {
    let Some(runtime_type) = runtime_type else {
        return Ok(());
    };
    if value_matches_runtime_type(value, runtime_type) {
        Ok(())
    } else {
        Err(format!(
            "E_PHP_VM_PROPERTY_TYPE_MISMATCH: property {class_name}::${property} got {}, expected {}",
            value_type_name(value),
            runtime_type_name(runtime_type)
        ))
    }
}

fn ir_runtime_type(return_type: Option<&IrReturnType>) -> Option<RuntimeType> {
    Some(match return_type? {
        IrReturnType::Int => RuntimeType::Int,
        IrReturnType::Float => RuntimeType::Float,
        IrReturnType::String => RuntimeType::String,
        IrReturnType::Array => RuntimeType::Array,
        IrReturnType::Callable => RuntimeType::Callable,
        IrReturnType::Object => RuntimeType::Object,
        IrReturnType::Bool => RuntimeType::Bool,
        IrReturnType::Null => RuntimeType::Null,
        IrReturnType::Void => RuntimeType::Void,
        IrReturnType::Mixed => RuntimeType::Mixed,
        IrReturnType::Class { name } => RuntimeType::Class { name: name.clone() },
        IrReturnType::Nullable { inner } => RuntimeType::Nullable {
            inner: Box::new(ir_runtime_type(Some(inner))?),
        },
    })
}

fn value_matches_runtime_type(value: &Value, runtime_type: &RuntimeType) -> bool {
    match runtime_type {
        RuntimeType::Mixed => true,
        RuntimeType::Null => matches!(value, Value::Null),
        RuntimeType::Void => false,
        RuntimeType::Bool => matches!(value, Value::Bool(_)),
        RuntimeType::Int => matches!(value, Value::Int(_)),
        RuntimeType::Float => matches!(value, Value::Float(_) | Value::Int(_)),
        RuntimeType::String => matches!(value, Value::String(_)),
        RuntimeType::Array => matches!(value, Value::Array(_)),
        RuntimeType::Callable => matches!(value, Value::Callable(_)),
        RuntimeType::Object => matches!(value, Value::Object(_)),
        RuntimeType::Class { name } => matches!(
            value,
            Value::Object(object) if object.class_name().eq_ignore_ascii_case(name)
        ),
        RuntimeType::Nullable { inner } => {
            matches!(value, Value::Null) || value_matches_runtime_type(value, inner)
        }
    }
}

fn runtime_type_name(runtime_type: &RuntimeType) -> String {
    match runtime_type {
        RuntimeType::Int => "int".to_owned(),
        RuntimeType::Float => "float".to_owned(),
        RuntimeType::String => "string".to_owned(),
        RuntimeType::Array => "array".to_owned(),
        RuntimeType::Callable => "callable".to_owned(),
        RuntimeType::Object => "object".to_owned(),
        RuntimeType::Bool => "bool".to_owned(),
        RuntimeType::Null => "null".to_owned(),
        RuntimeType::Void => "void".to_owned(),
        RuntimeType::Mixed => "mixed".to_owned(),
        RuntimeType::Class { name } => name.clone(),
        RuntimeType::Nullable { inner } => format!("?{}", runtime_type_name(inner)),
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Uninitialized => "uninitialized",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Callable(_) => "callable",
        Value::Reference(_) => "reference",
    }
}

fn array_key_from_value(value: &Value) -> Result<ArrayKey, String> {
    ArrayKey::from_value_mvp(value).ok_or_else(|| {
        format!(
            "E_PHP_VM_ARRAY_KEY_CONVERSION: cannot use {} as array key",
            value_type_name(value)
        )
    })
}

fn array_key_to_value(key: ArrayKey) -> Value {
    match key {
        ArrayKey::Int(value) => Value::Int(value),
        ArrayKey::String(value) => Value::String(value),
    }
}

fn clone_with_property_name(key: &ArrayKey) -> Result<String, String> {
    let ArrayKey::String(value) = key else {
        return Err(
            "E_PHP_VM_CLONE_WITH_PROPERTY_KEY: clone-with property names must be strings"
                .to_owned(),
        );
    };
    String::from_utf8(value.as_bytes().to_vec()).map_err(|_| {
        "E_PHP_VM_CLONE_WITH_PROPERTY_KEY: clone-with property name is not valid UTF-8".to_owned()
    })
}

fn fetch_dim_value(array: &Value, key: &ArrayKey) -> Result<Option<Value>, String> {
    let Value::Array(array) = array else {
        return Err("E_PHP_VM_ARRAY_FETCH_TYPE: value is not an array".to_owned());
    };
    Ok(array.get(key).cloned())
}

fn fetch_dim_path<'a>(value: &'a Value, dims: &[ArrayKey]) -> Result<Option<&'a Value>, String> {
    let mut current = value;
    for key in dims {
        let Value::Array(array) = current else {
            return Ok(None);
        };
        let Some(next) = array.get(key) else {
            return Ok(None);
        };
        current = next;
    }
    Ok(Some(current))
}

fn read_dim_operands(
    unit: &IrUnit,
    stack: &CallStack,
    dims: &[Operand],
) -> Result<Vec<ArrayKey>, String> {
    dims.iter()
        .map(|operand| {
            read_operand(unit, stack, *operand).and_then(|value| array_key_from_value(&value))
        })
        .collect()
}

fn read_local_value(stack: &CallStack, local: LocalId) -> Option<Value> {
    stack.current()?.locals.get(local)
}

fn is_this_local(function: &IrFunction, local: LocalId) -> bool {
    function
        .locals
        .get(local.index())
        .is_some_and(|name| name == "this")
}

fn read_call_args(
    unit: &IrUnit,
    stack: &CallStack,
    args: &[Operand],
) -> Result<Vec<Value>, String> {
    args.iter()
        .map(|arg| read_operand(unit, stack, *arg))
        .collect()
}

fn assign_dim_local(
    stack: &mut CallStack,
    local: LocalId,
    dims: &[ArrayKey],
    value: Value,
    append: bool,
) -> Result<(), String> {
    let frame = stack.current_mut().ok_or("no active frame")?;
    let Some(slot) = frame.locals.get_slot_mut(local) else {
        return Err(format!("invalid local local:{}", local.raw()));
    };
    let mut current = slot.read();
    if matches!(current, Value::Uninitialized | Value::Null) {
        current = Value::Array(PhpArray::new());
    }
    assign_dim_value(&mut current, dims, value, append)?;
    slot.write(current);
    Ok(())
}

fn assign_dim_value(
    container: &mut Value,
    dims: &[ArrayKey],
    value: Value,
    append: bool,
) -> Result<(), String> {
    let Value::Array(array) = container else {
        return Err(format!(
            "E_PHP_VM_ARRAY_ASSIGN_TYPE: cannot assign dimension on {}",
            value_type_name(container)
        ));
    };
    let Some((first, rest)) = dims.split_first() else {
        if append {
            array.append(value);
            return Ok(());
        }
        return Err("E_PHP_VM_ARRAY_ASSIGN_DIM: missing array dimension".to_owned());
    };
    if rest.is_empty() && !append {
        array.insert(first.clone(), value);
        return Ok(());
    }
    if array.get(first).is_none() {
        array.insert(first.clone(), Value::Array(PhpArray::new()));
    }
    let Some(child) = array.get_mut(first) else {
        return Err("E_PHP_VM_ARRAY_ASSIGN_DIM: failed to create nested array".to_owned());
    };
    if matches!(child, Value::Uninitialized | Value::Null) {
        *child = Value::Array(PhpArray::new());
    }
    assign_dim_value(child, rest, value, append)
}

fn unset_dim_local(stack: &mut CallStack, local: LocalId, dims: &[ArrayKey]) -> Result<(), String> {
    let frame = stack.current_mut().ok_or("no active frame")?;
    let Some(slot) = frame.locals.get_slot_mut(local) else {
        return Err(format!("invalid local local:{}", local.raw()));
    };
    let mut current = slot.read();
    unset_dim_value(&mut current, dims);
    slot.write(current);
    Ok(())
}

fn unset_dim_value(container: &mut Value, dims: &[ArrayKey]) {
    let Some((first, rest)) = dims.split_first() else {
        return;
    };
    let Value::Array(array) = container else {
        return;
    };
    if rest.is_empty() {
        array.remove(first);
        return;
    }
    if let Some(child) = array.get_mut(first) {
        unset_dim_value(child, rest);
    }
}

fn php_empty(value: &Value) -> Result<bool, String> {
    match value {
        Value::Uninitialized | Value::Null => Ok(true),
        Value::Bool(value) => Ok(!*value),
        Value::Int(value) => Ok(*value == 0),
        Value::Float(value) => {
            let value = value.to_f64();
            Ok(value == 0.0 || value.is_nan())
        }
        Value::String(value) => Ok(value.is_empty() || value.as_bytes() == b"0"),
        Value::Array(array) => Ok(array.is_empty()),
        Value::Object(_) | Value::Callable(_) | Value::Reference(_) => Ok(false),
    }
}

fn undefined_array_key_warning(
    key: &ArrayKey,
    stack_trace: Vec<RuntimeStackFrame>,
) -> RuntimeDiagnostic {
    let key = match key {
        ArrayKey::Int(value) => value.to_string(),
        ArrayKey::String(value) => value.to_string_lossy(),
    };
    RuntimeDiagnostic::new(
        "E_PHP_RUNTIME_UNDEFINED_ARRAY_KEY_WARNING",
        RuntimeSeverity::Warning,
        format!("undefined array key {key}"),
        RuntimeSourceSpan::default(),
        stack_trace,
        Some(php_runtime::PhpReferenceClassification::Warning),
    )
}

fn packed_array_get(array: &Value, index: &Value) -> Result<Value, String> {
    let Some(elements) = array.packed_elements() else {
        return Err("E_PHP_VM_ARRAY_FETCH_TYPE: value is not an array".to_owned());
    };
    let NumericValue::Int(index) = to_number(index)? else {
        return Err("E_PHP_VM_ARRAY_FETCH_INDEX: array index must be int".to_owned());
    };
    if index < 0 {
        return Ok(Value::Null);
    }
    Ok(elements
        .get(index as usize)
        .map(|value| (*value).clone())
        .unwrap_or(Value::Null))
}

fn read_operand(unit: &IrUnit, stack: &CallStack, operand: Operand) -> Result<Value, String> {
    match operand {
        Operand::Register(id) => {
            let frame = stack.current().ok_or("no active frame")?;
            let Some(value) = frame.registers.get(id) else {
                return Err(format!("invalid register r{}", id.raw()));
            };
            if value.is_uninitialized() {
                return Err(format!("read uninitialized register r{}", id.raw()));
            }
            Ok(value.clone())
        }
        Operand::Constant(id) => constant_value(unit, id),
        Operand::Local(id) => {
            let frame = stack.current().ok_or("no active frame")?;
            let Some(value) = frame.locals.get(id) else {
                return Err(format!("invalid local local:{}", id.raw()));
            };
            Ok(if value.is_uninitialized() {
                Value::Null
            } else {
                value
            })
        }
    }
}

fn next_block_id(function: &IrFunction, current: BlockId) -> Result<BlockId, String> {
    let next = current.raw() + 1;
    if next as usize >= function.blocks.len() {
        return Err(format!(
            "fallthrough block after block:{} is missing",
            current.raw()
        ));
    }
    Ok(BlockId::new(next))
}

fn write_echo(output: &mut OutputBuffer, value: &Value) -> Result<(), String> {
    let string = to_string(value)?;
    output.write_php_string(&string);
    Ok(())
}

fn execute_binary(op: BinaryOp, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    match op {
        BinaryOp::Concat => {
            let mut bytes = to_string(lhs)?.into_bytes();
            bytes.extend_from_slice(to_string(rhs)?.as_bytes());
            Ok(Value::String(PhpString::from_bytes(bytes)))
        }
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
            let lhs = to_number(lhs)?;
            let rhs = to_number(rhs)?;
            execute_arithmetic(op, lhs, rhs)
        }
        BinaryOp::Pow => {
            let lhs = to_number(lhs)?;
            let rhs = to_number(rhs)?;
            Ok(Value::float(lhs.as_f64().powf(rhs.as_f64())))
        }
    }
}

fn execute_arithmetic(op: BinaryOp, lhs: NumericValue, rhs: NumericValue) -> Result<Value, String> {
    match op {
        BinaryOp::Add if !lhs.is_float() && !rhs.is_float() => match (lhs, rhs) {
            (NumericValue::Int(lhs), NumericValue::Int(rhs)) => lhs
                .checked_add(rhs)
                .map(Value::Int)
                .ok_or_else(|| "integer addition overflow".to_owned()),
            _ => unreachable!("guarded by integer check"),
        },
        BinaryOp::Sub if !lhs.is_float() && !rhs.is_float() => match (lhs, rhs) {
            (NumericValue::Int(lhs), NumericValue::Int(rhs)) => lhs
                .checked_sub(rhs)
                .map(Value::Int)
                .ok_or_else(|| "integer subtraction overflow".to_owned()),
            _ => unreachable!("guarded by integer check"),
        },
        BinaryOp::Mul if !lhs.is_float() && !rhs.is_float() => match (lhs, rhs) {
            (NumericValue::Int(lhs), NumericValue::Int(rhs)) => lhs
                .checked_mul(rhs)
                .map(Value::Int)
                .ok_or_else(|| "integer multiplication overflow".to_owned()),
            _ => unreachable!("guarded by integer check"),
        },
        BinaryOp::Div => {
            if rhs.as_f64() == 0.0 {
                return Err("division by zero".to_owned());
            }
            Ok(Value::float(lhs.as_f64() / rhs.as_f64()))
        }
        BinaryOp::Mod => match (lhs, rhs) {
            (NumericValue::Int(_), NumericValue::Int(0)) => Err("modulo by zero".to_owned()),
            (NumericValue::Int(lhs), NumericValue::Int(rhs)) => Ok(Value::Int(lhs % rhs)),
            _ => Err("modulo is only implemented for integer operands".to_owned()),
        },
        BinaryOp::Add => Ok(Value::float(lhs.as_f64() + rhs.as_f64())),
        BinaryOp::Sub => Ok(Value::float(lhs.as_f64() - rhs.as_f64())),
        BinaryOp::Mul => Ok(Value::float(lhs.as_f64() * rhs.as_f64())),
        BinaryOp::Concat | BinaryOp::Pow => unreachable!("handled outside arithmetic"),
    }
}

fn execute_unary(op: UnaryOp, src: &Value) -> Result<Value, String> {
    match op {
        UnaryOp::Plus => match to_number(src)? {
            NumericValue::Int(value) => Ok(Value::Int(value)),
            NumericValue::Float(value) => Ok(Value::float(value)),
        },
        UnaryOp::Minus => match to_number(src)? {
            NumericValue::Int(value) => value
                .checked_neg()
                .map(Value::Int)
                .ok_or_else(|| "integer negation overflow".to_owned()),
            NumericValue::Float(value) => Ok(Value::float(-value)),
        },
        UnaryOp::Not => Ok(Value::Bool(!to_bool(src)?)),
        UnaryOp::BitNot => match src {
            Value::Int(value) => Ok(Value::Int(!value)),
            Value::String(value) => {
                let bytes: Vec<u8> = value.as_bytes().iter().map(|byte| !byte).collect();
                Ok(Value::String(PhpString::from_bytes(bytes)))
            }
            _ => Err("bitwise not is only implemented for int and string operands".to_owned()),
        },
    }
}

fn execute_compare(op: CompareOp, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    let value = match op {
        CompareOp::Equal => Value::Bool(equal(lhs, rhs)?),
        CompareOp::NotEqual => Value::Bool(!equal(lhs, rhs)?),
        CompareOp::Identical => Value::Bool(identical(lhs, rhs)),
        CompareOp::NotIdentical => Value::Bool(!identical(lhs, rhs)),
        CompareOp::Less => Value::Bool(compare(lhs, rhs)?.is_lt()),
        CompareOp::LessEqual => Value::Bool(!compare(lhs, rhs)?.is_gt()),
        CompareOp::Greater => Value::Bool(compare(lhs, rhs)?.is_gt()),
        CompareOp::GreaterEqual => Value::Bool(!compare(lhs, rhs)?.is_lt()),
        CompareOp::Spaceship => {
            let result = match compare(lhs, rhs)? {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            };
            Value::Int(result)
        }
    };
    Ok(value)
}

fn execute_cast(kind: CastKind, src: &Value) -> Result<Value, String> {
    match kind {
        CastKind::Bool => Ok(Value::Bool(to_bool(src)?)),
        CastKind::Int => match to_number(src)? {
            NumericValue::Int(value) => Ok(Value::Int(value)),
            NumericValue::Float(value) => Ok(Value::Int(value as i64)),
        },
        CastKind::Float => Ok(Value::float(to_number(src)?.as_f64())),
        CastKind::String => Ok(Value::String(to_string(src)?)),
        CastKind::Void => Ok(Value::Null),
        CastKind::Array => Err("array cast is not implemented".to_owned()),
        CastKind::Object => Err("object cast is not implemented".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_ir::{
        FunctionFlags, IrBuilder, IrConstant, IrSpan, Operand, RegId, UnitId,
        instruction::InstructionKind,
    };
    use php_runtime::ExitStatus;

    #[test]
    fn vm_core_returns_null_from_manual_ir() {
        let unit = manual_return_unit(IrConstant::Null);
        let result = Vm::new().execute(unit);

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.return_value, Some(Value::Null));
        assert_eq!(result.output.as_bytes(), b"");
    }

    #[test]
    fn vm_core_echoes_string_from_manual_ir() {
        let unit = manual_echo_unit(IrConstant::String("hello".to_string()));
        let result = Vm::new().execute(unit);

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.return_value, Some(Value::Null));
        assert_eq!(result.output.as_bytes(), b"hello");
    }

    #[test]
    fn vm_core_echoes_int_from_manual_ir() {
        let unit = manual_echo_unit(IrConstant::Int(123));
        let result = Vm::new().execute(unit);

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"123");
    }

    #[test]
    fn vm_core_bad_register_is_controlled_when_verifier_is_disabled() {
        let mut unit = manual_return_unit(IrConstant::Null);
        unit.functions[0].blocks[0]
            .instructions
            .push(php_ir::Instruction {
                id: php_ir::InstrId::new(1),
                span: IrSpan::new(php_ir::FileId::new(0), 0, 0),
                kind: InstructionKind::Move {
                    dst: RegId::new(0),
                    src: Operand::Register(RegId::new(99)),
                },
            });
        let vm = Vm::with_options(VmOptions {
            verify_ir: false,
            ..VmOptions::default()
        });

        let result = vm.execute(unit);

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(result.status.message(), Some("invalid register r99"));
    }

    #[test]
    fn expressions_execute_arithmetic_concat_unary_and_comparisons() {
        let result = execute_source(
            "<?php echo 1 + 2 * 3, \"|\", \"a\" . \"b\", \"|\", !false, \"|\", 2 <=> 3;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"7|ab|1|-1");
    }

    #[test]
    fn expressions_execute_casts_and_truthiness() {
        let result =
            execute_source("<?php echo (int) \"12\", \"|\", (string) true, \"|\", (bool) \"0\";");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"12|1|");
    }

    #[test]
    fn expressions_division_by_zero_is_controlled_runtime_error() {
        let result = execute_source("<?php echo 1 / 0;");

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(result.status.message(), Some("division by zero"));
    }

    #[test]
    fn constants_execute_global_const_fetches() {
        let result = execute_source(
            "<?php const ANSWER = 42; const WORD = \"ok\"; echo ANSWER, \"|\", WORD;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"42|ok");
    }

    #[test]
    fn constants_execute_builtin_php_version() {
        let result = execute_source("<?php echo PHP_VERSION;");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(
            result.output.as_bytes(),
            php_source::reference_php_version().as_bytes()
        );
    }

    #[test]
    fn constants_report_undefined_constant() {
        let result = execute_source("<?php echo MISSING_CONSTANT;");

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            result.diagnostics[0].id(),
            "E_PHP_RUNTIME_UNDEFINED_CONSTANT"
        );
    }

    #[test]
    fn constants_execute_magic_constants_top_level_and_function() {
        let source = "<?php\nfunction f() {\n echo __FUNCTION__, \"|\", __LINE__, \"|\", __CLASS__, \"|\", __METHOD__, \"|\", __NAMESPACE__;\n}\necho __FILE__, \"|\", __DIR__, \"|\", __CLASS__, \"|\", __METHOD__, \"|\", __NAMESPACE__, \"\\n\";\nf();";
        let result = execute_source(source);

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(
            result.output.as_bytes(),
            b"/tmp/phrust-test.php|/tmp|||\nf|3||f|"
        );
    }

    #[test]
    fn include_executes_local_file_and_returns_value() {
        let result = execute_fixture_file("fixtures/runtime/valid/includes/include-return.php");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"before|child:value|after\n");
    }

    #[test]
    fn include_shares_top_level_locals() {
        let result = execute_fixture_file("fixtures/runtime/valid/includes/share-variable.php");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"parent|included\n");
    }

    #[test]
    fn include_once_and_require_once_skip_second_execution() {
        let result = execute_fixture_file("fixtures/runtime/valid/includes/include-once.php");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1\n");
    }

    #[test]
    fn include_missing_warns_and_continues_but_require_missing_fails() {
        let include = execute_fixture_file("fixtures/runtime/valid/includes/include-missing.php");
        assert!(include.status.is_success(), "{:?}", include.status);
        assert_eq!(include.output.as_bytes(), b"before|after\n");
        assert_eq!(include.diagnostics[0].id(), "E_PHP_VM_INCLUDE_MISSING");
        assert_eq!(include.diagnostics[0].severity(), RuntimeSeverity::Warning);

        let require = execute_fixture_file("fixtures/runtime/invalid/includes/require-missing.php");
        assert_eq!(require.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(require.output.as_bytes(), b"before|");
        assert_eq!(require.diagnostics[0].id(), "E_PHP_VM_INCLUDE_MISSING");
        assert_eq!(
            require.diagnostics[0].severity(),
            RuntimeSeverity::FatalError
        );
    }

    #[test]
    fn objects_execute_constructor_and_public_properties() {
        let result = execute_source(
            "<?php class Box { public $value; function __construct($value) { $this->value = $value; } } $box = new Box(7); echo $box->value;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"7");
    }

    #[test]
    fn objects_keep_independent_instance_properties() {
        let result = execute_source(
            "<?php class Cell { public $value; } $left = new Cell(); $right = new Cell(); $left->value = 1; $right->value = 2; echo $left->value, '|', $right->value;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|2");
    }

    #[test]
    fn objects_report_unknown_class_and_unsupported_property_modifier() {
        let unknown = execute_source("<?php $object = new MissingObject();");

        assert_eq!(unknown.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(unknown.diagnostics[0].id(), "E_PHP_VM_UNKNOWN_CLASS");

        let private = php_ir::lower_frontend_result(
            &php_semantics::analyze_source(
                "<?php class PrivateSlot { private $value; } new PrivateSlot();",
            ),
            php_ir::LoweringOptions::default(),
        );

        assert_eq!(
            private.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER"
        );
    }

    #[test]
    fn clone_executes_shallow_object_copy_with_independent_identity() {
        let result = execute_source(
            "<?php class Cell { public $value; } $original = new Cell(); $original->value = 1; $copy = clone $original; $copy->value = 2; echo $original->value, '|', $copy->value;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|2");
    }

    #[test]
    fn clone_with_applies_public_property_replacements_to_copy() {
        let result = execute_source(
            "<?php class Box { public $name; public $count; } $original = new Box(); $original->name = 'old'; $original->count = 1; $copy = clone($original, ['name' => 'new', 'count' => 2]); echo $original->name, ':', $original->count, '|', $copy->name, ':', $copy->count;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"old:1|new:2");
    }

    #[test]
    fn clone_with_classifies_unsupported_properties() {
        let private = php_ir::lower_frontend_result(
            &php_semantics::analyze_source(
                "<?php class Secret { private $value; } $original = new Secret(); $copy = clone($original, ['value' => 1]);",
            ),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            private.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER"
        );

        let readonly = php_ir::lower_frontend_result(
            &php_semantics::analyze_source(
                "<?php class Locked { public readonly $value; } $original = new Locked(); $copy = clone($original, ['value' => 1]);",
            ),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            readonly.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER"
        );
    }

    #[test]
    fn methods_execute_instance_calls_and_this_property() {
        let result = execute_source(
            "<?php class Box { public $value; function __construct($value) { $this->value = $value; } function get() { return $this->value; } function plus($value) { return $this->get() + $value; } } $box = new Box(7); echo $box->get(), '|', $box->plus(5);",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"7|12");
    }

    #[test]
    fn methods_execute_static_calls() {
        let result = execute_source(
            "<?php class Util { static function name() { return 'ok'; } } echo Util::name();",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"ok");
    }

    #[test]
    fn methods_classify_visibility_static_and_this_gaps() {
        let private = php_ir::lower_frontend_result(
            &php_semantics::analyze_source(
                "<?php class Secret { private function hidden() { return 1; } }",
            ),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            private.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_OBJECT_METHOD_MODIFIER"
        );

        let this_outside = execute_source("<?php echo $this;");
        assert_eq!(this_outside.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            this_outside.diagnostics[0].id(),
            "E_PHP_VM_THIS_OUTSIDE_METHOD"
        );

        let static_property = php_ir::lower_frontend_result(
            &php_semantics::analyze_source("<?php class Slot { static $value; }"),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            static_property.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER"
        );
    }

    #[test]
    fn expressions_modulo_float_type_error_is_controlled() {
        let result = execute_source("<?php echo 5.5 % 2;");

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            result.status.message(),
            Some("modulo is only implemented for integer operands")
        );
    }

    #[test]
    fn variables_execute_assignment_and_fetch() {
        let result = execute_source("<?php $a = 1; echo $a;");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1");
    }

    #[test]
    fn trace_is_disabled_by_default() {
        let result = execute_source("<?php echo \"trace off\";");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"trace off");
        assert!(result.trace.is_empty());
    }

    #[test]
    fn trace_captures_deterministic_instruction_state() {
        let result = execute_source_with_options(
            "<?php $a = 1; echo $a, \"\\n\";",
            VmOptions {
                trace: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1\n");
        assert!(!result.trace.is_empty());
        assert!(
            result
                .trace
                .iter()
                .all(|line| !line.contains("0x") && !line.contains(" at ")),
            "{:#?}",
            result.trace
        );
        assert!(
            result
                .trace
                .iter()
                .any(|line| line.contains("function=main(0)")
                    && line.contains("stack_depth=1")
                    && line.contains("output_len=0")),
            "{:#?}",
            result.trace
        );
        assert!(
            result
                .trace
                .iter()
                .any(|line| line.contains("locals=[a=Int(1)]")),
            "{:#?}",
            result.trace
        );
    }

    #[test]
    fn variables_execute_compound_assignment_through_binary_ops() {
        let result = execute_source("<?php $a = 1; $a += 2; $a .= \"x\"; echo $a;");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"3x");
    }

    #[test]
    fn variables_execute_pre_and_post_inc_dec() {
        let result =
            execute_source("<?php $a = 1; echo $a++, \"|\", ++$a, \"|\", $a--, \"|\", --$a;");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|3|3|1");
    }

    #[test]
    fn variables_undefined_fetch_reads_null_in_mvp() {
        let result = execute_source("<?php echo $missing, \"x\";");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"x");
    }

    #[test]
    fn references_execute_by_value_assignment_and_local_alias_mvp() {
        let by_value = execute_source("<?php $a = 1; $b = $a; $b = 2; echo $a, $b;");

        assert!(by_value.status.is_success(), "{:?}", by_value.status);
        assert_eq!(by_value.output.as_bytes(), b"12");

        let alias = execute_source("<?php $a = 1; $b =& $a; $b = 2; echo $a; $a = 3; echo $b;");

        assert!(alias.status.is_success(), "{:?}", alias.status);
        assert_eq!(alias.output.as_bytes(), b"23");
    }

    #[test]
    fn references_reject_unsupported_categories_with_stable_ids() {
        let by_ref_param = php_ir::lower_frontend_result(
            &php_semantics::analyze_source("<?php function set_ref(&$value) { $value = 2; }"),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            by_ref_param.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_BY_REF_PARAMETER"
        );

        let by_ref_return = php_ir::lower_frontend_result(
            &php_semantics::analyze_source("<?php function &pick_ref() { return 1; }"),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            by_ref_return.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_BY_REF_RETURN"
        );

        let array_ref = php_ir::lower_frontend_result(
            &php_semantics::analyze_source("<?php $array = [1]; $alias =& $array[0];"),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            array_ref.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_ARRAY_ELEMENT_REFERENCE"
        );
    }

    #[test]
    fn control_flow_executes_if_else_and_nested_if() {
        let result = execute_source(
            "<?php $a = 0; if (false) { echo \"bad\"; } else { if (true) { echo \"ok\"; } }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"ok");
    }

    #[test]
    fn control_flow_executes_while_do_and_for_loops() {
        let result = execute_source(
            "<?php $i = 0; while ($i < 3) { echo $i; $i++; } do { echo \"d\"; $i--; } while ($i > 2); for ($j = 0; $j < 3; $j++) { echo $j; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"012d012");
    }

    #[test]
    fn control_flow_executes_break_and_continue() {
        let result = execute_source(
            "<?php $i = 0; while ($i < 5) { $i++; if ($i == 2) { continue; } if ($i == 4) { break; } echo $i; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"13");
    }

    #[test]
    fn short_circuit_skips_rhs_side_effects() {
        let result = execute_source(
            "<?php $x = 0; $y = 0; echo ($x && ++$y) ? \"bad\" : \"ok\"; echo $y; echo \"|\"; echo (true || ++$y) ? \"ok\" : \"bad\"; echo $y;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"ok0|ok0");
    }

    #[test]
    fn control_flow_executes_switch_match_ternary_coalesce_and_return() {
        let result = execute_source(
            "<?php $x = 0; switch ($x) { case 0: echo \"zero\"; case 1: echo \"one\"; break; default: echo \"default\"; } echo \"|\"; echo match ($x) { 0 => \"match\", default => \"default\" }; echo \"|\"; echo $missing ?? \"fallback\"; echo \"|\"; echo true ? \"yes\" : \"no\"; return \"done\"; echo \"bad\";",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"zeroone|match|fallback|yes");
        assert_eq!(
            result.return_value,
            Some(Value::String(php_runtime::PhpString::from_test_str("done")))
        );
    }

    #[test]
    fn control_flow_match_no_arm_is_stable_runtime_error() {
        let result = execute_source("<?php echo match (2) { 0 => \"zero\", 1 => \"one\" };");

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            result.status.message(),
            Some("E_PHP_VM_UNHANDLED_MATCH: match expression did not match any arm")
        );
    }

    #[test]
    fn functions_execute_user_calls_locals_recursion_and_null_return() {
        let result = execute_source(
            "<?php function add($a, $b) { $local = $a + $b; return $local; } function fact($n) { if ($n <= 1) { return 1; } return $n * fact($n - 1); } function empty_return() { return; } $x = 10; echo add(2, 3), \"|\", fact(5), \"|\"; echo empty_return(), \"|\", $x;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"5|120||10");
    }

    #[test]
    fn functions_runtime_errors_include_call_stack() {
        let result = execute_source(
            "<?php function boom() { echo 1 / 0; } function wrap() { boom(); } wrap();",
        );

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        let message = result.status.message().expect("runtime error message");
        assert!(message.contains("division by zero"), "{message}");
        assert!(message.contains("call_stack:"), "{message}");
        assert!(message.contains("at boom"), "{message}");
        assert!(message.contains("at wrap"), "{message}");
        assert!(message.contains("at main"), "{message}");
    }

    #[test]
    fn function_params_defaults_and_variadics_execute() {
        let result = execute_source(
            "<?php function greet($name = \"world\", $punct = \"!\") { echo \"hi \", $name, $punct; } function sum(...$xs) { return $xs[0] + $xs[1]; } greet(); echo \"|\"; greet(\"php\", \"?\"); echo \"|\", sum(2, 3);",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"hi world!|hi php?|5");
    }

    #[test]
    fn function_params_argument_count_errors_are_stable() {
        let missing = execute_source("<?php function one($a) { return $a; } one();");
        assert_eq!(missing.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            missing.status.message(),
            Some("E_PHP_VM_TOO_FEW_ARGS: function one expects at least 1 argument(s), got 0")
        );

        let extra = execute_source("<?php function one($a) { return $a; } one(1, 2);");
        assert_eq!(extra.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            extra.status.message(),
            Some("E_PHP_VM_TOO_MANY_ARGS: function one expects at most 1 argument(s), got 2")
        );
    }

    #[test]
    fn function_params_return_type_success_and_failure() {
        let success = execute_source(
            "<?php function text(): string { return \"ok\"; } function number(): int { return 4; } function nothing(): void { return; } echo text(), \"|\", number(), \"|\"; echo nothing(), \"x\";",
        );
        assert!(success.status.is_success(), "{:?}", success.status);
        assert_eq!(success.output.as_bytes(), b"ok|4|x");

        let failure = execute_source("<?php function bad(): int { return \"no\"; } bad();");
        assert_eq!(failure.status.exit_status(), ExitStatus::RuntimeError);
        let message = failure.status.message().expect("runtime error message");
        assert!(
            message.contains("E_PHP_VM_RETURN_TYPE_MISMATCH"),
            "{message}"
        );
        assert!(
            message.contains("function bad returned string, expected int"),
            "{message}"
        );
    }

    #[test]
    fn runtime_types_check_scalar_params_and_nullable_values() {
        let success = execute_source(
            "<?php function add_one(int $value): int { return $value + 1; } function label(?string $value): string { if ($value === null) { return 'none'; } return $value; } echo add_one(4), '|', label(null), '|', label('ok');",
        );
        assert!(success.status.is_success(), "{:?}", success.status);
        assert_eq!(success.output.as_bytes(), b"5|none|ok");

        let failure = execute_source(
            "<?php function add_one(int $value): int { return $value + 1; } add_one([]);",
        );
        assert_eq!(failure.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(failure.diagnostics[0].id(), "E_PHP_VM_PARAM_TYPE_MISMATCH");
        let message = failure.status.message().expect("runtime error message");
        assert!(
            message.contains("function add_one argument $value got array, expected int"),
            "{message}"
        );
    }

    #[test]
    fn runtime_types_check_returns_void_and_properties() {
        let success = execute_source(
            "<?php class Box { public int $value; } function text(): string { return 'ok'; } function done(): void { return; } $box = new Box(); $box->value = 7; echo text(), '|', done(), '|', $box->value;",
        );
        assert!(success.status.is_success(), "{:?}", success.status);
        assert_eq!(success.output.as_bytes(), b"ok||7");

        let bad_return = execute_source("<?php function text(): string { return []; } text();");
        assert_eq!(bad_return.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            bad_return.diagnostics[0].id(),
            "E_PHP_VM_RETURN_TYPE_MISMATCH"
        );

        let bad_property = execute_source(
            "<?php class Box { public int $value; } $box = new Box(); $box->value = [];",
        );
        assert_eq!(bad_property.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            bad_property.diagnostics[0].id(),
            "E_PHP_VM_PROPERTY_TYPE_MISMATCH"
        );

        let bad_void =
            php_semantics::analyze_source("<?php function bad(): void { return null; } bad();");
        assert!(bad_void.has_errors());
        assert!(
            bad_void
                .semantic_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.id().as_str()
                    == "E_PHP_RETURN_VALUE_FROM_VOID_FUNCTION")
        );
    }

    #[test]
    fn closures_execute_simple_calls_captures_arrows_and_returns() {
        let simple = execute_source("<?php $f = function($x) { return $x + 1; }; echo $f(2);");
        assert!(simple.status.is_success(), "{:?}", simple.status);
        assert_eq!(simple.output.as_bytes(), b"3");

        let use_by_value = execute_source(
            "<?php $x = 2; $f = function($y) use ($x) { return $x + $y; }; $x = 100; echo $f(3);",
        );
        assert!(
            use_by_value.status.is_success(),
            "{:?}",
            use_by_value.status
        );
        assert_eq!(use_by_value.output.as_bytes(), b"5");

        let arrow = execute_source("<?php $x = 4; $f = fn($y) => $x + $y; $x = 100; echo $f(3);");
        assert!(arrow.status.is_success(), "{:?}", arrow.status);
        assert_eq!(arrow.output.as_bytes(), b"7");

        let returned = execute_source(
            "<?php function make($x) { return function() use ($x) { return $x; }; } $f = make(9); echo $f();",
        );
        assert!(returned.status.is_success(), "{:?}", returned.status);
        assert_eq!(returned.output.as_bytes(), b"9");
    }

    #[test]
    fn closures_by_ref_capture_is_stable_runtime_gap() {
        let result =
            execute_source("<?php $x = 1; $f = function() use (&$x) { return $x; }; $f();");

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        let message = result.status.message().expect("runtime error message");
        assert!(
            message.contains("E_PHP_VM_UNSUPPORTED_BY_REF_CAPTURE"),
            "{message}"
        );
    }

    #[test]
    fn pipe_executes_user_function_closure_builtin_and_non_callable_error() {
        let user_function =
            execute_source("<?php function plus1($x) { return $x + 1; } echo 2 |> plus1(...);");
        assert!(
            user_function.status.is_success(),
            "{:?}",
            user_function.status
        );
        assert_eq!(user_function.output.as_bytes(), b"3");

        let closure = execute_source("<?php $f = fn($x) => $x + 2; echo 2 |> $f;");
        assert!(closure.status.is_success(), "{:?}", closure.status);
        assert_eq!(closure.output.as_bytes(), b"4");

        let builtin = execute_source(
            "<?php echo \" a \" |> trim(...), \"|\"; echo \"ab\" |> strlen(...), \"|\"; echo \"hi\" |> strtoupper(...);",
        );
        assert!(builtin.status.is_success(), "{:?}", builtin.status);
        assert_eq!(builtin.output.as_bytes(), b"a|2|HI");

        let side_effects = execute_source(
            "<?php function id($x) { return $x; } $x = 0; echo ($x = 7) |> id(...); echo \"|\", $x;",
        );
        assert!(
            side_effects.status.is_success(),
            "{:?}",
            side_effects.status
        );
        assert_eq!(side_effects.output.as_bytes(), b"7|7");

        let not_callable = execute_source("<?php echo 2 |> 4;");
        assert_eq!(not_callable.status.exit_status(), ExitStatus::RuntimeError);
        let message = not_callable
            .status
            .message()
            .expect("runtime error message");
        assert!(
            message.contains("E_PHP_VM_PIPE_RHS_NOT_CALLABLE"),
            "{message}"
        );
    }

    #[test]
    fn arrays_execute_indexed_and_string_key_literals() {
        let result = execute_source(
            "<?php $a = [1, 2, \"x\" => 3]; echo $a[0], \"|\", $a[1], \"|\", $a[\"x\"]; ",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|2|3");
    }

    #[test]
    fn arrays_execute_append_and_overwrite_assignments() {
        let result = execute_source(
            "<?php $a = []; $a[] = 1; $a[] = 2; $a[1] = 5; $a[\"k\"] = 7; echo $a[0], \"|\", $a[1], \"|\", $a[\"k\"];",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|5|7");
    }

    #[test]
    fn arrays_execute_nested_fetch_and_assignment() {
        let result = execute_source(
            "<?php $a = [\"outer\" => [\"inner\" => 4]]; $a[\"outer\"][\"next\"] = 8; echo $a[\"outer\"][\"inner\"], \"|\", $a[\"outer\"][\"next\"]; ",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"4|8");
    }

    #[test]
    fn arrays_missing_key_warns_and_reads_null() {
        let result = execute_source("<?php $a = []; echo $a[\"missing\"], \"x\";");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"x");
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].id(),
            "E_PHP_RUNTIME_UNDEFINED_ARRAY_KEY_WARNING"
        );
    }

    #[test]
    fn arrays_execute_isset_empty_and_unset() {
        let result = execute_source(
            "<?php $a = [\"x\" => 0, \"y\" => 1]; echo isset($a[\"x\"]), isset($a[\"z\"]), \"|\"; echo empty($a[\"x\"]), empty($a[\"z\"]), empty($missing), \"|\"; unset($a[\"y\"]); echo isset($a[\"y\"]);",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|111|");
    }

    #[test]
    fn exceptions_catch_exception_object() {
        let result = execute_source(
            "<?php try { throw new Exception(\"boom\"); } catch (Exception $e) { echo \"caught:\", $e->message; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"caught:boom");
    }

    #[test]
    fn exceptions_run_finally_before_return() {
        let result = execute_source(
            "<?php function f() { try { return \"body\"; } finally { echo \"finally|\"; } } echo f();",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"finally|body");
    }

    #[test]
    fn exceptions_run_finally_before_uncaught_throw() {
        let result = execute_source(
            "<?php try { throw new Exception(\"boom\"); } finally { echo \"finally\"; }",
        );

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(result.output.as_bytes(), b"finally");
        assert_eq!(result.diagnostics[0].id(), "E_PHP_VM_UNCAUGHT_EXCEPTION");
    }

    #[test]
    fn exceptions_rethrow_from_catch_is_uncaught() {
        let result = execute_source(
            "<?php try { throw new Exception(\"boom\"); } catch (Exception $e) { echo \"catch|\"; throw $e; }",
        );

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(result.output.as_bytes(), b"catch|");
        assert_eq!(result.diagnostics[0].id(), "E_PHP_VM_UNCAUGHT_EXCEPTION");
    }

    #[test]
    fn foreach_executes_value_iteration() {
        let result = execute_source("<?php foreach ([1, 2, 3] as $value) { echo $value; }");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"123");
    }

    #[test]
    fn foreach_executes_key_value_iteration_in_insertion_order() {
        let result = execute_source(
            "<?php foreach ([\"a\" => 1, 4 => 2, \"b\" => 3] as $key => $value) { echo $key, \":\", $value, \";\"; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"a:1;4:2;b:3;");
    }

    #[test]
    fn foreach_executes_break_continue_and_nested_loops() {
        let flow = execute_source(
            "<?php foreach ([1, 2, 3, 4] as $value) { if ($value == 2) { continue; } if ($value == 4) { break; } echo $value; }",
        );
        assert!(flow.status.is_success(), "{:?}", flow.status);
        assert_eq!(flow.output.as_bytes(), b"13");

        let nested = execute_source(
            "<?php foreach ([\"a\", \"b\"] as $left) { foreach ([1, 2] as $right) { echo $left, $right, \";\"; } }",
        );
        assert!(nested.status.is_success(), "{:?}", nested.status);
        assert_eq!(nested.output.as_bytes(), b"a1;a2;b1;b2;");
    }

    #[test]
    fn foreach_uses_snapshot_iteration_for_mutated_arrays() {
        let result = execute_source(
            "<?php $items = [1, 2]; foreach ($items as $value) { echo $value; $items[] = 9; } echo \"|\"; foreach ($items as $value) { echo $value; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"12|1299");
    }

    #[test]
    fn foreach_by_ref_is_stable_known_gap() {
        let frontend =
            php_semantics::analyze_source("<?php foreach ([1] as &$value) { echo $value; }");
        let lowering = php_ir::lower_frontend_result(&frontend, php_ir::LoweringOptions::default());

        assert!(
            lowering.verification.is_ok(),
            "{:#?}",
            lowering.verification
        );
        assert_eq!(lowering.diagnostics.len(), 1);
        assert_eq!(
            lowering.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH"
        );
        let result = Vm::new().execute(lowering.unit);
        assert_eq!(result.status.exit_status(), ExitStatus::Unsupported);
        assert_eq!(
            result.diagnostics[0].id(),
            "E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH"
        );
    }

    #[test]
    fn unsupported_prompt31_known_gaps_surface_stable_runtime_ids() {
        let diagnostic_ids = [
            "E_PHP_IR_UNSUPPORTED_GENERATOR",
            "E_PHP_IR_UNSUPPORTED_YIELD_FROM",
            "E_PHP_IR_UNSUPPORTED_FIBER",
            "E_PHP_IR_UNSUPPORTED_EVAL",
            "E_PHP_IR_UNSUPPORTED_AUTOLOAD",
            "E_PHP_IR_UNSUPPORTED_REFLECTION",
            "E_PHP_IR_UNSUPPORTED_TRAIT_RUNTIME",
            "E_PHP_IR_UNSUPPORTED_ENUM_RUNTIME",
            "E_PHP_IR_UNSUPPORTED_PROPERTY_HOOKS",
            "E_PHP_IR_UNSUPPORTED_REFERENCE_SEMANTICS",
        ];

        for diagnostic_id in diagnostic_ids {
            let result = Vm::with_options(VmOptions {
                verify_ir: false,
                ..VmOptions::default()
            })
            .execute(manual_unsupported_unit_for(diagnostic_id));
            assert_eq!(
                result.status.exit_status(),
                ExitStatus::Unsupported,
                "{diagnostic_id}: {:?}",
                result.status
            );
            assert_eq!(result.diagnostics[0].id(), diagnostic_id);
        }
    }

    #[test]
    fn builtins_execute_direct_calls_print_var_dump_and_callable_resolution() {
        let direct = execute_source(
            "<?php echo gettype(null), \"|\", gettype(7), \"|\", gettype(\"x\"), \"|\"; echo is_int(7), is_string(\"x\"), is_bool(false), is_null(null), is_array(null);",
        );
        assert!(direct.status.is_success(), "{:?}", direct.status);
        assert_eq!(direct.output.as_bytes(), b"NULL|integer|string|1111");

        let print = execute_source("<?php echo print \"x\";");
        assert!(print.status.is_success(), "{:?}", print.status);
        assert_eq!(print.output.as_bytes(), b"x1");

        let dump = execute_source(
            "<?php function dump_args(...$args) { var_dump($args); } var_dump(null, true, 7, \"hi\"); dump_args(1, \"x\");",
        );
        assert!(dump.status.is_success(), "{:?}", dump.status);
        assert_eq!(
            dump.output.to_string_lossy(),
            "NULL\nbool(true)\nint(7)\nstring(2) \"hi\"\narray(2) {\n  [0]=>\n  int(1)\n  [1]=>\n  string(1) \"x\"\n}\n"
        );

        let callable =
            execute_source("<?php echo \"abc\" |> gettype(...), \"|\", \"abc\" |> strlen(...);");
        assert!(callable.status.is_success(), "{:?}", callable.status);
        assert_eq!(callable.output.as_bytes(), b"string|3");
    }

    #[test]
    fn runtime_errors_emit_structured_diagnostics_and_warning_continuation() {
        let division = execute_source("<?php echo 1 / 0;");
        assert_eq!(division.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(division.status.message(), Some("division by zero"));
        assert_eq!(division.diagnostics.len(), 1);
        assert_eq!(
            division.diagnostics[0].id(),
            "E_PHP_RUNTIME_DIVISION_BY_ZERO"
        );
        assert_eq!(division.diagnostics[0].stack_trace()[0].function(), "main");

        let undefined = execute_source("<?php missing_function();");
        assert_eq!(undefined.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            undefined.diagnostics[0].id(),
            "E_PHP_RUNTIME_UNDEFINED_FUNCTION"
        );

        let stack = execute_source("<?php function boom() { echo 1 / 0; } boom();");
        assert_eq!(stack.status.exit_status(), ExitStatus::RuntimeError);
        let frames = stack.diagnostics[0]
            .stack_trace()
            .iter()
            .map(|frame| frame.function())
            .collect::<Vec<_>>();
        assert_eq!(frames, vec!["boom", "main"]);

        let warning = execute_source("<?php echo $missing, \"ok\";");
        assert!(warning.status.is_success(), "{:?}", warning.status);
        assert_eq!(warning.output.as_bytes(), b"ok");
        assert_eq!(warning.diagnostics.len(), 1);
        assert_eq!(
            warning.diagnostics[0].id(),
            "E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING"
        );
        assert_eq!(warning.diagnostics[0].severity(), RuntimeSeverity::Warning);

        let unsupported = Vm::with_options(VmOptions {
            verify_ir: false,
            ..VmOptions::default()
        })
        .execute(manual_unsupported_unit());
        assert_eq!(unsupported.status.exit_status(), ExitStatus::Unsupported);
        assert_eq!(unsupported.diagnostics.len(), 1);
        assert_eq!(
            unsupported.diagnostics[0].id(),
            "E_PHP_RUNTIME_UNSUPPORTED_GENERATOR_EXECUTION"
        );
        assert_eq!(
            unsupported.diagnostics[0].severity(),
            RuntimeSeverity::UnsupportedFeature
        );
    }

    fn manual_return_unit(value: IrConstant) -> php_ir::IrUnit {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("manual.php");
        let function = builder.start_function(
            "main",
            FunctionFlags {
                is_top_level: true,
                ..FunctionFlags::default()
            },
            IrSpan::new(file, 0, 0),
        );
        let block = builder.append_block(function);
        let constant = builder.add_constant(value);
        builder.terminate_return(
            function,
            block,
            Some(Operand::Constant(constant)),
            IrSpan::new(file, 0, 0),
        );
        builder.set_entry(function);
        builder.finish()
    }

    fn execute_source(source: &str) -> VmResult {
        execute_source_with_options(source, VmOptions::default())
    }

    fn execute_source_with_options(source: &str, options: VmOptions) -> VmResult {
        let frontend = php_semantics::analyze_source(source);
        assert!(
            !frontend.has_errors(),
            "frontend errors: {:?}",
            frontend.semantic_diagnostics()
        );
        let lowering = php_ir::lower_frontend_result(
            &frontend,
            php_ir::LoweringOptions {
                source_path: "/tmp/phrust-test.php".to_owned(),
                source_text: Some(source.to_owned()),
                ..php_ir::LoweringOptions::default()
            },
        );
        assert!(
            lowering.diagnostics.is_empty(),
            "{:#?}",
            lowering.diagnostics
        );
        assert!(
            lowering.verification.is_ok(),
            "{:#?}",
            lowering.verification
        );
        Vm::with_options(options).execute(lowering.unit)
    }

    fn execute_fixture_file(path: &str) -> VmResult {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crate should live under workspace/crates/php_vm");
        let path = workspace.join(path);
        let source = std::fs::read_to_string(&path).expect("fixture should be readable");
        let frontend = php_semantics::analyze_source(&source);
        assert!(
            !frontend.has_errors(),
            "frontend errors: {:?}",
            frontend.semantic_diagnostics()
        );
        let canonical = std::fs::canonicalize(&path).expect("fixture should canonicalize");
        let lowering = php_ir::lower_frontend_result(
            &frontend,
            php_ir::LoweringOptions {
                source_path: canonical.to_string_lossy().into_owned(),
                source_text: Some(source),
                ..php_ir::LoweringOptions::default()
            },
        );
        assert!(
            lowering.diagnostics.is_empty(),
            "{:#?}",
            lowering.diagnostics
        );
        assert!(
            lowering.verification.is_ok(),
            "{:#?}",
            lowering.verification
        );
        let loader = IncludeLoader::for_root(
            canonical
                .parent()
                .expect("fixture should have parent")
                .to_path_buf(),
        )
        .expect("include loader should initialize");
        Vm::with_options(VmOptions {
            include_loader: Some(loader),
            ..VmOptions::default()
        })
        .execute(lowering.unit)
    }

    fn manual_echo_unit(value: IrConstant) -> php_ir::IrUnit {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("manual.php");
        let function = builder.start_function(
            "main",
            FunctionFlags {
                is_top_level: true,
                ..FunctionFlags::default()
            },
            IrSpan::new(file, 0, 0),
        );
        let block = builder.append_block(function);
        let value = builder.add_constant(value);
        let null = builder.add_constant(IrConstant::Null);
        let register = builder.alloc_register(function);
        builder.emit_load_const(function, block, register, value, IrSpan::new(file, 0, 0));
        builder.emit(
            function,
            block,
            InstructionKind::Echo {
                src: Operand::Register(register),
            },
            IrSpan::new(file, 0, 0),
        );
        builder.terminate_return(
            function,
            block,
            Some(Operand::Constant(null)),
            IrSpan::new(file, 0, 0),
        );
        builder.set_entry(function);
        builder.finish()
    }

    fn manual_unsupported_unit() -> php_ir::IrUnit {
        manual_unsupported_unit_for("E_PHP_RUNTIME_UNSUPPORTED_GENERATOR_EXECUTION")
    }

    fn manual_unsupported_unit_for(diagnostic_id: &str) -> php_ir::IrUnit {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("manual.php");
        let function = builder.start_function(
            "main",
            FunctionFlags {
                is_top_level: true,
                ..FunctionFlags::default()
            },
            IrSpan::new(file, 0, 0),
        );
        let block = builder.append_block(function);
        let null = builder.add_constant(IrConstant::Null);
        builder.emit(
            function,
            block,
            InstructionKind::Unsupported {
                diagnostic_id: diagnostic_id.to_owned(),
            },
            IrSpan::new(file, 0, 0),
        );
        builder.terminate_return(
            function,
            block,
            Some(Operand::Constant(null)),
            IrSpan::new(file, 0, 0),
        );
        builder.set_entry(function);
        builder.finish()
    }
}
