//! First minimal VM dispatch loop.

#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]

use crate::compiled_unit::CompiledUnit;
use crate::frame::{CallStack, Frame};
use crate::include::IncludeLoader;
use php_ir::constants::IrConstant;
use php_ir::function::{IrFunction, IrParam, IrReturnType};
use php_ir::ids::{BlockId, ConstId, FunctionId, LocalId};
use php_ir::instruction::{
    BinaryOp, CallableKind, CastKind, ClosureCaptureArg, CompareOp, IncludeKind, Instruction,
    InstructionKind, IrCallArg, TerminatorKind, UnaryOp,
};
use php_ir::module::IrUnit;
use php_ir::operand::Operand;
use php_ir::verify::verify_unit;
use php_runtime::{
    ArrayKey, AttributeEntry as RuntimeAttributeEntry, AutoloadRegistry, BuiltinContext,
    BuiltinRegistry, CallableValue, ClassConstantEntry as RuntimeClassConstantEntry,
    ClassConstantFlags as RuntimeClassConstantFlags, ClassEntry as RuntimeClassEntry,
    ClassEnumBackingType as RuntimeClassEnumBackingType,
    ClassEnumCaseEntry as RuntimeClassEnumCaseEntry, ClassFlags as RuntimeClassFlags,
    ClassMethodEntry as RuntimeClassMethodEntry, ClassMethodFlags as RuntimeClassMethodFlags,
    ClassPropertyEntry as RuntimeClassPropertyEntry,
    ClassPropertyFlags as RuntimeClassPropertyFlags,
    ClassPropertyHooks as RuntimeClassPropertyHooks, ClosureCaptureValue, ExecutionStatus,
    FiberRef, FiberState, GeneratorRef, GeneratorState, GlobalSymbolTable, NumericValue, ObjectRef,
    OutputBuffer, PhpArray, PhpString, ReferenceCell, RuntimeContext, RuntimeDiagnostic,
    RuntimeSeverity, RuntimeSourceSpan, RuntimeStackFrame, RuntimeType, Value, compare,
    division_by_zero_mvp, equal, identical, runtime_type_name, to_bool, to_float, to_int,
    to_number, to_string, undefined_function, undefined_variable_warning, unsupported_feature,
    value_matches_runtime_type, value_type_name,
};
#[cfg(test)]
use php_runtime::{GcEntityId, GcEntityKind};
use php_runtime::{GcRoot, GcRootKind, GcSnapshot, scan_roots};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_EVAL_DEPTH: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ForeachIterator {
    Snapshot {
        entries: Vec<(ArrayKey, Value)>,
        position: usize,
    },
    ObjectProperties {
        object: ObjectRef,
        position: usize,
    },
    IteratorObject {
        object: ObjectRef,
        needs_next: bool,
    },
    ByReference {
        local: LocalId,
        position: usize,
    },
    Generator {
        generator: GeneratorRef,
        consumed: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExceptionHandler {
    catch: Option<BlockId>,
    catch_types: Vec<String>,
    finally: Option<BlockId>,
    after: BlockId,
    exception_local: Option<LocalId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingControl {
    Return(Option<Value>),
    Throw(Value),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparedArg {
    value: Value,
    reference: Option<ReferenceCell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratorYield {
    key: Option<Value>,
    value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GeneratorContinuation {
    frame: Frame,
    block_id: BlockId,
    instruction_index: usize,
    yield_result: php_ir::ids::RegId,
    foreach_iterators: HashMap<php_ir::ids::RegId, ForeachIterator>,
    exception_handlers: Vec<ExceptionHandler>,
    pending_control: Option<PendingControl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FiberContinuation {
    frame: Frame,
    block_id: BlockId,
    instruction_index: usize,
    resume_result: php_ir::ids::RegId,
    foreach_iterators: HashMap<php_ir::ids::RegId, ForeachIterator>,
    exception_handlers: Vec<ExceptionHandler>,
    pending_control: Option<PendingControl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FiberResumeInput {
    Value(Value),
    Throw(Value),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FiberSuspension {
    value: Value,
    continuations: Vec<FiberContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum GeneratorResumeInput {
    Value(Value),
    Throw(Value),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct YieldFromKey {
    generator_id: u64,
    block_id: BlockId,
    instruction_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum YieldFromDelegation {
    Array {
        entries: Vec<(ArrayKey, Value)>,
        position: usize,
    },
    Generator {
        generator: GeneratorRef,
        started: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum YieldFromStep {
    Yield { key: Option<Value>, value: Value },
    Complete(Value),
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
    /// Capture deterministic runtime object, reference, COW, and suspension events.
    pub trace_runtime: bool,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            verify_ir: true,
            max_steps: 100_000,
            include_loader: None,
            runtime_context: RuntimeContext::default(),
            trace: false,
            trace_runtime: false,
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
    yielded: Option<GeneratorYield>,
    fiber_suspension: Option<FiberSuspension>,
    return_ref: Option<ReferenceCell>,
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
    globals: GlobalSymbolTable,
    included_once: Vec<PathBuf>,
    static_locals: HashMap<(u32, String), ReferenceCell>,
    static_properties: HashMap<(String, String), Value>,
    enum_cases: HashMap<(String, String), ObjectRef>,
    destructor_queue: DestructorQueue,
    magic_property_stack: Vec<MagicPropertyCall>,
    magic_method_stack: Vec<MagicMethodCall>,
    property_hook_stack: Vec<PropertyHookCall>,
    generator_continuations: HashMap<u64, GeneratorContinuation>,
    fiber_continuations: HashMap<u64, Vec<FiberContinuation>>,
    yield_from_delegations: HashMap<YieldFromKey, YieldFromDelegation>,
    eval_depth: usize,
    eval_counter: usize,
    autoload_registry: AutoloadRegistry,
    autoload_stack: Vec<String>,
    dynamic_classes: Vec<php_ir::module::ClassEntry>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DestructorQueue {
    entries: Vec<DestructorEntry>,
}

impl DestructorQueue {
    fn register(&mut self, object: ObjectRef, class_name: String, function: FunctionId) {
        if self
            .entries
            .iter()
            .any(|entry| entry.object.id() == object.id())
        {
            return;
        }
        self.entries.push(DestructorEntry {
            object,
            class_name,
            function,
        });
    }

    fn drain_reverse(&mut self) -> Vec<DestructorEntry> {
        let mut entries = std::mem::take(&mut self.entries);
        entries.reverse();
        entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DestructorEntry {
    object: ObjectRef,
    class_name: String,
    function: FunctionId,
}

fn gc_snapshot_from_vm_roots(stack: &CallStack, state: &ExecutionState) -> GcSnapshot {
    scan_roots(gc_roots_from_vm(stack, state))
}

fn gc_root_count_from_vm_roots(stack: &CallStack, state: &ExecutionState) -> usize {
    gc_roots_from_vm(stack, state).len()
}

fn gc_roots_from_vm(stack: &CallStack, state: &ExecutionState) -> Vec<GcRoot> {
    let mut roots = Vec::new();
    for (frame_index, frame) in stack.frames().iter().enumerate() {
        for (index, value) in frame.registers.iter() {
            roots.push(GcRoot::value(
                GcRootKind::FrameRegister,
                format!("frame{frame_index}.r{index}"),
                value.clone(),
            ));
        }
        for (index, slot) in frame.locals.iter() {
            roots.push(GcRoot::slot(
                GcRootKind::FrameLocal,
                format!("frame{frame_index}.local{index}"),
                slot,
            ));
        }
    }
    for ((function, name), cell) in &state.static_locals {
        roots.push(GcRoot::value(
            GcRootKind::StaticLocal,
            format!("static-local:{function}:{name}"),
            Value::Reference(cell.clone()),
        ));
    }
    for ((class_name, property), value) in &state.static_properties {
        roots.push(GcRoot::value(
            GcRootKind::ClassTable,
            format!("static-property:{class_name}::{property}"),
            value.clone(),
        ));
    }
    for ((class_name, case_name), object) in &state.enum_cases {
        roots.push(GcRoot::value(
            GcRootKind::ClassTable,
            format!("enum-case:{class_name}::{case_name}"),
            Value::Object(object.clone()),
        ));
    }
    for (index, entry) in state.destructor_queue.entries.iter().enumerate() {
        roots.push(GcRoot::value(
            GcRootKind::DestructorQueue,
            format!("destructor-queue:{index}"),
            Value::Object(entry.object.clone()),
        ));
    }
    for (fiber_id, continuations) in &state.fiber_continuations {
        for (continuation_index, continuation) in continuations.iter().enumerate() {
            for (index, value) in continuation.frame.registers.iter() {
                roots.push(GcRoot::value(
                    GcRootKind::FiberStack,
                    format!("fiber{fiber_id}.continuation{continuation_index}.r{index}"),
                    value.clone(),
                ));
            }
            for (index, slot) in continuation.frame.locals.iter() {
                roots.push(GcRoot::slot(
                    GcRootKind::FiberStack,
                    format!("fiber{fiber_id}.continuation{continuation_index}.local{index}"),
                    slot,
                ));
            }
        }
    }
    roots
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MagicPropertyCall {
    object_id: u64,
    method: String,
    property: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MagicMethodCall {
    receiver: String,
    magic_method: String,
    called_method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PropertyHookCall {
    object_id: u64,
    class_name: String,
    property: String,
}

struct FunctionCall<'a> {
    args: Vec<CallArgument>,
    captures: Vec<ClosureCaptureValue>,
    this_value: Option<ObjectRef>,
    scope_class: Option<String>,
    called_class: Option<String>,
    declaring_class: Option<String>,
    shared_top_level_locals: Option<&'a mut HashMap<String, Value>>,
    running_generator: Option<GeneratorRef>,
    resume_continuation: Option<GeneratorContinuation>,
    resume_input: Option<GeneratorResumeInput>,
    running_fiber: Option<FiberRef>,
    resume_fiber_continuation: Option<FiberContinuation>,
    resume_fiber_input: Option<FiberResumeInput>,
}

impl FunctionCall<'_> {
    fn new(args: Vec<CallArgument>, captures: Vec<ClosureCaptureValue>) -> Self {
        Self {
            args,
            captures,
            this_value: None,
            scope_class: None,
            called_class: None,
            declaring_class: None,
            shared_top_level_locals: None,
            running_generator: None,
            resume_continuation: None,
            resume_input: None,
            running_fiber: None,
            resume_fiber_continuation: None,
            resume_fiber_input: None,
        }
    }

    fn running_generator(mut self, generator: GeneratorRef) -> Self {
        self.running_generator = Some(generator);
        self
    }

    fn resume_generator(
        mut self,
        continuation: GeneratorContinuation,
        input: GeneratorResumeInput,
    ) -> Self {
        self.resume_continuation = Some(continuation);
        self.resume_input = Some(input);
        self
    }

    fn running_fiber(mut self, fiber: FiberRef) -> Self {
        self.running_fiber = Some(fiber);
        self
    }

    fn inherit_fiber_context(mut self, fiber: &Option<FiberRef>) -> Self {
        self.running_fiber = fiber.clone();
        self
    }

    fn resume_fiber(
        mut self,
        fiber: FiberRef,
        continuation: FiberContinuation,
        input: FiberResumeInput,
    ) -> Self {
        self.running_fiber = Some(fiber);
        self.resume_fiber_continuation = Some(continuation);
        self.resume_fiber_input = Some(input);
        self
    }

    fn with_this(mut self, this_value: ObjectRef) -> Self {
        self.this_value = Some(this_value);
        self
    }

    fn with_class_context(
        mut self,
        scope_class: impl Into<String>,
        called_class: impl Into<String>,
        declaring_class: impl Into<String>,
    ) -> Self {
        self.scope_class = Some(normalize_class_name(&scope_class.into()));
        self.called_class = Some(normalize_class_name(&called_class.into()));
        self.declaring_class = Some(normalize_class_name(&declaring_class.into()));
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CallArgument {
    name: Option<String>,
    value: Value,
    by_ref_local: Option<LocalId>,
}

impl CallArgument {
    fn positional(value: Value) -> Self {
        Self {
            name: None,
            value,
            by_ref_local: None,
        }
    }
}

impl VmResult {
    fn success(output: OutputBuffer, return_value: Option<Value>) -> Self {
        Self {
            status: ExecutionStatus::success(),
            output,
            diagnostics: Vec::new(),
            return_value,
            yielded: None,
            fiber_suspension: None,
            return_ref: None,
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
            yielded: None,
            fiber_suspension: None,
            return_ref: None,
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
            yielded: None,
            fiber_suspension: None,
            return_ref: None,
            trace: Vec::new(),
        }
    }

    fn compile_error(output: OutputBuffer, message: impl Into<String>) -> Self {
        Self {
            status: ExecutionStatus::compile_error(message),
            output,
            diagnostics: Vec::new(),
            return_value: None,
            yielded: None,
            fiber_suspension: None,
            return_ref: None,
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
        if let Err(message) = validate_class_table(&unit) {
            return VmResult::compile_error(output, message);
        }

        let mut stack = CallStack::new();
        let mut state = ExecutionState::default();
        seed_runtime_globals(&mut state.globals, &self.options.runtime_context);
        let mut result = self.execute_function(
            &unit,
            entry,
            FunctionCall::new(Vec::new(), Vec::new()),
            &mut output,
            &mut stack,
            &mut state,
        );
        if result.status.is_success() {
            match self.run_shutdown_destructors(&unit, &mut output, &mut state) {
                Ok(diagnostics) => {
                    result.diagnostics.extend(diagnostics);
                    result.output = output.clone();
                }
                Err(error) => {
                    result = error;
                }
            }
        }
        if self.options.trace_runtime {
            self.record_gc_root_trace_event(&stack, &state);
        }
        if self.options.trace || self.options.trace_runtime {
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

    fn record_lvalue_trace_event(&self, operation: &str, local: LocalId, dims: &[ArrayKey]) {
        if !(self.options.trace || self.options.trace_runtime) {
            return;
        }
        let mut trace = self.trace.borrow_mut();
        let step = trace.len() + 1;
        trace.push(format!(
            "step={step} runtime lvalue operation={operation} local={} path=[{}]",
            local.raw(),
            dims.iter()
                .map(format_array_key_for_trace)
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    fn record_runtime_trace_event(&self, event: impl AsRef<str>) {
        if !self.options.trace_runtime {
            return;
        }
        let mut trace = self.trace.borrow_mut();
        let step = trace.len() + 1;
        trace.push(format!("step={step} runtime {}", event.as_ref()));
    }

    fn record_gc_root_trace_event(&self, stack: &CallStack, state: &ExecutionState) {
        if !self.options.trace_runtime {
            return;
        }
        let root_count = gc_root_count_from_vm_roots(stack, state);
        let snapshot = gc_snapshot_from_vm_roots(stack, state);
        self.record_runtime_trace_event(format!(
            "gc-roots roots={} entities={} cycle_candidates={}",
            root_count,
            snapshot.nodes.len(),
            snapshot.cycle_candidates.len()
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
        let mut block_id;
        let mut start_instruction_index = 0usize;
        let mut steps = 0usize;
        let mut foreach_iterators;
        let mut exception_handlers: Vec<ExceptionHandler>;
        let mut pending_control: Option<PendingControl>;
        let running_fiber = call.running_fiber.clone();

        if let Some(continuation) = call.resume_fiber_continuation.take() {
            block_id = continuation.block_id;
            start_instruction_index = continuation.instruction_index;
            foreach_iterators = continuation.foreach_iterators;
            exception_handlers = continuation.exception_handlers;
            pending_control = continuation.pending_control;
            stack.push(continuation.frame);
            match call
                .resume_fiber_input
                .take()
                .unwrap_or(FiberResumeInput::Value(Value::Null))
            {
                FiberResumeInput::Value(value) => {
                    if let Err(message) = stack
                        .current_mut()
                        .expect("resumed fiber frame is active")
                        .registers
                        .set(continuation.resume_result, value)
                    {
                        return self.runtime_error(output, compiled, stack, message);
                    }
                }
                FiberResumeInput::Throw(value) => {
                    if let Some(target) = handle_throw(
                        compiled,
                        value.clone(),
                        &mut exception_handlers,
                        stack,
                        &mut pending_control,
                    ) {
                        block_id = target;
                        start_instruction_index = 0;
                    } else {
                        stack.pop();
                        return uncaught_exception(output, compiled, stack, value);
                    }
                }
            }
        } else if let Some(continuation) = call.resume_continuation.take() {
            block_id = continuation.block_id;
            start_instruction_index = continuation.instruction_index;
            foreach_iterators = continuation.foreach_iterators;
            exception_handlers = continuation.exception_handlers;
            pending_control = continuation.pending_control;
            stack.push(continuation.frame);
            match call
                .resume_input
                .take()
                .unwrap_or(GeneratorResumeInput::Value(Value::Null))
            {
                GeneratorResumeInput::Value(value) => {
                    if let Err(message) = stack
                        .current_mut()
                        .expect("resumed generator frame is active")
                        .registers
                        .set(continuation.yield_result, value)
                    {
                        return self.runtime_error(output, compiled, stack, message);
                    }
                }
                GeneratorResumeInput::Throw(value) => {
                    if let Some(target) = handle_throw(
                        compiled,
                        value.clone(),
                        &mut exception_handlers,
                        stack,
                        &mut pending_control,
                    ) {
                        block_id = target;
                        start_instruction_index = 0;
                    } else {
                        stack.pop();
                        return uncaught_exception(output, compiled, stack, value);
                    }
                }
            }
        } else {
            let args = match prepare_arguments(function, call.args, stack) {
                Ok(args) => args,
                Err(message) => {
                    return self.runtime_error(output, compiled, stack, message);
                }
            };
            if function.flags.is_generator && call.running_generator.is_none() {
                if function.returns_by_ref {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_RUNTIME_GENERATOR_BY_REF_YIELD_GAP: by-reference generator yields are not implemented in Prompt 35",
                    );
                }
                if args.iter().any(|arg| arg.reference.is_some()) {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_UNSUPPORTED_GENERATOR_BY_REF_ARG: generator by-reference arguments are not implemented in Prompt 33",
                    );
                }
                let generator_args = args.into_iter().map(|arg| arg.value).collect();
                return VmResult::success(
                    output.clone(),
                    Some(Value::Generator(GeneratorRef::new(
                        function_id.raw(),
                        generator_args,
                    ))),
                );
            }
            stack.push(Frame::new_with_class_context(
                function_id,
                function.register_count,
                function.local_count,
                call.scope_class.take(),
                call.called_class.take(),
                call.declaring_class.take(),
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
                bind_top_level_global_locals(function, stack, state);
            }
            for (param, mut arg) in function.params.iter().zip(args) {
                if let Err(message) = coerce_or_check_param_type(
                    compiled,
                    compiled.unit(),
                    function,
                    param,
                    &mut arg.value,
                    arg.reference.is_some(),
                ) {
                    let result = self.runtime_error(output, compiled, stack, message);
                    stack.pop();
                    return result;
                }
                let locals = &mut stack.current_mut().expect("frame was pushed").locals;
                let result = if param.by_ref {
                    let Some(reference) = arg.reference else {
                        unreachable!("prepare_arguments validates by-reference argument cells");
                    };
                    locals.bind_reference_cell(param.local, reference)
                } else {
                    locals.set(param.local, arg.value)
                };
                if let Err(message) = result {
                    let result = self.runtime_error(output, compiled, stack, message);
                    stack.pop();
                    return result;
                }
            }
            block_id = BlockId::new(0);
            foreach_iterators = HashMap::new();
            exception_handlers = Vec::new();
            pending_control = None;
        }

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
            let instruction_start = start_instruction_index;
            start_instruction_index = 0;

            for (instruction_index, instruction) in block
                .instructions
                .iter()
                .enumerate()
                .skip(instruction_start)
            {
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
                        let value = match self
                            .execute_binary(compiled, *op, &lhs, &rhs, output, stack, state)
                        {
                            Ok(value) => value,
                            Err(result) => {
                                return result;
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
                    InstructionKind::InstanceOf {
                        dst,
                        object,
                        class_name,
                    } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = match object_instanceof(compiled, &object, class_name) {
                            Ok(value) => Value::Bool(value),
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
                        let value =
                            match self.execute_cast(compiled, *kind, &src, output, stack, state) {
                                Ok(value) => value,
                                Err(result) => {
                                    return result;
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
                        let value = if is_globals_local(function, *local) {
                            Value::Array(state.globals.globals_array())
                        } else {
                            match stack
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
                        let value = if is_globals_local(function, *local) {
                            Value::Array(state.globals.globals_array())
                        } else {
                            match stack
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
                    InstructionKind::BindGlobal { local, name } => {
                        let cell = state.globals.ensure_slot(name.clone(), Value::Null);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .bind_reference_cell(*local, cell)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::BindReferenceDim {
                        local,
                        dims,
                        append,
                        source,
                    } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let cell = match stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .ensure_reference_cell(*source)
                        {
                            Ok(cell) => cell,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) =
                            bind_dim_local_to_reference_cell(stack, *local, &dims, *append, cell)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        self.record_lvalue_trace_event(
                            if *append {
                                "bind-reference-dim-append"
                            } else {
                                "bind-reference-dim"
                            },
                            *local,
                            &dims,
                        );
                    }
                    InstructionKind::BindReferenceFromDim {
                        target,
                        local,
                        dims,
                    } => {
                        let dims = match read_dim_operands(unit, stack, dims) {
                            Ok(dims) => dims,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let cell = match ensure_dim_reference_cell(stack, *local, &dims) {
                            Ok(cell) => cell,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .bind_reference_cell(*target, cell)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        self.record_lvalue_trace_event("bind-reference-from-dim", *local, &dims);
                    }
                    InstructionKind::InitStaticLocal {
                        local,
                        name,
                        default,
                    } => {
                        let key = (function_id.raw(), name.clone());
                        let cell = if let Some(cell) = state.static_locals.get(&key) {
                            cell.clone()
                        } else {
                            let value = match read_operand(unit, stack, *default) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            let cell = ReferenceCell::new(value);
                            state.static_locals.insert(key, cell.clone());
                            cell
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .locals
                            .bind_reference_cell(*local, cell)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::BindReferenceFromCall { target, name, args } => {
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let Some(callee) = compiled.lookup_function(name) else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_BY_REF_RETURN_NOT_CALLABLE: function {name} is not a user function"
                                ),
                            );
                        };
                        let result = self.execute_function(
                            compiled,
                            callee,
                            FunctionCall::new(values, Vec::new()),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        diagnostics.extend(result.diagnostics);
                        let Some(reference) = result.return_ref else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_BY_REF_RETURN_NOT_REFERENCEABLE: function {name} did not return a reference"
                                ),
                            );
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("caller frame is active")
                            .locals
                            .bind_reference_cell(*target, reference)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::EnterTry {
                        catch,
                        catch_types,
                        finally,
                        after,
                        exception_local,
                    } => {
                        exception_handlers.push(ExceptionHandler {
                            catch: *catch,
                            catch_types: catch_types.clone(),
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
                            if let Err(message) =
                                check_return_type(compiled, function, value.as_ref())
                            {
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
                                compiled,
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
                            compiled,
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
                    InstructionKind::MakeException {
                        dst,
                        class_name,
                        message,
                    } => {
                        let message = match read_operand(unit, stack, *message) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let object = match make_exception_object(class_name, &message) {
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
                        if is_fiber_runtime_class(class_name) {
                            let values = match read_call_args(unit, stack, args) {
                                Ok(values) => values,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            let fiber = match new_fiber_object(values) {
                                Ok(fiber) => fiber,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            if let Err(message) = stack
                                .current_mut()
                                .expect("frame was pushed")
                                .registers
                                .set(*dst, Value::Fiber(fiber))
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        }
                        if is_reflection_runtime_class(class_name) {
                            let values = match read_call_args(unit, stack, args) {
                                Ok(values) => values,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            let values =
                                values.into_iter().map(|arg| arg.value).collect::<Vec<_>>();
                            let object = match reflection_new_object(compiled, class_name, values) {
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
                            continue;
                        }
                        let class = match lookup_class_in_state(compiled, state, class_name) {
                            Some(class) => class,
                            None => {
                                if class_name
                                    .trim_start_matches('\\')
                                    .to_ascii_lowercase()
                                    .starts_with("reflection")
                                {
                                    let values = match read_call_args(unit, stack, args) {
                                        Ok(values) => values
                                            .into_iter()
                                            .map(|arg| arg.value)
                                            .collect::<Vec<_>>(),
                                        Err(message) => {
                                            return self
                                                .runtime_error(output, compiled, stack, message);
                                        }
                                    };
                                    let object =
                                        match reflection_new_object(compiled, class_name, values) {
                                            Ok(object) => object,
                                            Err(message) => {
                                                return self.runtime_error(
                                                    output, compiled, stack, message,
                                                );
                                            }
                                        };
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, Value::Object(object))
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                match self
                                    .autoload_class(compiled, class_name, output, stack, state)
                                {
                                    Ok(()) => {}
                                    Err(result) => return result,
                                }
                                if let Some(class) =
                                    lookup_class_in_state(compiled, state, class_name)
                                {
                                    class
                                } else {
                                    return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_CLASS: class {class_name} is not defined"
                                    ),
                                );
                                }
                            }
                        };
                        let runtime_class = match runtime_class_entry(compiled, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let object = ObjectRef::new(&runtime_class);
                        if let Some(constructor) = class.constructor {
                            let result = self.execute_function(
                                compiled,
                                constructor,
                                FunctionCall::new(values, Vec::new())
                                    .with_this(object.clone())
                                    .with_class_context(
                                        class.name.clone(),
                                        class.name.clone(),
                                        class.name.clone(),
                                    )
                                    .inherit_fiber_context(&running_fiber),
                                output,
                                stack,
                                state,
                            );
                            if !result.status.is_success() {
                                return result;
                            }
                            if result.fiber_suspension.is_some() {
                                return self.propagate_fiber_suspension(
                                    result,
                                    compiled,
                                    *dst,
                                    block_id,
                                    instruction_index + 1,
                                    &foreach_iterators,
                                    &exception_handlers,
                                    &pending_control,
                                    output,
                                    stack,
                                );
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
                        self.register_destructor_if_needed(compiled, &class, object.clone(), state);
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
                        let runtime_class = match runtime_class_entry(compiled, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let copy = match self
                            .clone_object_with_magic(compiled, object, &class, output, stack, state)
                        {
                            Ok(copy) => copy,
                            Err(result) => return result,
                        };
                        self.register_destructor_if_needed(compiled, &class, copy.clone(), state);
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Object(copy))
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
                        let runtime_class = match runtime_class_entry(compiled, &class) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(message) = validate_object_mvp(&runtime_class) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let copy = match self.clone_object_with_magic(
                            compiled,
                            object.clone(),
                            &class,
                            output,
                            stack,
                            state,
                        ) {
                            Ok(copy) => copy,
                            Err(result) => return result,
                        };
                        self.register_destructor_if_needed(compiled, &class, copy.clone(), state);
                        for (key, value) in replacements.iter() {
                            let property = match clone_with_property_name(key) {
                                Ok(property) => property,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            let Some(ir_property) =
                                class.properties.iter().find(|entry| entry.name == property)
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
                            if ir_property.flags.is_static
                                || ir_property.flags.is_private
                                || ir_property.flags.is_protected
                                || ir_property.flags.set_is_private
                                || ir_property.flags.set_is_protected
                                || ir_property.flags.is_readonly
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
                            let storage_name = property_storage_name(&class, ir_property);
                            let Some(entry) = runtime_class
                                .properties
                                .iter()
                                .find(|entry| entry.name == storage_name)
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
                            if let Err(message) = check_property_type(
                                compiled,
                                object.class_name().as_str(),
                                &property,
                                &entry.type_,
                                value,
                            ) {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            if let Some(function_id) = entry.hooks.set_function_id {
                                match self.call_property_hook(
                                    compiled,
                                    copy.clone(),
                                    &class,
                                    ir_property,
                                    FunctionId::new(function_id),
                                    vec![CallArgument::positional(value.clone())],
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(_) => continue,
                                    Err(result) => return result,
                                }
                            }
                            if !entry.hooks.backed
                                && (entry.hooks.get_function_id.is_some()
                                    || entry.hooks.set_function_id.is_some())
                            {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_VIRTUAL_PROPERTY_WRITE: property {}::${property} has no backing storage",
                                        object.class_name()
                                    ),
                                );
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
                        if internal_throwable_instanceof(&object.class_name(), "throwable")
                            .is_some()
                        {
                            let value = object.get_property(property).unwrap_or(Value::Null);
                            if let Err(message) = stack
                                .current_mut()
                                .expect("frame was pushed")
                                .registers
                                .set(*dst, value)
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        }
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
                        let scope = current_scope_class(compiled, stack);
                        let resolved = match lookup_property_in_hierarchy(
                            compiled,
                            class,
                            property,
                            scope.as_deref(),
                        ) {
                            Ok(Some(resolved)) => resolved,
                            Ok(None) => {
                                if let Some(value) = object.get_property(property) {
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, value)
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                match self.call_magic_property_method(
                                    compiled,
                                    object.clone(),
                                    "__get",
                                    property,
                                    vec![CallArgument::positional(Value::String(
                                        PhpString::from_test_str(property),
                                    ))],
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(value)) => {
                                        if let Err(message) = stack
                                            .current_mut()
                                            .expect("frame was pushed")
                                            .registers
                                            .set(*dst, value)
                                        {
                                            return self
                                                .runtime_error(output, compiled, stack, message);
                                        }
                                        continue;
                                    }
                                    Ok(None) => {}
                                    Err(result) => return result,
                                }
                                diagnostics.push(RuntimeDiagnostic::new(
                                    "E_PHP_VM_UNDEFINED_PROPERTY",
                                    RuntimeSeverity::Warning,
                                    format!(
                                        "E_PHP_VM_UNDEFINED_PROPERTY: property {}::${property} is not declared",
                                        object.class_name()
                                    ),
                                    RuntimeSourceSpan::default(),
                                    stack_trace(compiled, stack),
                                    None,
                                ));
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("frame was pushed")
                                    .registers
                                    .set(*dst, Value::Null)
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                continue;
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if let Err(_message) = validate_property_access(
                            compiled,
                            stack,
                            resolved.class,
                            resolved.property,
                        ) {
                            match self.call_magic_property_method(
                                compiled,
                                object.clone(),
                                "__get",
                                property,
                                vec![CallArgument::positional(Value::String(
                                    PhpString::from_test_str(property),
                                ))],
                                output,
                                stack,
                                state,
                            ) {
                                Ok(Some(value)) => {
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, value)
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                Ok(None) => {
                                    return self.runtime_error(output, compiled, stack, _message);
                                }
                                Err(result) => return result,
                            }
                        }
                        if !property_hook_is_active(
                            state,
                            &object,
                            resolved.class,
                            resolved.property,
                        ) && let Some(function) = resolved.property.hooks.get
                        {
                            match self.call_property_hook(
                                compiled,
                                object.clone(),
                                resolved.class,
                                resolved.property,
                                function,
                                Vec::new(),
                                output,
                                stack,
                                state,
                            ) {
                                Ok(value) => {
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, value)
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                Err(result) => return result,
                            }
                        }
                        let storage_name = property_storage_name(resolved.class, resolved.property);
                        let value = match object.get_property(&storage_name) {
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
                        if matches!(value, Value::Uninitialized) {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_UNINITIALIZED_PROPERTY: typed property {}::${property} must not be accessed before initialization",
                                    object.class_name()
                                ),
                            );
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
                    InstructionKind::FetchStaticProperty {
                        dst,
                        class_name,
                        property,
                    } => {
                        let class = match resolve_static_class_name(compiled, stack, class_name) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let scope = current_scope_class(compiled, stack);
                        let resolved = match lookup_property_in_hierarchy(
                            compiled,
                            class,
                            property,
                            scope.as_deref(),
                        ) {
                            Ok(Some(resolved)) => resolved,
                            Ok(None) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_STATIC_PROPERTY: property {}::${property} is not declared",
                                        class.name
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if !resolved.property.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_NON_STATIC_PROPERTY_ACCESS: property {}::${} is not static",
                                    resolved.class.name, resolved.property.name
                                ),
                            );
                        }
                        if let Err(message) = validate_property_access(
                            compiled,
                            stack,
                            resolved.class,
                            resolved.property,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let key = static_property_key(resolved.class, resolved.property);
                        if !state.static_properties.contains_key(&key) {
                            let default = match static_property_default(
                                unit,
                                resolved.class,
                                resolved.property,
                            ) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            state.static_properties.insert(key.clone(), default);
                        }
                        let value = state
                            .static_properties
                            .get(&key)
                            .cloned()
                            .unwrap_or(Value::Uninitialized);
                        if matches!(value, Value::Uninitialized) {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_UNINITIALIZED_STATIC_PROPERTY: typed static property {}::${} must not be accessed before initialization",
                                    resolved.class.name, resolved.property.name
                                ),
                            );
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
                    InstructionKind::FetchClassConstant {
                        dst,
                        class_name,
                        constant,
                    } => {
                        let class = match resolve_static_class_name(compiled, stack, class_name) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = if constant.eq_ignore_ascii_case("class") {
                            Value::String(PhpString::from_test_str(&class.display_name))
                        } else {
                            if class.flags.is_enum
                                && let Some(case) = class
                                    .enum_cases
                                    .iter()
                                    .find(|case| case.name.eq_ignore_ascii_case(constant))
                            {
                                let object = match enum_case_object(compiled, state, class, case) {
                                    Ok(object) => object,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
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
                                continue;
                            }
                            let scope = current_scope_class(compiled, stack);
                            let resolved = match lookup_constant_in_hierarchy(
                                compiled,
                                class,
                                constant,
                                scope.as_deref(),
                            ) {
                                Ok(Some(resolved)) => resolved,
                                Ok(None) => {
                                    return self.runtime_error(
                                        output,
                                        compiled,
                                        stack,
                                        format!(
                                            "E_PHP_VM_UNKNOWN_CLASS_CONSTANT: constant {}::{constant} is not declared",
                                            class.name
                                        ),
                                    );
                                }
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            if let Err(message) = validate_constant_access(
                                compiled,
                                stack,
                                resolved.class,
                                resolved.constant,
                            ) {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            match resolved.constant.value {
                                Some(value) => match constant_value(unit, value) {
                                    Ok(value) => value,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                },
                                None => Value::Null,
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
                    InstructionKind::IssetProperty {
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
                                        "E_PHP_VM_PROPERTY_FETCH_NON_OBJECT: cannot test property {property} on {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let value = property_state_value(compiled, stack, &object, property);
                        let result = if let Some(value) = value {
                            !matches!(value, Value::Uninitialized | Value::Null)
                        } else {
                            match self.call_magic_property_method(
                                compiled,
                                object.clone(),
                                "__isset",
                                property,
                                vec![CallArgument::positional(Value::String(
                                    PhpString::from_test_str(property),
                                ))],
                                output,
                                stack,
                                state,
                            ) {
                                Ok(Some(value)) => match to_bool(&value) {
                                    Ok(value) => value,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                },
                                Ok(None) => false,
                                Err(result) => return result,
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
                    InstructionKind::EmptyProperty {
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
                                        "E_PHP_VM_PROPERTY_FETCH_NON_OBJECT: cannot test property {property} on {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = match property_state_value(compiled, stack, &object, property)
                        {
                            Some(value) => match php_empty(&value) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            },
                            None => {
                                let isset = match self.call_magic_property_method(
                                    compiled,
                                    object.clone(),
                                    "__isset",
                                    property,
                                    vec![CallArgument::positional(Value::String(
                                        PhpString::from_test_str(property),
                                    ))],
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(value)) => match to_bool(&value) {
                                        Ok(value) => value,
                                        Err(message) => {
                                            return self
                                                .runtime_error(output, compiled, stack, message);
                                        }
                                    },
                                    Ok(None) => false,
                                    Err(result) => return result,
                                };
                                if !isset {
                                    true
                                } else {
                                    match self.call_magic_property_method(
                                        compiled,
                                        object.clone(),
                                        "__get",
                                        property,
                                        vec![CallArgument::positional(Value::String(
                                            PhpString::from_test_str(property),
                                        ))],
                                        output,
                                        stack,
                                        state,
                                    ) {
                                        Ok(Some(value)) => match php_empty(&value) {
                                            Ok(value) => value,
                                            Err(message) => {
                                                return self.runtime_error(
                                                    output, compiled, stack, message,
                                                );
                                            }
                                        },
                                        Ok(None) => true,
                                        Err(result) => return result,
                                    }
                                }
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
                    InstructionKind::UnsetProperty { object, property } => {
                        let object = match read_operand(unit, stack, *object) {
                            Ok(Value::Object(object)) => object,
                            Ok(other) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_PROPERTY_FETCH_NON_OBJECT: cannot unset property {property} on {}",
                                        value_type_name(&other)
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let class = compiled.lookup_class(&object.class_name());
                        let scope = current_scope_class(compiled, stack);
                        let declared = match class {
                            Some(class) => match lookup_property_in_hierarchy(
                                compiled,
                                class,
                                property,
                                scope.as_deref(),
                            ) {
                                Ok(property) => property,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            },
                            None => None,
                        };
                        if let Some(resolved) = declared {
                            if validate_property_access(
                                compiled,
                                stack,
                                resolved.class,
                                resolved.property,
                            )
                            .is_err()
                            {
                                match self.call_magic_property_method(
                                    compiled,
                                    object.clone(),
                                    "__unset",
                                    property,
                                    vec![CallArgument::positional(Value::String(
                                        PhpString::from_test_str(property),
                                    ))],
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(_)) | Ok(None) => {}
                                    Err(result) => return result,
                                }
                                continue;
                            }
                            let storage_name =
                                property_storage_name(resolved.class, resolved.property);
                            object.set_property(storage_name, Value::Uninitialized);
                        } else {
                            match self.call_magic_property_method(
                                compiled,
                                object.clone(),
                                "__unset",
                                property,
                                vec![CallArgument::positional(Value::String(
                                    PhpString::from_test_str(property),
                                ))],
                                output,
                                stack,
                                state,
                            ) {
                                Ok(Some(_)) | Ok(None) => {
                                    object.unset_property(property);
                                }
                                Err(result) => return result,
                            }
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
                        let scope = current_scope_class(compiled, stack);
                        let resolved = match lookup_property_in_hierarchy(
                            compiled,
                            class,
                            property,
                            scope.as_deref(),
                        ) {
                            Ok(Some(resolved)) => resolved,
                            Ok(None) => {
                                let value = match read_operand(unit, stack, *value) {
                                    Ok(value) => value,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                };
                                match self.call_magic_property_method(
                                    compiled,
                                    object.clone(),
                                    "__set",
                                    property,
                                    vec![
                                        CallArgument::positional(Value::String(
                                            PhpString::from_test_str(property),
                                        )),
                                        CallArgument::positional(value.clone()),
                                    ],
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(_)) => {
                                        if let Err(message) = stack
                                            .current_mut()
                                            .expect("frame was pushed")
                                            .registers
                                            .set(*dst, value)
                                        {
                                            return self
                                                .runtime_error(output, compiled, stack, message);
                                        }
                                        continue;
                                    }
                                    Ok(None) => {}
                                    Err(result) => return result,
                                }
                                diagnostics.push(RuntimeDiagnostic::new(
                                    "E_PHP_VM_DYNAMIC_PROPERTY_DEPRECATED",
                                    RuntimeSeverity::Deprecation,
                                    format!(
                                        "E_PHP_VM_DYNAMIC_PROPERTY_DEPRECATED: creating dynamic property {}::${property}",
                                        object.class_name()
                                    ),
                                    RuntimeSourceSpan::default(),
                                    stack_trace(compiled, stack),
                                    None,
                                ));
                                object.set_property(property, value.clone());
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("frame was pushed")
                                    .registers
                                    .set(*dst, value)
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                continue;
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let entry = resolved.property;
                        if let Err(message) =
                            validate_property_access(compiled, stack, resolved.class, entry)
                                .and_then(|()| {
                                    validate_property_set_access(
                                        compiled,
                                        stack,
                                        resolved.class,
                                        entry,
                                    )
                                })
                        {
                            let value = match read_operand(unit, stack, *value) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            match self.call_magic_property_method(
                                compiled,
                                object.clone(),
                                "__set",
                                property,
                                vec![
                                    CallArgument::positional(Value::String(
                                        PhpString::from_test_str(property),
                                    )),
                                    CallArgument::positional(value.clone()),
                                ],
                                output,
                                stack,
                                state,
                            ) {
                                Ok(Some(_)) => {
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, value)
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                Ok(None) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                Err(result) => return result,
                            }
                        }
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let property_type = ir_runtime_type(entry.type_.as_ref());
                        if let Err(message) = check_property_type(
                            compiled,
                            object.class_name().as_str(),
                            property,
                            &property_type,
                            &value,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) =
                            validate_property_write(resolved.class, entry, &object, stack, compiled)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if !property_hook_is_active(state, &object, resolved.class, entry)
                            && let Some(function) = entry.hooks.set
                        {
                            match self.call_property_hook(
                                compiled,
                                object.clone(),
                                resolved.class,
                                entry,
                                function,
                                vec![CallArgument::positional(value.clone())],
                                output,
                                stack,
                                state,
                            ) {
                                Ok(_) => {
                                    if let Err(message) = stack
                                        .current_mut()
                                        .expect("frame was pushed")
                                        .registers
                                        .set(*dst, value)
                                    {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    continue;
                                }
                                Err(result) => return result,
                            }
                        }
                        if !entry.hooks.backed
                            && (entry.hooks.get.is_some() || entry.hooks.set.is_some())
                        {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_VIRTUAL_PROPERTY_WRITE: property {}::${} has no backing storage",
                                    resolved.class.name, entry.name
                                ),
                            );
                        }
                        let storage_name = property_storage_name(resolved.class, entry);
                        object.set_property(storage_name, value.clone());
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, value)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::AssignStaticProperty {
                        dst,
                        class_name,
                        property,
                        value,
                    } => {
                        let class = match resolve_static_class_name(compiled, stack, class_name) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let scope = current_scope_class(compiled, stack);
                        let resolved = match lookup_property_in_hierarchy(
                            compiled,
                            class,
                            property,
                            scope.as_deref(),
                        ) {
                            Ok(Some(resolved)) => resolved,
                            Ok(None) => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_UNKNOWN_STATIC_PROPERTY: property {}::${property} is not declared",
                                        class.name
                                    ),
                                );
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if !resolved.property.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_NON_STATIC_PROPERTY_ACCESS: property {}::${} is not static",
                                    resolved.class.name, resolved.property.name
                                ),
                            );
                        }
                        if let Err(message) = validate_property_access(
                            compiled,
                            stack,
                            resolved.class,
                            resolved.property,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let value = match read_operand(unit, stack, *value) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let property_type = ir_runtime_type(resolved.property.type_.as_ref());
                        if let Err(message) = check_property_type(
                            compiled,
                            resolved.class.name.as_str(),
                            resolved.property.name.as_str(),
                            &property_type,
                            &value,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let key = static_property_key(resolved.class, resolved.property);
                        let current = if let Some(value) = state.static_properties.get(&key) {
                            value.clone()
                        } else {
                            match static_property_default(unit, resolved.class, resolved.property) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            }
                        };
                        if let Err(message) = validate_static_property_write(
                            compiled,
                            stack,
                            resolved.class,
                            resolved.property,
                            &current,
                        ) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        state.static_properties.insert(key, value.clone());
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
                        let result = if is_globals_local(function, *local) {
                            assign_globals_dim(&mut state.globals, &dims, value.clone(), false)
                        } else {
                            assign_dim_local(stack, *local, &dims, value.clone(), false)
                        };
                        if let Err(message) = result {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        self.record_lvalue_trace_event("array-write-dim", *local, &dims);
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
                        let result = if is_globals_local(function, *local) {
                            assign_globals_dim(&mut state.globals, &dims, value.clone(), true)
                        } else {
                            assign_dim_local(stack, *local, &dims, value.clone(), true)
                        };
                        if let Err(message) = result {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        self.record_lvalue_trace_event("array-append-dim", *local, &dims);
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
                            .unset(*local)
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
                        let value = read_local_value(stack, *local)
                            .and_then(|value| fetch_dim_path_value(&value, &dims).ok().flatten());
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
                            .and_then(|value| fetch_dim_path_value(&value, &dims).ok().flatten())
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
                        self.record_lvalue_trace_event("array-unset-dim", *local, &dims);
                    }
                    InstructionKind::ForeachInit { iterator, source } => {
                        let source = match read_operand(unit, stack, *source) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let foreach_iterator = match self
                            .foreach_iterator_from_value(compiled, source, output, stack, state)
                        {
                            Ok(iterator) => iterator,
                            Err(result) => return result,
                        };
                        self.record_runtime_trace_event(format!(
                            "foreach init iterator=r{} kind={}",
                            iterator.raw(),
                            format_foreach_iterator_kind(&foreach_iterator)
                        ));
                        foreach_iterators.insert(*iterator, foreach_iterator);
                    }
                    InstructionKind::ForeachNext {
                        has_value,
                        iterator,
                        key,
                        value,
                    } => {
                        let next_value = match foreach_iterators.get(iterator).cloned() {
                            Some(ForeachIterator::Snapshot { entries, position }) => {
                                let next = entries
                                    .get(position)
                                    .cloned()
                                    .map(|(key, value)| (Some(array_key_to_value(key)), value));
                                if next.is_some()
                                    && let Some(ForeachIterator::Snapshot { position, .. }) =
                                        foreach_iterators.get_mut(iterator)
                                {
                                    *position += 1;
                                }
                                next
                            }
                            Some(ForeachIterator::ObjectProperties { object, position }) => {
                                let keys = match object_property_iteration_keys(compiled, &object) {
                                    Ok(keys) => keys,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                };
                                let next = keys.get(position).and_then(|name| {
                                    object.get_property(name).map(|value| {
                                        (
                                            Some(Value::string(name.as_bytes().to_vec())),
                                            effective_value(&value),
                                        )
                                    })
                                });
                                if next.is_some()
                                    && let Some(ForeachIterator::ObjectProperties {
                                        position, ..
                                    }) = foreach_iterators.get_mut(iterator)
                                {
                                    *position += 1;
                                }
                                next
                            }
                            Some(ForeachIterator::IteratorObject { object, needs_next }) => {
                                if needs_next
                                    && let Err(result) = self.call_object_method_value(
                                        compiled,
                                        object.clone(),
                                        "next",
                                        output,
                                        stack,
                                        state,
                                    )
                                {
                                    return result;
                                }
                                let valid = match self.call_object_method_value(
                                    compiled,
                                    object.clone(),
                                    "valid",
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(value) => match to_bool(&value) {
                                        Ok(value) => value,
                                        Err(message) => {
                                            return self
                                                .runtime_error(output, compiled, stack, message);
                                        }
                                    },
                                    Err(result) => return result,
                                };
                                if !valid {
                                    None
                                } else {
                                    let entry_value = match self.call_object_method_value(
                                        compiled,
                                        object.clone(),
                                        "current",
                                        output,
                                        stack,
                                        state,
                                    ) {
                                        Ok(value) => value,
                                        Err(result) => return result,
                                    };
                                    let entry_key = if key.is_some() {
                                        match self.call_object_method_value(
                                            compiled,
                                            object.clone(),
                                            "key",
                                            output,
                                            stack,
                                            state,
                                        ) {
                                            Ok(value) => Some(value),
                                            Err(result) => return result,
                                        }
                                    } else {
                                        None
                                    };
                                    if let Some(ForeachIterator::IteratorObject {
                                        needs_next,
                                        ..
                                    }) = foreach_iterators.get_mut(iterator)
                                    {
                                        *needs_next = true;
                                    }
                                    Some((entry_key, entry_value))
                                }
                            }
                            Some(ForeachIterator::Generator {
                                generator,
                                consumed,
                            }) => {
                                if consumed {
                                    match self.resume_generator_to_next_yield(
                                        compiled,
                                        generator,
                                        GeneratorResumeInput::Value(Value::Null),
                                        output,
                                        stack,
                                        state,
                                    ) {
                                        Ok(next) => next,
                                        Err(result) => return result,
                                    }
                                } else {
                                    if let Some(ForeachIterator::Generator { consumed, .. }) =
                                        foreach_iterators.get_mut(iterator)
                                    {
                                        *consumed = true;
                                    }
                                    match self.advance_generator_to_first_yield(
                                        compiled, generator, output, stack, state,
                                    ) {
                                        Ok(next) => next,
                                        Err(result) => return result,
                                    }
                                }
                            }
                            Some(ForeachIterator::ByReference { .. }) | None => {
                                return self.runtime_error(
                                    output,
                                    compiled,
                                    stack,
                                    format!(
                                        "E_PHP_VM_FOREACH_ITERATOR_MISSING: iterator r{} is not initialized",
                                        iterator.raw()
                                    ),
                                );
                            }
                        };
                        let Some((entry_key, entry_value)) = next_value else {
                            self.record_runtime_trace_event(format!(
                                "foreach next iterator=r{} status=done",
                                iterator.raw()
                            ));
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
                        self.record_runtime_trace_event(format!(
                            "foreach next iterator=r{} status=value key={} value={}",
                            iterator.raw(),
                            entry_key
                                .as_ref()
                                .map(trace_value)
                                .unwrap_or_else(|| "None".to_owned()),
                            trace_value(&entry_value)
                        ));
                        let frame = stack.current_mut().expect("frame was pushed");
                        if let Err(message) = frame.registers.set(*has_value, Value::Bool(true)) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Some(key) = key
                            && let Err(message) = frame
                                .registers
                                .set(*key, entry_key.unwrap_or(Value::Int(0)))
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Err(message) = frame.registers.set(*value, entry_value) {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                    }
                    InstructionKind::ForeachInitRef { iterator, local } => {
                        let source = read_local_value(stack, *local).unwrap_or(Value::Null);
                        let Value::Array(_) = effective_value(&source) else {
                            let diagnostic = unsupported_feature(
                                "E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE",
                                format!(
                                    "foreach by reference over {} is not implemented; Phase 5 supports local arrays only",
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
                                yielded: None,
                                fiber_suspension: None,
                                return_ref: None,
                                trace: Vec::new(),
                            };
                        };
                        foreach_iterators.insert(
                            *iterator,
                            ForeachIterator::ByReference {
                                local: *local,
                                position: 0,
                            },
                        );
                        self.record_runtime_trace_event(format!(
                            "foreach init-ref iterator=r{} local={}",
                            iterator.raw(),
                            local.raw()
                        ));
                    }
                    InstructionKind::ForeachNextRef {
                        has_value,
                        iterator,
                        key,
                        value_local,
                    } => {
                        let Some(ForeachIterator::ByReference { local, position }) =
                            foreach_iterators.get(iterator).cloned()
                        else {
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
                        let keys = match foreach_array_keys_from_local(stack, local) {
                            Ok(keys) => keys,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let Some(entry_key) = keys.get(position).cloned() else {
                            self.record_runtime_trace_event(format!(
                                "foreach next-ref iterator=r{} status=done",
                                iterator.raw()
                            ));
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
                        let Some(ForeachIterator::ByReference { position, .. }) =
                            foreach_iterators.get_mut(iterator)
                        else {
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
                        *position += 1;
                        self.record_runtime_trace_event(format!(
                            "foreach next-ref iterator=r{} status=value key={}",
                            iterator.raw(),
                            format_array_key_for_trace(&entry_key)
                        ));
                        let cell = match ensure_dim_reference_cell(
                            stack,
                            local,
                            std::slice::from_ref(&entry_key),
                        ) {
                            Ok(cell) => cell,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
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
                        if let Err(message) = frame.locals.bind_reference_cell(*value_local, cell) {
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
                        if let Err(result) = self.write_echo(compiled, output, stack, state, &value)
                        {
                            return result;
                        }
                    }
                    InstructionKind::Yield { dst, key, value } => {
                        let key = match key {
                            Some(key) => match read_operand(unit, stack, *key) {
                                Ok(value) => Some(value),
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            },
                            None => None,
                        };
                        let value = match value {
                            Some(value) => match read_operand(unit, stack, *value) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            },
                            None => Value::Null,
                        };
                        if let Err(message) = stack
                            .current_mut()
                            .expect("frame was pushed")
                            .registers
                            .set(*dst, Value::Null)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        if let Some(shared) = call.shared_top_level_locals.as_deref_mut() {
                            export_shared_locals(function, stack, shared);
                        }
                        let Some(frame) = stack.pop() else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                "E_PHP_VM_GENERATOR_FRAME_MISSING: generator frame missing at yield",
                            );
                        };
                        if let Some(generator) = call.running_generator.as_ref() {
                            state.generator_continuations.insert(
                                generator.id(),
                                GeneratorContinuation {
                                    frame,
                                    block_id,
                                    instruction_index: instruction_index + 1,
                                    yield_result: *dst,
                                    foreach_iterators: foreach_iterators.clone(),
                                    exception_handlers: exception_handlers.clone(),
                                    pending_control: pending_control.clone(),
                                },
                            );
                        }
                        let mut result =
                            VmResult::success_with_diagnostics(output.clone(), None, diagnostics);
                        result.yielded = Some(GeneratorYield { key, value });
                        return result;
                    }
                    InstructionKind::YieldFrom { dst, source } => {
                        let Some(owner) = call.running_generator.as_ref() else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                "E_PHP_VM_YIELD_FROM_OUTSIDE_GENERATOR: yield from executed outside a generator",
                            );
                        };
                        let delegation_key = YieldFromKey {
                            generator_id: owner.id(),
                            block_id,
                            instruction_index,
                        };
                        let step = match self.advance_yield_from_delegation(
                            compiled,
                            delegation_key.clone(),
                            *source,
                            output,
                            stack,
                            state,
                        ) {
                            Ok(step) => step,
                            Err(result) => return result,
                        };
                        match step {
                            YieldFromStep::Yield { key, value } => {
                                let Some(frame) = stack.pop() else {
                                    return self.runtime_error(
                                        output,
                                        compiled,
                                        stack,
                                        "E_PHP_VM_GENERATOR_FRAME_MISSING: generator frame missing at yield from",
                                    );
                                };
                                state.generator_continuations.insert(
                                    owner.id(),
                                    GeneratorContinuation {
                                        frame,
                                        block_id,
                                        instruction_index,
                                        yield_result: *dst,
                                        foreach_iterators: foreach_iterators.clone(),
                                        exception_handlers: exception_handlers.clone(),
                                        pending_control: pending_control.clone(),
                                    },
                                );
                                let mut result = VmResult::success_with_diagnostics(
                                    output.clone(),
                                    None,
                                    diagnostics,
                                );
                                result.yielded = Some(GeneratorYield { key, value });
                                return result;
                            }
                            YieldFromStep::Complete(return_value) => {
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("frame was pushed")
                                    .registers
                                    .set(*dst, return_value)
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            }
                        }
                    }
                    InstructionKind::CallFunction { dst, name, args } => {
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = if is_autoload_builtin_name(name)
                            || is_class_probe_builtin_name(name)
                        {
                            self.call_autoload_builtin(compiled, name, values, output, stack, state)
                        } else if let Some(callee) = compiled.lookup_function(name) {
                            self.execute_function(
                                compiled,
                                callee,
                                FunctionCall::new(values, Vec::new())
                                    .inherit_fiber_context(&running_fiber),
                                output,
                                stack,
                                state,
                            )
                        } else if BuiltinRegistry::new().contains(name) {
                            let values = match call_args_to_positional(name, values) {
                                Ok(values) => values,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            if name == "var_dump"
                                && let Some(message) = debug_info_gap_message(compiled, &values)
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
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
                        if result.fiber_suspension.is_some() {
                            return self.propagate_fiber_suspension(
                                result,
                                compiled,
                                *dst,
                                block_id,
                                instruction_index + 1,
                                &foreach_iterators,
                                &exception_handlers,
                                &pending_control,
                                output,
                                stack,
                            );
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
                        let receiver = match read_operand(unit, stack, *object) {
                            Ok(receiver) => receiver,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let object = match receiver {
                            Value::Fiber(fiber) => {
                                let value = match self.call_fiber_method(
                                    compiled, fiber, method, values, output, stack, state,
                                ) {
                                    Ok(value) => value,
                                    Err(result) => return result,
                                };
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("caller frame is active")
                                    .registers
                                    .set(*dst, value)
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                continue;
                            }
                            Value::Generator(generator) => {
                                let value = match self.call_generator_method(
                                    compiled, generator, method, values, output, stack, state,
                                ) {
                                    Ok(value) => value,
                                    Err(result) => return result,
                                };
                                if let Err(message) = stack
                                    .current_mut()
                                    .expect("caller frame is active")
                                    .registers
                                    .set(*dst, value)
                                {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                                continue;
                            }
                            Value::Object(object) => object,
                            other => {
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
                        };
                        if internal_throwable_instanceof(&object.class_name(), "throwable")
                            .is_some()
                        {
                            let value = match internal_throwable_method_value(
                                &object,
                                method,
                                values.into_iter().map(|arg| arg.value).collect(),
                            ) {
                                Ok(value) => value,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            if let Err(message) = stack
                                .current_mut()
                                .expect("caller frame is active")
                                .registers
                                .set(*dst, value)
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        }
                        if is_reflection_runtime_class(&object.class_name()) {
                            let values =
                                values.into_iter().map(|arg| arg.value).collect::<Vec<_>>();
                            let value =
                                match reflection_method_value(compiled, &object, method, values) {
                                    Ok(value) => value,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                };
                            if let Err(message) = stack
                                .current_mut()
                                .expect("caller frame is active")
                                .registers
                                .set(*dst, value)
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        }
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
                        let scope = current_scope_class(compiled, stack);
                        let resolved = match lookup_method_in_hierarchy(
                            compiled,
                            class,
                            method,
                            scope.as_deref(),
                        ) {
                            Ok(Some(method)) => method,
                            Ok(None) => {
                                let result = match self.call_magic_instance_method(
                                    compiled,
                                    object.clone(),
                                    "__call",
                                    method,
                                    values,
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(result)) => result,
                                    Ok(None) => {
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
                                    Err(result) => return result,
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
                                continue;
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let method_entry = resolved.method;
                        let declaring_class = resolved.class;
                        if method_entry.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_STATIC_METHOD_AS_INSTANCE: method {}::{} is static",
                                    declaring_class.name, method_entry.name
                                ),
                            );
                        }
                        if let Err(message) =
                            validate_method_callable(compiled, stack, declaring_class, method_entry)
                        {
                            if method_entry.flags.is_private || method_entry.flags.is_protected {
                                let result = match self.call_magic_instance_method(
                                    compiled,
                                    object.clone(),
                                    "__call",
                                    method,
                                    values,
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(result)) => result,
                                    Ok(None) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    Err(result) => return result,
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
                                continue;
                            }
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let result = self.execute_function(
                            compiled,
                            method_entry.function,
                            FunctionCall::new(values, Vec::new())
                                .with_this(object.clone())
                                .with_class_context(
                                    declaring_class.name.clone(),
                                    object.class_name(),
                                    declaring_class.name.clone(),
                                )
                                .inherit_fiber_context(&running_fiber),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        if result.fiber_suspension.is_some() {
                            return self.propagate_fiber_suspension(
                                result,
                                compiled,
                                *dst,
                                block_id,
                                instruction_index + 1,
                                &foreach_iterators,
                                &exception_handlers,
                                &pending_control,
                                output,
                                stack,
                            );
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
                        if is_fiber_runtime_class(class_name)
                            && normalize_method_name(method) == "suspend"
                        {
                            let values = match read_call_args(unit, stack, args) {
                                Ok(values) => values,
                                Err(message) => {
                                    return self.runtime_error(output, compiled, stack, message);
                                }
                            };
                            return match self.suspend_current_fiber(
                                compiled,
                                &running_fiber,
                                values,
                                *dst,
                                block_id,
                                instruction_index + 1,
                                &foreach_iterators,
                                &exception_handlers,
                                &pending_control,
                                output,
                                stack,
                            ) {
                                Ok(result) => result,
                                Err(result) => result,
                            };
                        }
                        let class = match resolve_static_class_name(compiled, stack, class_name) {
                            Ok(class) => class,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let scope =
                            method_lookup_scope_for_static_call(compiled, stack, class_name);
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        if class.flags.is_enum
                            && matches!(
                                normalize_method_name(method).as_str(),
                                "cases" | "from" | "tryfrom"
                            )
                        {
                            let value =
                                match enum_static_method(compiled, state, class, method, values) {
                                    Ok(value) => value,
                                    Err(message) => {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                };
                            if let Err(message) = stack
                                .current_mut()
                                .expect("caller frame is active")
                                .registers
                                .set(*dst, value)
                            {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                            continue;
                        }
                        let resolved = match lookup_method_in_hierarchy(
                            compiled,
                            class,
                            method,
                            scope.as_deref(),
                        ) {
                            Ok(Some(method)) => method,
                            Ok(None) => {
                                let called_class = called_class_for_static_call(
                                    compiled, stack, class_name, class,
                                );
                                let result = match self.call_magic_static_method(
                                    compiled,
                                    class,
                                    "__callStatic",
                                    method,
                                    values,
                                    called_class,
                                    output,
                                    stack,
                                    state,
                                ) {
                                    Ok(Some(result)) => result,
                                    Ok(None) => {
                                        return self.runtime_error(
                                            output,
                                            compiled,
                                            stack,
                                            format!(
                                                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                                                class.name, method
                                            ),
                                        );
                                    }
                                    Err(result) => return result,
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
                                continue;
                            }
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let method_entry = resolved.method;
                        let declaring_class = resolved.class;
                        if !method_entry.flags.is_static {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_NON_STATIC_METHOD_CALL: method {}::{} is not static",
                                    declaring_class.name, method_entry.name
                                ),
                            );
                        }
                        if method_entry.flags.is_private || method_entry.flags.is_protected {
                            let called_class =
                                called_class_for_static_call(compiled, stack, class_name, class);
                            let result = match self.call_magic_static_method(
                                compiled,
                                class,
                                "__callStatic",
                                method,
                                values,
                                called_class,
                                output,
                                stack,
                                state,
                            ) {
                                Ok(Some(result)) => result,
                                Ok(None) => {
                                    if let Err(message) = validate_method_callable(
                                        compiled,
                                        stack,
                                        declaring_class,
                                        method_entry,
                                    ) {
                                        return self
                                            .runtime_error(output, compiled, stack, message);
                                    }
                                    unreachable!("inaccessible method should fail validation");
                                }
                                Err(result) => return result,
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
                            continue;
                        }
                        if let Err(message) =
                            validate_method_callable(compiled, stack, declaring_class, method_entry)
                        {
                            return self.runtime_error(output, compiled, stack, message);
                        }
                        let called_class =
                            called_class_for_static_call(compiled, stack, class_name, class);
                        let result = self.execute_function(
                            compiled,
                            method_entry.function,
                            FunctionCall::new(values, Vec::new())
                                .with_class_context(
                                    declaring_class.name.clone(),
                                    called_class,
                                    declaring_class.name.clone(),
                                )
                                .inherit_fiber_context(&running_fiber),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        if result.fiber_suspension.is_some() {
                            return self.propagate_fiber_suspension(
                                result,
                                compiled,
                                *dst,
                                block_id,
                                instruction_index + 1,
                                &foreach_iterators,
                                &exception_handlers,
                                &pending_control,
                                output,
                                stack,
                            );
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
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = self.execute_function(
                            compiled,
                            FunctionId::new(function),
                            FunctionCall::new(values, captures.clone())
                                .inherit_fiber_context(&running_fiber),
                            output,
                            stack,
                            state,
                        );
                        if !result.status.is_success() {
                            return result;
                        }
                        if result.fiber_suspension.is_some() {
                            return self.propagate_fiber_suspension(
                                result,
                                compiled,
                                *dst,
                                block_id,
                                instruction_index + 1,
                                &foreach_iterators,
                                &exception_handlers,
                                &pending_control,
                                output,
                                stack,
                            );
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
                        let values = match read_call_args(unit, stack, args) {
                            Ok(values) => values,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
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
                            vec![CallArgument::positional(input)],
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
                    InstructionKind::Eval { dst, code } => {
                        let code = match read_operand(unit, stack, *code) {
                            Ok(value) => value,
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        };
                        let result = self.execute_eval(compiled, &code, output, stack, state);
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
                            yielded: None,
                            fiber_suspension: None,
                            return_ref: None,
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
                TerminatorKind::Return {
                    value,
                    by_ref_local,
                } => {
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
                    if let Err(message) = check_return_type(compiled, function, value.as_ref()) {
                        return self.runtime_error(output, compiled, stack, message);
                    }
                    let return_ref = if function.returns_by_ref {
                        let Some(local) = by_ref_local else {
                            return self.runtime_error(
                                output,
                                compiled,
                                stack,
                                format!(
                                    "E_PHP_VM_BY_REF_RETURN_TEMPORARY: function {} must return a variable by reference",
                                    function.name
                                ),
                            );
                        };
                        let frame = stack.current_mut().expect("frame is active");
                        match frame.locals.ensure_reference_cell(*local) {
                            Ok(reference) => Some(reference),
                            Err(message) => {
                                return self.runtime_error(output, compiled, stack, message);
                            }
                        }
                    } else {
                        None
                    };
                    if let Some(shared) = call.shared_top_level_locals.as_deref_mut() {
                        export_shared_locals(function, stack, shared);
                    }
                    stack.pop();
                    let mut result =
                        VmResult::success_with_diagnostics(output.clone(), value, diagnostics);
                    result.return_ref = return_ref;
                    return result;
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
        args: Vec<CallArgument>,
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
                let values = match call_args_to_positional(&name, args) {
                    Ok(values) => values,
                    Err(message) => {
                        return self.runtime_error(output, compiled, stack, message);
                    }
                };
                if name == "var_dump"
                    && let Some(message) = debug_info_gap_message(compiled, &values)
                {
                    return self.runtime_error(output, compiled, stack, message);
                }
                execute_builtin(&name, values, output)
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
            Value::String(name) => self.call_named_callable(
                compiled,
                &name.to_string_lossy(),
                args,
                output,
                stack,
                state,
            ),
            Value::Array(array) => {
                self.call_array_callable(compiled, &array, args, output, stack, state)
            }
            Value::Object(object) => {
                self.call_object_callable(compiled, object, args, output, stack, state)
            }
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

    fn call_fiber_callable(
        &self,
        compiled: &CompiledUnit,
        fiber: FiberRef,
        callee: Value,
        args: Vec<CallArgument>,
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
                    FunctionCall::new(args, Vec::new()).running_fiber(fiber),
                    output,
                    stack,
                    state,
                )
            }
            Value::Callable(CallableValue::Closure { function, captures }) => self
                .execute_function(
                    compiled,
                    FunctionId::new(function),
                    FunctionCall::new(args, captures).running_fiber(fiber),
                    output,
                    stack,
                    state,
                ),
            other => self.call_callable(compiled, other, args, output, stack, state),
        }
    }

    fn call_named_callable(
        &self,
        compiled: &CompiledUnit,
        name: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        if let Some((class_name, method)) = name.split_once("::") {
            return self.call_static_method_callable(
                compiled, class_name, method, args, output, stack, state,
            );
        }
        let normalized = name.to_ascii_lowercase();
        if let Some(function) = compiled.lookup_function(&normalized) {
            return self.execute_function(
                compiled,
                function,
                FunctionCall::new(args, Vec::new()),
                output,
                stack,
                state,
            );
        }
        if BuiltinRegistry::new().contains(&normalized) {
            let values = match call_args_to_positional(&normalized, args) {
                Ok(values) => values,
                Err(message) => {
                    return self.runtime_error(output, compiled, stack, message);
                }
            };
            if normalized == "var_dump"
                && let Some(message) = debug_info_gap_message(compiled, &values)
            {
                return self.runtime_error(output, compiled, stack, message);
            }
            return execute_builtin(&normalized, values, output);
        }
        self.runtime_error(
            output,
            compiled,
            stack,
            format!("E_PHP_VM_UNRESOLVED_CALLABLE: function {name} is not defined"),
        )
    }

    fn call_array_callable(
        &self,
        compiled: &CompiledUnit,
        array: &PhpArray,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let elements = array
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>();
        let [target, method]: [Value; 2] = match elements.try_into() {
            Ok(elements) => elements,
            Err(_) => {
                return self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_INVALID_CALLABLE_ARRAY: callable arrays must contain exactly target and method",
                );
            }
        };
        let Some(method) = callable_string_value(method) else {
            return self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_INVALID_CALLABLE_ARRAY: callable array method must be string",
            );
        };
        match callable_resolve_reference(target) {
            Value::Object(object) => {
                self.call_object_method_callable(compiled, object, &method, args, output, stack, state)
            }
            Value::String(class_name) => self.call_static_method_callable(
                compiled,
                &class_name.to_string_lossy(),
                &method,
                args,
                output,
                stack,
                state,
            ),
            other => self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_INVALID_CALLABLE_ARRAY: callable array target must be object or class string, got {}",
                    value_type_name(&other)
                ),
            ),
        }
    }

    fn call_object_callable(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        self.call_object_method_callable(compiled, object, "__invoke", args, output, stack, state)
    }

    fn call_object_method_callable(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        method: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let Some(class) = compiled.lookup_class(&object.class_name()) else {
            return self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                    object.class_name()
                ),
            );
        };
        let scope = current_scope_class(compiled, stack);
        let resolved = match lookup_method_in_hierarchy(compiled, class, method, scope.as_deref()) {
            Ok(Some(method)) => method,
            Ok(None) => {
                return match self.call_magic_instance_method(
                    compiled,
                    object.clone(),
                    "__call",
                    method,
                    args,
                    output,
                    stack,
                    state,
                ) {
                    Ok(Some(result)) => result,
                    Ok(None) => self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                            object.class_name(),
                            method
                        ),
                    ),
                    Err(result) => result,
                };
            }
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        let method_entry = resolved.method;
        let declaring_class = resolved.class;
        if method_entry.flags.is_static {
            return self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_STATIC_METHOD_AS_INSTANCE: method {}::{} is static",
                    declaring_class.name, method_entry.name
                ),
            );
        }
        if method_entry.flags.is_private || method_entry.flags.is_protected {
            return match self.call_magic_instance_method(
                compiled,
                object.clone(),
                "__call",
                method,
                args,
                output,
                stack,
                state,
            ) {
                Ok(Some(result)) => result,
                Ok(None) => {
                    if let Err(message) =
                        validate_method_callable(compiled, stack, declaring_class, method_entry)
                    {
                        self.runtime_error(output, compiled, stack, message)
                    } else {
                        self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_METHOD_VISIBILITY: inaccessible method passed validation",
                        )
                    }
                }
                Err(result) => result,
            };
        }
        if let Err(message) =
            validate_method_callable(compiled, stack, declaring_class, method_entry)
        {
            return self.runtime_error(output, compiled, stack, message);
        }
        self.record_runtime_trace_event(format!(
            "object-dispatch class={} method={} declaring_class={}",
            object.class_name(),
            method_entry.name,
            declaring_class.name
        ));
        self.execute_function(
            compiled,
            method_entry.function,
            FunctionCall::new(args, Vec::new())
                .with_this(object.clone())
                .with_class_context(
                    declaring_class.name.clone(),
                    object.class_name(),
                    declaring_class.name.clone(),
                ),
            output,
            stack,
            state,
        )
    }

    fn call_object_method_value(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        method: &str,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        let result = self.call_object_method_callable(
            compiled,
            object,
            method,
            Vec::new(),
            output,
            stack,
            state,
        );
        if !result.status.is_success()
            || result.yielded.is_some()
            || result.fiber_suspension.is_some()
        {
            return Err(result);
        }
        Ok(result.return_value.unwrap_or(Value::Null))
    }

    fn foreach_iterator_from_value(
        &self,
        compiled: &CompiledUnit,
        source: Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<ForeachIterator, VmResult> {
        match source {
            Value::Array(array) => Ok(ForeachIterator::Snapshot {
                entries: array
                    .iter()
                    .map(|(key, value)| (key.clone(), effective_value(value)))
                    .collect(),
                position: 0,
            }),
            Value::Generator(generator) => Ok(ForeachIterator::Generator {
                generator,
                consumed: false,
            }),
            Value::Object(object) => {
                self.foreach_iterator_from_object(compiled, object, output, stack, state)
            }
            source => {
                let diagnostic = unsupported_feature(
                    "E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE",
                    format!(
                        "foreach over {} is not implemented; Phase 5 supports arrays, public-property objects, Iterator, IteratorAggregate, and generator MVP objects",
                        value_type_name(&source)
                    ),
                    RuntimeSourceSpan::default(),
                    stack_trace(compiled, stack),
                );
                Err(VmResult {
                    status: ExecutionStatus::unsupported(diagnostic.message().to_owned()),
                    output: output.clone(),
                    diagnostics: vec![diagnostic],
                    return_value: None,
                    yielded: None,
                    fiber_suspension: None,
                    return_ref: None,
                    trace: Vec::new(),
                })
            }
        }
    }

    fn foreach_iterator_from_object(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<ForeachIterator, VmResult> {
        let class_name = object.class_name();
        match class_implements_interface(compiled, &class_name, "Iterator", &mut Vec::new()) {
            Ok(true) => {
                self.call_object_method_value(
                    compiled,
                    object.clone(),
                    "rewind",
                    output,
                    stack,
                    state,
                )?;
                return Ok(ForeachIterator::IteratorObject {
                    object,
                    needs_next: false,
                });
            }
            Ok(false) => {}
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        }
        match class_implements_interface(
            compiled,
            &class_name,
            "IteratorAggregate",
            &mut Vec::new(),
        ) {
            Ok(true) => {
                let inner = self.call_object_method_value(
                    compiled,
                    object,
                    "getIterator",
                    output,
                    stack,
                    state,
                )?;
                return self.foreach_iterator_from_value(compiled, inner, output, stack, state);
            }
            Ok(false) => {}
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        }
        if let Err(message) = object_property_iteration_keys(compiled, &object) {
            return Err(self.runtime_error(output, compiled, stack, message));
        }
        Ok(ForeachIterator::ObjectProperties {
            object,
            position: 0,
        })
    }

    fn call_magic_instance_method(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        magic_method: &str,
        called_method: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Option<VmResult>, VmResult> {
        let Some(class) = compiled.lookup_class(&object.class_name()) else {
            return Ok(None);
        };
        let resolved = match lookup_method_in_hierarchy(compiled, class, magic_method, None) {
            Ok(Some(method)) => method,
            Ok(None) => return Ok(None),
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static
            || resolved.method.flags.is_private
            || resolved.method.flags.is_protected
        {
            return Ok(None);
        }
        let guard = MagicMethodCall {
            receiver: format!("object:{}", object.id()),
            magic_method: normalize_method_name(magic_method),
            called_method: normalize_method_name(called_method),
        };
        if state
            .magic_method_stack
            .iter()
            .any(|active| active == &guard)
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_MAGIC_METHOD_RECURSION: recursive {magic_method} for {}::{called_method}",
                    object.class_name()
                ),
            ));
        }
        let magic_args = vec![
            CallArgument::positional(Value::String(PhpString::from_test_str(called_method))),
            CallArgument::positional(magic_args_array(args)),
        ];
        state.magic_method_stack.push(guard);
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(magic_args, Vec::new())
                .with_this(object.clone())
                .with_class_context(
                    resolved.class.name.clone(),
                    object.class_name(),
                    resolved.class.name.clone(),
                ),
            output,
            stack,
            state,
        );
        let _ = state.magic_method_stack.pop();
        Ok(Some(result))
    }

    #[allow(clippy::too_many_arguments)]
    fn call_magic_static_method(
        &self,
        compiled: &CompiledUnit,
        class: &php_ir::module::ClassEntry,
        magic_method: &str,
        called_method: &str,
        args: Vec<CallArgument>,
        called_class: String,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Option<VmResult>, VmResult> {
        let resolved = match lookup_method_in_hierarchy(compiled, class, magic_method, None) {
            Ok(Some(method)) => method,
            Ok(None) => return Ok(None),
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if !resolved.method.flags.is_static
            || resolved.method.flags.is_private
            || resolved.method.flags.is_protected
        {
            return Ok(None);
        }
        let guard = MagicMethodCall {
            receiver: format!("class:{}", normalize_class_name(&class.name)),
            magic_method: normalize_method_name(magic_method),
            called_method: normalize_method_name(called_method),
        };
        if state
            .magic_method_stack
            .iter()
            .any(|active| active == &guard)
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_MAGIC_METHOD_RECURSION: recursive {magic_method} for {}::{called_method}",
                    class.name
                ),
            ));
        }
        let magic_args = vec![
            CallArgument::positional(Value::String(PhpString::from_test_str(called_method))),
            CallArgument::positional(magic_args_array(args)),
        ];
        state.magic_method_stack.push(guard);
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(magic_args, Vec::new()).with_class_context(
                resolved.class.name.clone(),
                called_class,
                resolved.class.name.clone(),
            ),
            output,
            stack,
            state,
        );
        let _ = state.magic_method_stack.pop();
        Ok(Some(result))
    }

    fn advance_generator_to_first_yield(
        &self,
        compiled: &CompiledUnit,
        generator: GeneratorRef,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Option<(Option<Value>, Value)>, VmResult> {
        match generator.state() {
            GeneratorState::Created => {}
            GeneratorState::Suspended => return Ok(generator.current()),
            GeneratorState::Closed => return Ok(None),
            GeneratorState::Running => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_REENTRANCY: generator is already running",
                ));
            }
            GeneratorState::Errored => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_ERRORED: generator already errored",
                ));
            }
        }
        generator.set_state(GeneratorState::Running);
        self.record_runtime_trace_event(format!(
            "generator state function={} transition=created->running",
            generator.function()
        ));
        let args = generator
            .args()
            .into_iter()
            .map(CallArgument::positional)
            .collect();
        let result = self.execute_function(
            compiled,
            FunctionId::new(generator.function()),
            FunctionCall::new(args, Vec::new()).running_generator(generator.clone()),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            generator.set_state(GeneratorState::Errored);
            self.record_runtime_trace_event(format!(
                "generator state function={} transition=running->errored",
                generator.function()
            ));
            return Err(result);
        }
        if let Some(yielded) = result.yielded {
            generator.suspend(yielded.key.clone(), yielded.value.clone());
            self.record_runtime_trace_event(format!(
                "generator suspend function={} key={} value={}",
                generator.function(),
                yielded
                    .key
                    .as_ref()
                    .map(trace_value)
                    .unwrap_or_else(|| "None".to_owned()),
                trace_value(&yielded.value)
            ));
            Ok(Some((yielded.key, yielded.value)))
        } else {
            generator.close(result.return_value);
            self.record_runtime_trace_event(format!(
                "generator state function={} transition=running->closed",
                generator.function()
            ));
            Ok(None)
        }
    }

    fn resume_generator_to_next_yield(
        &self,
        compiled: &CompiledUnit,
        generator: GeneratorRef,
        input: GeneratorResumeInput,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Option<(Option<Value>, Value)>, VmResult> {
        match generator.state() {
            GeneratorState::Suspended => {}
            GeneratorState::Closed => return Ok(None),
            GeneratorState::Running => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_REENTRANCY: generator is already running",
                ));
            }
            GeneratorState::Errored => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_ERRORED: generator already errored",
                ));
            }
            GeneratorState::Created => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_NOT_STARTED: generator has not reached a yield",
                ));
            }
        }

        let Some(continuation) = state.generator_continuations.remove(&generator.id()) else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_GENERATOR_CONTINUATION_MISSING: suspended generator has no VM continuation",
            ));
        };
        generator.set_state(GeneratorState::Running);
        self.record_runtime_trace_event(format!(
            "generator state function={} transition=suspended->running input={}",
            generator.function(),
            match &input {
                GeneratorResumeInput::Value(value) => format!("value({})", trace_value(value)),
                GeneratorResumeInput::Throw(value) => format!("throw({})", trace_value(value)),
            }
        ));
        let result = self.execute_function(
            compiled,
            FunctionId::new(generator.function()),
            FunctionCall::new(Vec::new(), Vec::new())
                .running_generator(generator.clone())
                .resume_generator(continuation, input),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            generator.set_state(GeneratorState::Errored);
            state.generator_continuations.remove(&generator.id());
            self.record_runtime_trace_event(format!(
                "generator state function={} transition=running->errored",
                generator.function()
            ));
            return Err(result);
        }
        if let Some(yielded) = result.yielded {
            generator.suspend(yielded.key.clone(), yielded.value.clone());
            self.record_runtime_trace_event(format!(
                "generator suspend function={} key={} value={}",
                generator.function(),
                yielded
                    .key
                    .as_ref()
                    .map(trace_value)
                    .unwrap_or_else(|| "None".to_owned()),
                trace_value(&yielded.value)
            ));
            Ok(Some((yielded.key, yielded.value)))
        } else {
            state.generator_continuations.remove(&generator.id());
            generator.close(result.return_value);
            self.record_runtime_trace_event(format!(
                "generator state function={} transition=running->closed",
                generator.function()
            ));
            Ok(None)
        }
    }

    fn advance_yield_from_delegation(
        &self,
        compiled: &CompiledUnit,
        key: YieldFromKey,
        source: Operand,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<YieldFromStep, VmResult> {
        if !state.yield_from_delegations.contains_key(&key) {
            let source = match read_operand(compiled.unit(), stack, source) {
                Ok(source) => source,
                Err(message) => {
                    return Err(self.runtime_error(output, compiled, stack, message));
                }
            };
            let delegation = match source {
                Value::Array(array) => YieldFromDelegation::Array {
                    entries: array
                        .iter()
                        .map(|(key, value)| (key.clone(), effective_value(value)))
                        .collect(),
                    position: 0,
                },
                Value::Generator(generator) => YieldFromDelegation::Generator {
                    generator,
                    started: false,
                },
                other => {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_UNSUPPORTED_YIELD_FROM_SOURCE: yield from over {} is not implemented; Phase 5 supports arrays and generator MVP objects",
                            value_type_name(&other)
                        ),
                    ));
                }
            };
            state.yield_from_delegations.insert(key.clone(), delegation);
        }

        let Some(mut delegation) = state.yield_from_delegations.remove(&key) else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_YIELD_FROM_DELEGATION_MISSING: yield from delegation state is missing",
            ));
        };
        let step = match &mut delegation {
            YieldFromDelegation::Array { entries, position } => {
                if let Some((entry_key, value)) = entries.get(*position).cloned() {
                    *position += 1;
                    YieldFromStep::Yield {
                        key: Some(array_key_to_value(entry_key)),
                        value,
                    }
                } else {
                    YieldFromStep::Complete(Value::Null)
                }
            }
            YieldFromDelegation::Generator { generator, started } => {
                let next = if *started {
                    self.resume_generator_to_next_yield(
                        compiled,
                        generator.clone(),
                        GeneratorResumeInput::Value(Value::Null),
                        output,
                        stack,
                        state,
                    )?
                } else {
                    *started = true;
                    self.advance_generator_to_first_yield(
                        compiled,
                        generator.clone(),
                        output,
                        stack,
                        state,
                    )?
                };
                if let Some((key, value)) = next {
                    YieldFromStep::Yield { key, value }
                } else {
                    YieldFromStep::Complete(generator.return_value().unwrap_or(Value::Null))
                }
            }
        };
        if matches!(step, YieldFromStep::Yield { .. }) {
            state.yield_from_delegations.insert(key, delegation);
        }
        Ok(step)
    }

    #[allow(clippy::too_many_arguments)]
    fn suspend_current_fiber(
        &self,
        compiled: &CompiledUnit,
        running_fiber: &Option<FiberRef>,
        args: Vec<CallArgument>,
        resume_result: php_ir::ids::RegId,
        block_id: BlockId,
        instruction_index: usize,
        foreach_iterators: &HashMap<php_ir::ids::RegId, ForeachIterator>,
        exception_handlers: &[ExceptionHandler],
        pending_control: &Option<PendingControl>,
        output: &OutputBuffer,
        stack: &mut CallStack,
    ) -> Result<VmResult, VmResult> {
        let Some(fiber) = running_fiber.as_ref() else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_FIBER_SUSPEND_OUTSIDE_FIBER: Fiber::suspend called outside a running fiber",
            ));
        };
        if let Some(name) = args.iter().find_map(|arg| arg.name.as_deref()) {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_UNKNOWN_NAMED_ARG: Fiber::suspend has no builtin parameter ${name}"
                ),
            ));
        }
        if args.len() > 1 {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_TOO_MANY_ARGS: Fiber::suspend expects at most 1 argument(s), {} given",
                    args.len()
                ),
            ));
        }
        let value = args
            .into_iter()
            .next()
            .map(|arg| arg.value)
            .unwrap_or(Value::Null);
        self.record_runtime_trace_event(format!(
            "fiber suspend transition=running->suspended state={:?} value={}",
            fiber.state(),
            trace_value(&value)
        ));
        let Some(frame) = stack.pop() else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_FIBER_FRAME_MISSING: fiber frame missing at suspend",
            ));
        };
        let mut result = VmResult::success(output.clone(), None);
        result.fiber_suspension = Some(FiberSuspension {
            value,
            continuations: vec![FiberContinuation {
                frame,
                block_id,
                instruction_index,
                resume_result,
                foreach_iterators: foreach_iterators.clone(),
                exception_handlers: exception_handlers.to_vec(),
                pending_control: pending_control.clone(),
            }],
        });
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn propagate_fiber_suspension(
        &self,
        mut result: VmResult,
        compiled: &CompiledUnit,
        resume_result: php_ir::ids::RegId,
        block_id: BlockId,
        instruction_index: usize,
        foreach_iterators: &HashMap<php_ir::ids::RegId, ForeachIterator>,
        exception_handlers: &[ExceptionHandler],
        pending_control: &Option<PendingControl>,
        output: &OutputBuffer,
        stack: &mut CallStack,
    ) -> VmResult {
        if let Some(suspension) = result.fiber_suspension.as_mut() {
            let Some(frame) = stack.pop() else {
                return self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_FIBER_FRAME_MISSING: caller frame missing while propagating fiber suspension",
                );
            };
            suspension.continuations.insert(
                0,
                FiberContinuation {
                    frame,
                    block_id,
                    instruction_index,
                    resume_result,
                    foreach_iterators: foreach_iterators.clone(),
                    exception_handlers: exception_handlers.to_vec(),
                    pending_control: pending_control.clone(),
                },
            );
        }
        result
    }

    fn resume_fiber_continuations(
        &self,
        compiled: &CompiledUnit,
        fiber: FiberRef,
        mut continuations: Vec<FiberContinuation>,
        mut input: FiberResumeInput,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        while let Some(continuation) = continuations.pop() {
            let function = continuation.frame.function;
            let result = self.execute_function(
                compiled,
                function,
                FunctionCall::new(Vec::new(), Vec::new()).resume_fiber(
                    fiber.clone(),
                    continuation,
                    input,
                ),
                output,
                stack,
                state,
            );
            if !result.status.is_success() || result.fiber_suspension.is_some() {
                return result;
            }
            if continuations.is_empty() {
                return result;
            }
            input = FiberResumeInput::Value(result.return_value.unwrap_or(Value::Null));
        }
        VmResult::success(output.clone(), Some(Value::Null))
    }

    fn call_generator_method(
        &self,
        compiled: &CompiledUnit,
        generator: GeneratorRef,
        method: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        let method_name = normalize_method_name(method);
        if matches!(
            method_name.as_str(),
            "current" | "key" | "next" | "valid" | "rewind" | "getreturn"
        ) {
            validate_generator_arg_count(&method_name, &args, 0)
                .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
        }

        match method_name.as_str() {
            "current" => {
                self.advance_generator_to_first_yield(
                    compiled,
                    generator.clone(),
                    output,
                    stack,
                    state,
                )?;
                Ok(generator.current_value().unwrap_or(Value::Null))
            }
            "key" => {
                self.advance_generator_to_first_yield(
                    compiled,
                    generator.clone(),
                    output,
                    stack,
                    state,
                )?;
                Ok(generator.current_key().unwrap_or(Value::Null))
            }
            "valid" => {
                self.advance_generator_to_first_yield(
                    compiled,
                    generator.clone(),
                    output,
                    stack,
                    state,
                )?;
                Ok(Value::Bool(matches!(
                    generator.state(),
                    GeneratorState::Suspended
                )))
            }
            "rewind" => match generator.state() {
                GeneratorState::Created | GeneratorState::Suspended => {
                    self.advance_generator_to_first_yield(
                        compiled, generator, output, stack, state,
                    )?;
                    Ok(Value::Null)
                }
                GeneratorState::Closed => Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_REWIND_CLOSED: cannot rewind a closed generator",
                )),
                GeneratorState::Running => Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_REENTRANCY: generator is already running",
                )),
                GeneratorState::Errored => Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_ERRORED: generator already errored",
                )),
            },
            "next" => {
                match generator.state() {
                    GeneratorState::Created => {
                        self.advance_generator_to_first_yield(
                            compiled,
                            generator.clone(),
                            output,
                            stack,
                            state,
                        )?;
                    }
                    GeneratorState::Suspended => {}
                    GeneratorState::Closed => return Ok(Value::Null),
                    GeneratorState::Running => {
                        return Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_GENERATOR_REENTRANCY: generator is already running",
                        ));
                    }
                    GeneratorState::Errored => {
                        return Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_GENERATOR_ERRORED: generator already errored",
                        ));
                    }
                }
                if matches!(generator.state(), GeneratorState::Suspended) {
                    self.resume_generator_to_next_yield(
                        compiled,
                        generator,
                        GeneratorResumeInput::Value(Value::Null),
                        output,
                        stack,
                        state,
                    )?;
                }
                Ok(Value::Null)
            }
            "getreturn" => match generator.state() {
                GeneratorState::Closed => Ok(generator.return_value().unwrap_or(Value::Null)),
                GeneratorState::Created | GeneratorState::Suspended | GeneratorState::Running => {
                    Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_GENERATOR_GET_RETURN_BEFORE_CLOSE: cannot get return value before generator completion",
                    ))
                }
                GeneratorState::Errored => Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_GENERATOR_ERRORED: generator already errored",
                )),
            },
            "send" => {
                validate_generator_arg_count(&method_name, &args, 1)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                if matches!(generator.state(), GeneratorState::Created) {
                    self.advance_generator_to_first_yield(
                        compiled,
                        generator.clone(),
                        output,
                        stack,
                        state,
                    )?;
                }
                if !matches!(generator.state(), GeneratorState::Suspended) {
                    return Ok(Value::Null);
                }
                let next = self.resume_generator_to_next_yield(
                    compiled,
                    generator,
                    GeneratorResumeInput::Value(args[0].value.clone()),
                    output,
                    stack,
                    state,
                )?;
                Ok(next.map(|(_, value)| value).unwrap_or(Value::Null))
            }
            "throw" => {
                validate_generator_arg_count(&method_name, &args, 1)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                if matches!(generator.state(), GeneratorState::Created) {
                    self.advance_generator_to_first_yield(
                        compiled,
                        generator.clone(),
                        output,
                        stack,
                        state,
                    )?;
                }
                let throwable = args[0].value.clone();
                let Value::Object(object) = &throwable else {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_GENERATOR_THROW_NON_THROWABLE: Generator::throw expects Throwable, {} given",
                            value_type_name(&throwable)
                        ),
                    ));
                };
                if internal_throwable_instanceof(&object.class_name(), "throwable") != Some(true) {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_GENERATOR_THROW_NON_THROWABLE: Generator::throw expects Throwable, {} given",
                            object.class_name()
                        ),
                    ));
                }
                if !matches!(generator.state(), GeneratorState::Suspended) {
                    return Err(uncaught_exception(output, compiled, stack, throwable));
                }
                let next = self.resume_generator_to_next_yield(
                    compiled,
                    generator,
                    GeneratorResumeInput::Throw(throwable),
                    output,
                    stack,
                    state,
                )?;
                Ok(next.map(|(_, value)| value).unwrap_or(Value::Null))
            }
            _ => Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!("E_PHP_VM_UNKNOWN_METHOD: method Generator::{method} is not defined"),
            )),
        }
    }

    fn call_fiber_method(
        &self,
        compiled: &CompiledUnit,
        fiber: FiberRef,
        method: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        let method_name = normalize_method_name(method);
        match method_name.as_str() {
            "start" => {
                let args = call_args_to_positional("Fiber::start", args)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?
                    .into_iter()
                    .map(CallArgument::positional)
                    .collect::<Vec<_>>();
                match fiber.state() {
                    FiberState::NotStarted => {}
                    FiberState::Running => {
                        return Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_FIBER_ALREADY_RUNNING: FiberError: fiber is already running",
                        ));
                    }
                    FiberState::Suspended => {
                        return Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_FIBER_ALREADY_STARTED: FiberError: fiber has already started",
                        ));
                    }
                    FiberState::Terminated | FiberState::Errored => {
                        return Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_FIBER_ALREADY_TERMINATED: FiberError: fiber has already terminated",
                        ));
                    }
                }
                fiber.set_state(FiberState::Running);
                self.record_runtime_trace_event("fiber start transition=not-started->running");
                let result = self.call_fiber_callable(
                    compiled,
                    fiber.clone(),
                    fiber.callable(),
                    args,
                    output,
                    stack,
                    state,
                );
                if !result.status.is_success() {
                    fiber.set_state(FiberState::Errored);
                    self.record_runtime_trace_event("fiber start transition=running->errored");
                    return Err(result);
                }
                if let Some(suspension) = result.fiber_suspension {
                    state
                        .fiber_continuations
                        .insert(fiber.id(), suspension.continuations);
                    fiber.set_state(FiberState::Suspended);
                    self.record_runtime_trace_event(format!(
                        "fiber start transition=running->suspended value={}",
                        trace_value(&suspension.value)
                    ));
                    return Ok(suspension.value);
                }
                fiber.terminate(result.return_value);
                self.record_runtime_trace_event("fiber start transition=running->terminated");
                Ok(Value::Null)
            }
            "isstarted" => {
                validate_fiber_arg_count(&method_name, &args, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                Ok(Value::Bool(!matches!(
                    fiber.state(),
                    FiberState::NotStarted
                )))
            }
            "issuspended" => {
                validate_fiber_arg_count(&method_name, &args, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                Ok(Value::Bool(matches!(fiber.state(), FiberState::Suspended)))
            }
            "isrunning" => {
                validate_fiber_arg_count(&method_name, &args, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                Ok(Value::Bool(matches!(fiber.state(), FiberState::Running)))
            }
            "isterminated" => {
                validate_fiber_arg_count(&method_name, &args, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                Ok(Value::Bool(matches!(
                    fiber.state(),
                    FiberState::Terminated | FiberState::Errored
                )))
            }
            "resume" => {
                validate_fiber_arg_count(&method_name, &args, 1)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                if !matches!(fiber.state(), FiberState::Suspended) {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_FIBER_NOT_SUSPENDED: FiberError: fiber is not suspended",
                    ));
                }
                let Some(continuations) = state.fiber_continuations.remove(&fiber.id()) else {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_FIBER_CONTINUATION_MISSING: suspended fiber has no VM continuation",
                    ));
                };
                fiber.set_state(FiberState::Running);
                self.record_runtime_trace_event(format!(
                    "fiber resume transition=suspended->running input={}",
                    trace_value(&args[0].value)
                ));
                let result = self.resume_fiber_continuations(
                    compiled,
                    fiber.clone(),
                    continuations,
                    FiberResumeInput::Value(args[0].value.clone()),
                    output,
                    stack,
                    state,
                );
                if !result.status.is_success() {
                    fiber.set_state(FiberState::Errored);
                    self.record_runtime_trace_event("fiber resume transition=running->errored");
                    return Err(result);
                }
                if let Some(suspension) = result.fiber_suspension {
                    state
                        .fiber_continuations
                        .insert(fiber.id(), suspension.continuations);
                    fiber.set_state(FiberState::Suspended);
                    self.record_runtime_trace_event(format!(
                        "fiber resume transition=running->suspended value={}",
                        trace_value(&suspension.value)
                    ));
                    return Ok(suspension.value);
                }
                fiber.terminate(result.return_value);
                self.record_runtime_trace_event("fiber resume transition=running->terminated");
                Ok(Value::Null)
            }
            "throw" => {
                validate_fiber_arg_count(&method_name, &args, 1)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                if !matches!(fiber.state(), FiberState::Suspended) {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_FIBER_NOT_SUSPENDED: FiberError: fiber is not suspended",
                    ));
                }
                let throwable = args[0].value.clone();
                let Value::Object(object) = &throwable else {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_FIBER_THROW_NON_THROWABLE: Fiber::throw expects Throwable, {} given",
                            value_type_name(&throwable)
                        ),
                    ));
                };
                if internal_throwable_instanceof(&object.class_name(), "throwable") != Some(true) {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_FIBER_THROW_NON_THROWABLE: Fiber::throw expects Throwable, {} given",
                            object.class_name()
                        ),
                    ));
                }
                let Some(continuations) = state.fiber_continuations.remove(&fiber.id()) else {
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_FIBER_CONTINUATION_MISSING: suspended fiber has no VM continuation",
                    ));
                };
                fiber.set_state(FiberState::Running);
                self.record_runtime_trace_event(format!(
                    "fiber throw transition=suspended->running input={}",
                    trace_value(&throwable)
                ));
                let result = self.resume_fiber_continuations(
                    compiled,
                    fiber.clone(),
                    continuations,
                    FiberResumeInput::Throw(throwable),
                    output,
                    stack,
                    state,
                );
                if !result.status.is_success() {
                    fiber.set_state(FiberState::Errored);
                    self.record_runtime_trace_event("fiber throw transition=running->errored");
                    return Err(result);
                }
                if let Some(suspension) = result.fiber_suspension {
                    state
                        .fiber_continuations
                        .insert(fiber.id(), suspension.continuations);
                    fiber.set_state(FiberState::Suspended);
                    self.record_runtime_trace_event(format!(
                        "fiber throw transition=running->suspended value={}",
                        trace_value(&suspension.value)
                    ));
                    return Ok(suspension.value);
                }
                fiber.terminate(result.return_value);
                self.record_runtime_trace_event("fiber throw transition=running->terminated");
                Ok(Value::Null)
            }
            "getreturn" => {
                validate_fiber_arg_count(&method_name, &args, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                match fiber.state() {
                    FiberState::Terminated => Ok(fiber.return_value().unwrap_or(Value::Null)),
                    FiberState::Errored => Err(self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_FIBER_ERRORED: FiberError: fiber terminated with an exception",
                    )),
                    FiberState::NotStarted | FiberState::Running | FiberState::Suspended => {
                        Err(self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_FIBER_GET_RETURN_BEFORE_TERMINATION: FiberError: cannot get fiber return value before termination",
                        ))
                    }
                }
            }
            "suspend" => Err(self.runtime_error(
                output,
                compiled,
                stack,
                "E_PHP_VM_FIBER_SUSPEND_INSTANCE_CALL: Fiber::suspend must be called statically",
            )),
            _ => Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!("E_PHP_VM_UNKNOWN_METHOD: method Fiber::{method} is not defined"),
            )),
        }
    }

    fn value_to_string(
        &self,
        compiled: &CompiledUnit,
        value: &Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        match value {
            Value::Object(object) => {
                self.object_to_string(compiled, object.clone(), output, stack, state)
            }
            Value::Reference(cell) => {
                self.value_to_string(compiled, &cell.get(), output, stack, state)
            }
            other => to_string(other)
                .map_err(|message| self.runtime_error(output, compiled, stack, message)),
        }
    }

    fn object_to_string(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        let Some(class) = compiled.lookup_class(&object.class_name()) else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
                    object.class_name()
                ),
            ));
        };
        let resolved = match lookup_method_in_hierarchy(compiled, class, "__toString", None) {
            Ok(Some(method)) => method,
            Ok(None) => {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    format!(
                        "E_PHP_RUNTIME_OBJECT_TO_STRING_GAP: object {} does not define __toString",
                        object.class_name()
                    ),
                ));
            }
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static
            || resolved.method.flags.is_private
            || resolved.method.flags.is_protected
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_TOSTRING_INACCESSIBLE: method {}::__toString is not public instance",
                    resolved.class.name
                ),
            ));
        }
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(Vec::new(), Vec::new())
                .with_this(object.clone())
                .with_class_context(
                    resolved.class.name.clone(),
                    object.class_name(),
                    resolved.class.name.clone(),
                ),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            return Err(result);
        }
        match result.return_value.unwrap_or(Value::Null) {
            Value::String(value) => Ok(value),
            other => Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_TOSTRING_RETURN_TYPE: __toString returned {}, expected string",
                    value_type_name(&other)
                ),
            )),
        }
    }

    fn clone_object_with_magic(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        class: &php_ir::module::ClassEntry,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<ObjectRef, VmResult> {
        let copy = object.clone_shallow();
        let resolved = match lookup_method_in_hierarchy(compiled, class, "__clone", None) {
            Ok(Some(method)) => method,
            Ok(None) => return Ok(copy),
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static
            || resolved.method.flags.is_private
            || resolved.method.flags.is_protected
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_CLONE_METHOD_INACCESSIBLE: method {}::__clone is not public instance",
                    resolved.class.name
                ),
            ));
        }
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(Vec::new(), Vec::new())
                .with_this(copy.clone())
                .with_class_context(
                    resolved.class.name.clone(),
                    copy.class_name(),
                    resolved.class.name.clone(),
                ),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            return Err(result);
        }
        Ok(copy)
    }

    fn register_destructor_if_needed(
        &self,
        compiled: &CompiledUnit,
        class: &php_ir::module::ClassEntry,
        object: ObjectRef,
        state: &mut ExecutionState,
    ) {
        if let Ok(Some(resolved)) = lookup_method_in_hierarchy(compiled, class, "__destruct", None)
            && !resolved.method.flags.is_static
            && !resolved.method.flags.is_private
            && !resolved.method.flags.is_protected
        {
            state.destructor_queue.register(
                object,
                resolved.class.name.clone(),
                resolved.method.function,
            );
        }
    }

    fn run_shutdown_destructors(
        &self,
        compiled: &CompiledUnit,
        output: &mut OutputBuffer,
        state: &mut ExecutionState,
    ) -> Result<Vec<RuntimeDiagnostic>, VmResult> {
        let mut diagnostics = Vec::new();
        let mut executed = 0usize;
        while !state.destructor_queue.entries.is_empty() {
            let entries = state.destructor_queue.drain_reverse();
            for entry in entries {
                executed += 1;
                if executed > 4096 {
                    let stack = CallStack::new();
                    return Err(self.runtime_error(
                        output,
                        compiled,
                        &stack,
                        "E_PHP_VM_DESTRUCTOR_QUEUE_OVERFLOW: destructor queue exceeded 4096 executions",
                    ));
                }
                let mut stack = CallStack::new();
                let result = self.execute_function(
                    compiled,
                    entry.function,
                    FunctionCall::new(Vec::new(), Vec::new())
                        .with_this(entry.object.clone())
                        .with_class_context(
                            entry.class_name.clone(),
                            entry.object.class_name(),
                            entry.class_name.clone(),
                        ),
                    output,
                    &mut stack,
                    state,
                );
                if !result.status.is_success() {
                    return Err(result);
                }
                diagnostics.extend(result.diagnostics);
            }
        }
        Ok(diagnostics)
    }

    fn write_echo(
        &self,
        compiled: &CompiledUnit,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
        value: &Value,
    ) -> Result<(), VmResult> {
        let string = self.value_to_string(compiled, value, output, stack, state)?;
        output.write_php_string(&string);
        Ok(())
    }

    fn execute_binary(
        &self,
        compiled: &CompiledUnit,
        op: BinaryOp,
        lhs: &Value,
        rhs: &Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        match op {
            BinaryOp::Concat => {
                let mut bytes = self
                    .value_to_string(compiled, lhs, output, stack, state)?
                    .into_bytes();
                bytes.extend_from_slice(
                    self.value_to_string(compiled, rhs, output, stack, state)?
                        .as_bytes(),
                );
                Ok(Value::String(PhpString::from_bytes(bytes)))
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                let lhs = to_number(lhs)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                let rhs = to_number(rhs)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                execute_arithmetic(op, lhs, rhs)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))
            }
            BinaryOp::Pow => {
                let lhs = to_number(lhs)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                let rhs = to_number(rhs)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                Ok(Value::float(lhs.as_f64().powf(rhs.as_f64())))
            }
        }
    }

    fn execute_cast(
        &self,
        compiled: &CompiledUnit,
        kind: CastKind,
        src: &Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        match kind {
            CastKind::Bool => to_bool(src)
                .map(Value::Bool)
                .map_err(|message| self.runtime_error(output, compiled, stack, message)),
            CastKind::Int => to_int(src)
                .map(Value::Int)
                .map_err(|message| self.runtime_error(output, compiled, stack, message)),
            CastKind::Float => to_float(src)
                .map(Value::float)
                .map_err(|message| self.runtime_error(output, compiled, stack, message)),
            CastKind::String => self
                .value_to_string(compiled, src, output, stack, state)
                .map(Value::String),
            CastKind::Void => Ok(Value::Null),
            CastKind::Array => {
                Err(self.runtime_error(output, compiled, stack, "array cast is not implemented"))
            }
            CastKind::Object => {
                Err(self.runtime_error(output, compiled, stack, "object cast is not implemented"))
            }
        }
    }

    fn call_magic_property_method(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        method: &str,
        property: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Option<Value>, VmResult> {
        let Some(class) = compiled.lookup_class(&object.class_name()) else {
            return Ok(None);
        };
        let resolved = match lookup_method_in_hierarchy(compiled, class, method, None) {
            Ok(Some(method)) => method,
            Ok(None) => return Ok(None),
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static {
            return Ok(None);
        }
        if resolved.method.flags.is_private || resolved.method.flags.is_protected {
            return Ok(None);
        }
        let guard = MagicPropertyCall {
            object_id: object.id(),
            method: normalize_method_name(method),
            property: property.to_owned(),
        };
        if state
            .magic_property_stack
            .iter()
            .any(|active| active == &guard)
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_MAGIC_PROPERTY_RECURSION: recursive {method} for {}::${property}",
                    object.class_name()
                ),
            ));
        }
        state.magic_property_stack.push(guard);
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(args, Vec::new())
                .with_this(object.clone())
                .with_class_context(
                    resolved.class.name.clone(),
                    object.class_name(),
                    resolved.class.name.clone(),
                ),
            output,
            stack,
            state,
        );
        let _ = state.magic_property_stack.pop();
        if result.status.is_success() {
            Ok(Some(result.return_value.unwrap_or(Value::Null)))
        } else {
            Err(result)
        }
    }

    fn call_property_hook(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        class: &php_ir::module::ClassEntry,
        property: &php_ir::module::ClassPropertyEntry,
        function: FunctionId,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        let guard = PropertyHookCall {
            object_id: object.id(),
            class_name: normalize_class_name(&class.name),
            property: property.name.clone(),
        };
        if state
            .property_hook_stack
            .iter()
            .any(|active| active == &guard)
        {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_PROPERTY_HOOK_RECURSION: recursive hook for {}::${}",
                    class.name, property.name
                ),
            ));
        }
        state.property_hook_stack.push(guard);
        let result = self.execute_function(
            compiled,
            function,
            FunctionCall::new(args, Vec::new())
                .with_this(object.clone())
                .with_class_context(class.name.clone(), object.class_name(), class.name.clone()),
            output,
            stack,
            state,
        );
        let _ = state.property_hook_stack.pop();
        if result.status.is_success() {
            Ok(result.return_value.unwrap_or(Value::Null))
        } else {
            Err(result)
        }
    }

    fn call_static_method_callable(
        &self,
        compiled: &CompiledUnit,
        class_name: &str,
        method: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let class = match resolve_static_class_name(compiled, stack, class_name) {
            Ok(class) => class,
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        let scope = method_lookup_scope_for_static_call(compiled, stack, class_name);
        let resolved = match lookup_method_in_hierarchy(compiled, class, method, scope.as_deref()) {
            Ok(Some(method)) => method,
            Ok(None) => {
                let called_class = called_class_for_static_call(compiled, stack, class_name, class);
                return match self.call_magic_static_method(
                    compiled,
                    class,
                    "__callStatic",
                    method,
                    args,
                    called_class,
                    output,
                    stack,
                    state,
                ) {
                    Ok(Some(result)) => result,
                    Ok(None) => self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!(
                            "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                            class.name, method
                        ),
                    ),
                    Err(result) => result,
                };
            }
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        let method_entry = resolved.method;
        let declaring_class = resolved.class;
        if !method_entry.flags.is_static {
            return self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_NON_STATIC_METHOD_CALL: method {}::{} is not static",
                    declaring_class.name, method_entry.name
                ),
            );
        }
        if method_entry.flags.is_private || method_entry.flags.is_protected {
            let called_class = called_class_for_static_call(compiled, stack, class_name, class);
            return match self.call_magic_static_method(
                compiled,
                class,
                "__callStatic",
                method,
                args,
                called_class,
                output,
                stack,
                state,
            ) {
                Ok(Some(result)) => result,
                Ok(None) => {
                    if let Err(message) =
                        validate_method_callable(compiled, stack, declaring_class, method_entry)
                    {
                        self.runtime_error(output, compiled, stack, message)
                    } else {
                        self.runtime_error(
                            output,
                            compiled,
                            stack,
                            "E_PHP_VM_METHOD_VISIBILITY: inaccessible static method passed validation",
                        )
                    }
                }
                Err(result) => result,
            };
        }
        if let Err(message) =
            validate_method_callable(compiled, stack, declaring_class, method_entry)
        {
            return self.runtime_error(output, compiled, stack, message);
        }
        let called_class = called_class_for_static_call(compiled, stack, class_name, class);
        self.execute_function(
            compiled,
            method_entry.function,
            FunctionCall::new(args, Vec::new()).with_class_context(
                declaring_class.name.clone(),
                called_class,
                declaring_class.name.clone(),
            ),
            output,
            stack,
            state,
        )
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
        register_dynamic_classes(state, included.unit());
        let mut shared = shared_locals_from_current_frame(compiled, stack);
        let call = FunctionCall {
            args: Vec::new(),
            captures: Vec::new(),
            this_value: None,
            scope_class: None,
            called_class: None,
            declaring_class: None,
            shared_top_level_locals: Some(&mut shared),
            running_generator: None,
            resume_continuation: None,
            resume_input: None,
            running_fiber: None,
            resume_fiber_continuation: None,
            resume_fiber_input: None,
        };
        let result =
            self.execute_function(&included, included.unit().entry, call, output, stack, state);
        if result.status.is_success() {
            write_shared_locals_to_current_frame(compiled, stack, &shared);
        }
        result
    }

    fn call_autoload_builtin(
        &self,
        compiled: &CompiledUnit,
        name: &str,
        args: Vec<CallArgument>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let values = match call_args_to_positional(name, args) {
            Ok(values) => values,
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        match name {
            "spl_autoload_register" => {
                let Some(callback) = values.first() else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_AUTOLOAD_ARITY: spl_autoload_register expects at least 1 argument",
                    );
                };
                let callback = match autoload_callback_from_value(compiled, callback.clone()) {
                    Ok(callback) => callback,
                    Err(message) => return self.runtime_error(output, compiled, stack, message),
                };
                state.autoload_registry.register(callback);
                VmResult::success(output.clone(), Some(Value::Bool(true)))
            }
            "spl_autoload_unregister" => {
                let Some(callback) = values.first() else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_AUTOLOAD_ARITY: spl_autoload_unregister expects at least 1 argument",
                    );
                };
                let callback = match autoload_callback_from_value(compiled, callback.clone()) {
                    Ok(callback) => callback,
                    Err(message) => return self.runtime_error(output, compiled, stack, message),
                };
                let removed = state.autoload_registry.unregister(&callback);
                VmResult::success(output.clone(), Some(Value::Bool(removed)))
            }
            "spl_autoload_functions" => {
                let mut array = PhpArray::new();
                for callback in state.autoload_registry.callbacks() {
                    array.append(Value::Callable(callback.clone()));
                }
                VmResult::success(output.clone(), Some(Value::Array(array)))
            }
            "spl_autoload_call" => {
                let Some(class_name) = values.first() else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_AUTOLOAD_ARITY: spl_autoload_call expects class name",
                    );
                };
                let class_name = match to_string(class_name) {
                    Ok(name) => name.to_string_lossy(),
                    Err(message) => return self.runtime_error(output, compiled, stack, message),
                };
                match self.autoload_class(compiled, &class_name, output, stack, state) {
                    Ok(()) => VmResult::success(output.clone(), Some(Value::Null)),
                    Err(result) => result,
                }
            }
            "class_exists" | "interface_exists" => {
                let Some(class_name) = values.first() else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        format!("E_PHP_VM_AUTOLOAD_ARITY: {name} expects class name"),
                    );
                };
                let class_name = match to_string(class_name) {
                    Ok(name) => name.to_string_lossy(),
                    Err(message) => return self.runtime_error(output, compiled, stack, message),
                };
                let autoload = values
                    .get(1)
                    .is_none_or(|value| to_bool(value).unwrap_or(true));
                if autoload
                    && lookup_class_in_state(compiled, state, &class_name).is_none()
                    && let Err(result) =
                        self.autoload_class(compiled, &class_name, output, stack, state)
                {
                    return result;
                }
                let exists =
                    lookup_class_in_state(compiled, state, &class_name).is_some_and(|class| {
                        if name == "interface_exists" {
                            class.flags.is_interface
                        } else {
                            !class.flags.is_interface
                        }
                    });
                VmResult::success(output.clone(), Some(Value::Bool(exists)))
            }
            _ => self.runtime_error(
                output,
                compiled,
                stack,
                format!("E_PHP_VM_UNKNOWN_AUTOLOAD_BUILTIN: {name}"),
            ),
        }
    }

    fn autoload_class(
        &self,
        compiled: &CompiledUnit,
        class_name: &str,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<(), VmResult> {
        let normalized = normalize_class_name(class_name);
        if lookup_class_in_state(compiled, state, class_name).is_some()
            || state.autoload_stack.iter().any(|name| name == &normalized)
        {
            return Ok(());
        }
        let callbacks = state.autoload_registry.callbacks().to_vec();
        state.autoload_stack.push(normalized.clone());
        for callback in callbacks {
            let result = self.call_callable(
                compiled,
                Value::Callable(callback),
                vec![CallArgument::positional(Value::string(
                    class_name.as_bytes().to_vec(),
                ))],
                output,
                stack,
                state,
            );
            if !result.status.is_success() {
                let _ = state.autoload_stack.pop();
                return Err(result);
            }
            if lookup_class_in_state(compiled, state, class_name).is_some() {
                break;
            }
        }
        let _ = state.autoload_stack.pop();
        Ok(())
    }

    fn execute_eval(
        &self,
        compiled: &CompiledUnit,
        code: &Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        if state.eval_depth >= MAX_EVAL_DEPTH {
            return eval_failure(
                output,
                "E_PHP_VM_EVAL_RECURSION_LIMIT: maximum nested eval depth exceeded",
                stack_trace(compiled, stack),
            );
        }

        let code = match to_string(code) {
            Ok(code) => code.to_string_lossy(),
            Err(message) => return self.runtime_error(output, compiled, stack, message),
        };
        state.eval_counter += 1;
        let source_path = format!("eval://{}", state.eval_counter);
        let source = format!("<?php {code}");
        let frontend = php_semantics::analyze_source(&source);
        if frontend.has_errors() {
            return eval_failure(
                output,
                format!("E_PHP_VM_EVAL_PARSE_ERROR: {source_path} failed frontend analysis"),
                stack_trace(compiled, stack),
            );
        }
        let lowering = php_ir::lower_frontend_result(
            &frontend,
            php_ir::LoweringOptions {
                source_path: source_path.clone(),
                source_text: Some(source.clone()),
                ..php_ir::LoweringOptions::default()
            },
        );
        if !lowering.diagnostics.is_empty() || lowering.verification.is_err() {
            return eval_failure(
                output,
                format!("E_PHP_VM_EVAL_COMPILE_ERROR: {source_path} failed IR lowering"),
                stack_trace(compiled, stack),
            );
        }
        let evaluated = CompiledUnit::new(lowering.unit);
        let has_named_function_declarations = evaluated
            .unit()
            .function_table
            .iter()
            .any(|entry| entry.function != evaluated.unit().entry);
        let has_source_class_declarations = evaluated
            .unit()
            .classes
            .iter()
            .any(|class| class.span != php_ir::source_map::IrSpan::default());
        if has_named_function_declarations || has_source_class_declarations {
            return eval_failure(
                output,
                "E_PHP_VM_EVAL_DECLARATION_GAP: eval declarations are not merged into the active runtime unit",
                stack_trace(compiled, stack),
            );
        }

        let mut shared = shared_locals_from_current_frame(compiled, stack);
        let call = FunctionCall {
            args: Vec::new(),
            captures: Vec::new(),
            this_value: None,
            scope_class: None,
            called_class: None,
            declaring_class: None,
            shared_top_level_locals: Some(&mut shared),
            running_generator: None,
            resume_continuation: None,
            resume_input: None,
            running_fiber: None,
            resume_fiber_continuation: None,
            resume_fiber_input: None,
        };
        state.eval_depth += 1;
        let result = self.execute_function(
            &evaluated,
            evaluated.unit().entry,
            call,
            output,
            stack,
            state,
        );
        state.eval_depth -= 1;
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
    match value {
        Value::Null => "Null".to_owned(),
        Value::Bool(value) => format!("Bool({value})"),
        Value::Int(value) => format!("Int({value})"),
        Value::Float(value) => format!("Float({value})"),
        Value::String(value) => format!("String({:?})", value.to_string_lossy()),
        Value::Uninitialized => "Uninitialized".to_owned(),
        Value::Array(array) => format!("Array(len={}, shared={})", array.len(), array.is_shared()),
        Value::Object(object) => format!("Object(class={})", object.class_name()),
        Value::Fiber(fiber) => format!("Fiber(state={:?})", fiber.state()),
        Value::Generator(generator) => format!("Generator(state={:?})", generator.state()),
        Value::Callable(CallableValue::UserFunction { name }) => {
            format!("Callable(user_function={name})")
        }
        Value::Callable(CallableValue::Closure { function, captures }) => {
            format!("Closure(function={function}, captures={})", captures.len())
        }
        Value::Callable(CallableValue::InternalBuiltin { name }) => {
            format!("Callable(internal_builtin={name})")
        }
        Value::Callable(CallableValue::MethodPlaceholder { target }) => {
            format!("Callable(method_placeholder={target})")
        }
        Value::Callable(CallableValue::UnresolvedDynamic { target }) => {
            format!("Callable(unresolved_dynamic={target})")
        }
        Value::Reference(cell) => format!("Reference(value={})", trace_value(&cell.get())),
    }
}

fn format_array_key_for_trace(key: &ArrayKey) -> String {
    match key {
        ArrayKey::Int(value) => format!("int({value})"),
        ArrayKey::String(value) => format!("string({:?})", value.to_string_lossy()),
    }
}

fn format_foreach_iterator_kind(iterator: &ForeachIterator) -> &'static str {
    match iterator {
        ForeachIterator::Snapshot { .. } => "snapshot",
        ForeachIterator::ObjectProperties { .. } => "object-properties",
        ForeachIterator::IteratorObject { .. } => "iterator-object",
        ForeachIterator::Generator { .. } => "generator",
        ForeachIterator::ByReference { .. } => "by-reference",
    }
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

fn internal_throwable_display_name(class_name: &str) -> String {
    match normalize_class_name(class_name).as_str() {
        "throwable" => "Throwable".to_owned(),
        "exception" => "Exception".to_owned(),
        "error" => "Error".to_owned(),
        "typeerror" => "TypeError".to_owned(),
        "valueerror" => "ValueError".to_owned(),
        "argumentcounterror" => "ArgumentCountError".to_owned(),
        "fibererror" => "FiberError".to_owned(),
        _ => class_name.to_owned(),
    }
}

fn internal_throwable_parent(class_name: &str) -> Option<&'static str> {
    match normalize_class_name(class_name).as_str() {
        "typeerror" | "valueerror" | "argumentcounterror" | "fibererror" => Some("Error"),
        _ => None,
    }
}

fn internal_throwable_instanceof(object_class: &str, target_class: &str) -> Option<bool> {
    let object_class = normalize_class_name(object_class);
    let target_class = normalize_class_name(target_class);
    if !matches!(
        object_class.as_str(),
        "exception" | "error" | "typeerror" | "valueerror" | "argumentcounterror" | "fibererror"
    ) {
        return None;
    }
    if target_class == "throwable" || object_class == target_class {
        return Some(true);
    }
    let parent = internal_throwable_parent(&object_class)?;
    Some(normalize_class_name(parent) == target_class)
}

fn throwable_class_name(value: &Value) -> String {
    match value {
        Value::Object(object) => internal_throwable_display_name(&object.class_name()),
        other => value_type_name(other).to_owned(),
    }
}

fn make_exception_object(class_name: &str, message: &Value) -> Result<ObjectRef, String> {
    let message = to_string(message)?.to_string_lossy();
    let class_name = internal_throwable_display_name(class_name);
    let parent = internal_throwable_parent(&class_name).map(str::to_owned);
    let class = RuntimeClassEntry {
        name: class_name.clone(),
        parent,
        interfaces: vec!["throwable".to_owned()],
        methods: Vec::new(),
        properties: vec![RuntimeClassPropertyEntry {
            name: "message".to_owned(),
            default: Value::String(PhpString::from_test_str(&message)),
            type_: Some(RuntimeType::String),
            flags: RuntimeClassPropertyFlags::default(),
            hooks: RuntimeClassPropertyHooks::default(),
            attributes: Vec::new(),
        }],
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: RuntimeClassFlags::default(),
    };
    Ok(ObjectRef::new(&class))
}

fn internal_throwable_method_value(
    object: &ObjectRef,
    method: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    if normalize_method_name(method) != "getmessage" {
        return Err(format!(
            "E_PHP_VM_UNKNOWN_METHOD: method {}::{method} is not declared",
            object.class_name()
        ));
    }
    if !args.is_empty() {
        return Err(format!(
            "E_PHP_VM_TOO_MANY_ARGS: {}::getMessage() expects exactly 0 arguments, {} given",
            object.class_name(),
            args.len()
        ));
    }
    Ok(object.get_property("message").unwrap_or(Value::Null))
}

fn handle_throw(
    compiled: &CompiledUnit,
    value: Value,
    handlers: &mut Vec<ExceptionHandler>,
    stack: &mut CallStack,
    pending_control: &mut Option<PendingControl>,
) -> Option<BlockId> {
    while let Some(handler) = handlers.pop() {
        if let Some(catch) = handler.catch
            && catch_matches(compiled, &value, &handler.catch_types).unwrap_or(false)
        {
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
    }
    None
}

fn catch_matches(
    compiled: &CompiledUnit,
    value: &Value,
    catch_types: &[String],
) -> Result<bool, String> {
    if catch_types.is_empty() {
        return Ok(true);
    }
    for catch_type in catch_types {
        if object_instanceof(compiled, value, catch_type)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn uncaught_exception(
    output: &OutputBuffer,
    compiled: &CompiledUnit,
    stack: &CallStack,
    value: Value,
) -> VmResult {
    let message = match &value {
        Value::Object(object) => object
            .get_property("message")
            .and_then(|value| to_string(&value).ok())
            .map(|value| value.to_string_lossy())
            .unwrap_or_default(),
        other => format!("uncaught {}", value_type_name(other)),
    };
    let full = if message.is_empty() {
        format!(
            "E_PHP_VM_UNCAUGHT_EXCEPTION: Uncaught {}",
            throwable_class_name(&value)
        )
    } else {
        format!(
            "E_PHP_VM_UNCAUGHT_EXCEPTION: Uncaught {}: {message}",
            throwable_class_name(&value)
        )
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
    compiled: &CompiledUnit,
    class: &php_ir::module::ClassEntry,
) -> Result<RuntimeClassEntry, String> {
    let mut lineage = Vec::new();
    collect_class_lineage(compiled, class, &mut lineage)?;
    let mut properties = Vec::new();
    let mut constants = Vec::new();
    for class in lineage {
        push_runtime_properties(compiled.unit(), class, &mut properties)?;
        push_runtime_constants(compiled.unit(), class, &mut constants)?;
    }
    Ok(RuntimeClassEntry {
        name: class.name.clone(),
        parent: class.parent.clone(),
        interfaces: class.interfaces.clone(),
        methods: class
            .methods
            .iter()
            .map(|method| {
                Ok(RuntimeClassMethodEntry {
                    name: method.name.clone(),
                    origin_class: method.origin_class.clone(),
                    function_id: method.function.raw(),
                    flags: RuntimeClassMethodFlags {
                        is_static: method.flags.is_static,
                        is_private: method.flags.is_private,
                        is_protected: method.flags.is_protected,
                        is_abstract: method.flags.is_abstract,
                        is_final: method.flags.is_final,
                    },
                    attributes: runtime_attributes(compiled.unit(), &method.attributes)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        properties,
        constants,
        enum_cases: push_runtime_enum_cases(compiled.unit(), class)?,
        attributes: runtime_attributes(compiled.unit(), &class.attributes)?,
        enum_backing_type: class.enum_backing_type.map(|backing| match backing {
            php_ir::module::ClassEnumBackingType::Int => RuntimeClassEnumBackingType::Int,
            php_ir::module::ClassEnumBackingType::String => RuntimeClassEnumBackingType::String,
        }),
        constructor_id: class.constructor.map(|function| function.raw()),
        flags: RuntimeClassFlags {
            is_abstract: class.flags.is_abstract,
            is_final: class.flags.is_final,
            is_readonly: class.flags.is_readonly,
            is_interface: class.flags.is_interface,
            is_enum: class.flags.is_enum,
        },
    })
}

fn collect_class_lineage<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    lineage: &mut Vec<&'a php_ir::module::ClassEntry>,
) -> Result<(), String> {
    collect_class_lineage_inner(compiled, class, lineage, &mut Vec::new())
}

fn collect_class_lineage_inner<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    lineage: &mut Vec<&'a php_ir::module::ClassEntry>,
    seen: &mut Vec<String>,
) -> Result<(), String> {
    let normalized = normalize_class_name(&class.name);
    if seen.iter().any(|name| name == &normalized) {
        return Err(format!(
            "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
            class.name
        ));
    }
    seen.push(normalized);
    if let Some(parent_name) = class.parent.as_deref() {
        let Some(parent) = compiled.lookup_class(parent_name) else {
            return Err(format!(
                "E_PHP_VM_UNKNOWN_PARENT_CLASS: class {} extends missing class {}",
                class.name, parent_name
            ));
        };
        collect_class_lineage_inner(compiled, parent, lineage, seen)?;
    }
    lineage.push(class);
    seen.pop();
    Ok(())
}

fn push_runtime_properties(
    unit: &IrUnit,
    class: &php_ir::module::ClassEntry,
    properties: &mut Vec<RuntimeClassPropertyEntry>,
) -> Result<(), String> {
    for property in &class.properties {
        if (property.hooks.get.is_some() || property.hooks.set.is_some())
            && !property.hooks.backed
            && !property.flags.is_static
        {
            properties.push(RuntimeClassPropertyEntry {
                name: property.name.clone(),
                default: Value::Uninitialized,
                type_: ir_runtime_type(property.type_.as_ref()),
                flags: RuntimeClassPropertyFlags {
                    is_static: property.flags.is_static,
                    is_private: property.flags.is_private,
                    is_protected: property.flags.is_protected,
                    set_is_private: property.flags.set_is_private,
                    set_is_protected: property.flags.set_is_protected,
                    is_readonly: property.flags.is_readonly,
                    is_typed: property.flags.is_typed,
                },
                hooks: RuntimeClassPropertyHooks {
                    get_function_id: property.hooks.get.map(|id| id.index() as u32),
                    set_function_id: property.hooks.set.map(|id| id.index() as u32),
                    backed: false,
                },
                attributes: runtime_attributes(unit, &property.attributes)?,
            });
            continue;
        }
        let default = if let Some(default) = property.default {
            constant_value(unit, default)?
        } else if property.flags.is_typed {
            Value::Uninitialized
        } else {
            Value::Null
        };
        properties.push(RuntimeClassPropertyEntry {
            name: property_storage_name(class, property),
            default,
            type_: ir_runtime_type(property.type_.as_ref()),
            flags: RuntimeClassPropertyFlags {
                is_static: property.flags.is_static,
                is_private: property.flags.is_private,
                is_protected: property.flags.is_protected,
                set_is_private: property.flags.set_is_private,
                set_is_protected: property.flags.set_is_protected,
                is_readonly: property.flags.is_readonly,
                is_typed: property.flags.is_typed,
            },
            hooks: RuntimeClassPropertyHooks {
                get_function_id: property.hooks.get.map(|id| id.index() as u32),
                set_function_id: property.hooks.set.map(|id| id.index() as u32),
                backed: property.hooks.backed,
            },
            attributes: runtime_attributes(unit, &property.attributes)?,
        });
    }
    Ok(())
}

fn push_runtime_constants(
    unit: &IrUnit,
    class: &php_ir::module::ClassEntry,
    constants: &mut Vec<RuntimeClassConstantEntry>,
) -> Result<(), String> {
    for constant in &class.constants {
        let value = if let Some(value) = constant.value {
            constant_value(unit, value)?
        } else {
            Value::Null
        };
        constants.push(RuntimeClassConstantEntry {
            name: constant.name.clone(),
            value,
            flags: RuntimeClassConstantFlags {
                is_private: constant.flags.is_private,
                is_protected: constant.flags.is_protected,
            },
            attributes: runtime_attributes(unit, &constant.attributes)?,
        });
    }
    Ok(())
}

fn push_runtime_enum_cases(
    unit: &IrUnit,
    class: &php_ir::module::ClassEntry,
) -> Result<Vec<RuntimeClassEnumCaseEntry>, String> {
    class
        .enum_cases
        .iter()
        .map(|case| {
            Ok(RuntimeClassEnumCaseEntry {
                name: case.name.clone(),
                value: case
                    .value
                    .map(|value| constant_value(unit, value))
                    .transpose()?,
                attributes: runtime_attributes(unit, &case.attributes)?,
            })
        })
        .collect()
}

fn runtime_attributes(
    unit: &IrUnit,
    attributes: &[php_ir::module::AttributeEntry],
) -> Result<Vec<RuntimeAttributeEntry>, String> {
    attributes
        .iter()
        .map(|attribute| {
            let arguments = attribute
                .arguments
                .iter()
                .map(|argument| constant_value(unit, *argument))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(RuntimeAttributeEntry {
                name: attribute.name.clone(),
                resolved_name: attribute.resolved_name.clone(),
                fallback_name: attribute.fallback_name.clone(),
                arguments,
                repeated_on_target: attribute.repeated_on_target,
                span: Some((
                    attribute.span.file.raw(),
                    attribute.span.start,
                    attribute.span.end,
                )),
            })
        })
        .collect()
}

fn is_reflection_runtime_class(name: &str) -> bool {
    let name = normalize_class_name(name);
    [
        "reflectionclass",
        "reflectionfunction",
        "reflectionmethod",
        "reflectionproperty",
        "reflectionclassconstant",
        "reflectionenum",
        "reflectionenumunitcase",
        "reflectionparameter",
        "reflectionattribute",
        "reflectionnamedtype",
    ]
    .contains(&name.as_str())
}

fn normalize_function_name(name: &str) -> String {
    name.trim_start_matches('\\').to_ascii_lowercase()
}

fn reflection_runtime_class(name: &str) -> RuntimeClassEntry {
    RuntimeClassEntry {
        name: name.to_owned(),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: RuntimeClassFlags::default(),
    }
}

fn reflection_object(name: &str, properties: Vec<(&str, Value)>) -> ObjectRef {
    let class = reflection_runtime_class(name);
    let object = ObjectRef::new(&class);
    for (property, value) in properties {
        object.set_property(property, value);
    }
    object
}

fn reflection_string_arg(args: &[Value], index: usize, owner: &str) -> Result<String, String> {
    let Some(value) = args.get(index) else {
        return Err(format!(
            "E_PHP_VM_REFLECTION_ARITY: {owner} missing argument {index}"
        ));
    };
    Ok(to_string(value)?.to_string_lossy())
}

fn reflection_string(value: impl AsRef<str>) -> Value {
    Value::String(PhpString::from_test_str(value.as_ref()))
}

fn reflection_string_array(values: impl IntoIterator<Item = String>) -> Value {
    let mut array = PhpArray::new();
    for value in values {
        array.append(reflection_string(value));
    }
    Value::Array(array)
}

fn reflection_objects_array(values: impl IntoIterator<Item = ObjectRef>) -> Value {
    let mut array = PhpArray::new();
    for value in values {
        array.append(Value::Object(value));
    }
    Value::Array(array)
}

fn reflection_assoc_insert(array: &mut PhpArray, key: &str, value: Value) {
    array.insert(ArrayKey::String(PhpString::from_test_str(key)), value);
}

fn reflection_type_value(type_: Option<&IrReturnType>) -> Value {
    let Some(type_) = type_ else {
        return Value::Null;
    };
    Value::Object(reflection_object(
        "ReflectionNamedType",
        vec![
            ("name", reflection_string(ir_type_name(type_))),
            ("allows_null", Value::Bool(ir_type_allows_null(type_))),
            ("builtin", Value::Bool(ir_type_is_builtin(type_))),
        ],
    ))
}

fn ir_type_name(type_: &IrReturnType) -> String {
    match type_ {
        IrReturnType::Int => "int".to_owned(),
        IrReturnType::Float => "float".to_owned(),
        IrReturnType::String => "string".to_owned(),
        IrReturnType::Array => "array".to_owned(),
        IrReturnType::Callable => "callable".to_owned(),
        IrReturnType::Iterable => "iterable".to_owned(),
        IrReturnType::Object => "object".to_owned(),
        IrReturnType::Bool => "bool".to_owned(),
        IrReturnType::Null => "null".to_owned(),
        IrReturnType::Void => "void".to_owned(),
        IrReturnType::Mixed => "mixed".to_owned(),
        IrReturnType::Never => "never".to_owned(),
        IrReturnType::False => "false".to_owned(),
        IrReturnType::True => "true".to_owned(),
        IrReturnType::Class { name } => name.clone(),
        IrReturnType::Nullable { inner } => ir_type_name(inner),
        IrReturnType::Union { members } => members
            .iter()
            .map(ir_type_name)
            .collect::<Vec<_>>()
            .join("|"),
        IrReturnType::Intersection { members } => members
            .iter()
            .map(ir_type_name)
            .collect::<Vec<_>>()
            .join("&"),
        IrReturnType::Dnf { members } => members
            .iter()
            .map(ir_type_name)
            .collect::<Vec<_>>()
            .join("|"),
    }
}

fn ir_type_allows_null(type_: &IrReturnType) -> bool {
    match type_ {
        IrReturnType::Null | IrReturnType::Mixed => true,
        IrReturnType::Nullable { .. } => true,
        IrReturnType::Union { members } | IrReturnType::Dnf { members } => {
            members.iter().any(ir_type_allows_null)
        }
        _ => false,
    }
}

fn ir_type_is_builtin(type_: &IrReturnType) -> bool {
    !matches!(type_, IrReturnType::Class { .. })
}

fn reflection_span_file(compiled: &CompiledUnit, span: php_ir::source_map::IrSpan) -> Value {
    compiled
        .unit()
        .files
        .get(span.file.index())
        .map(|file| reflection_string(&file.path))
        .unwrap_or(Value::Bool(false))
}

fn reflection_span_line(
    compiled: &CompiledUnit,
    span: php_ir::source_map::IrSpan,
    end: bool,
) -> Value {
    let Some(file) = compiled.unit().files.get(span.file.index()) else {
        return Value::Bool(false);
    };
    let Ok(source) = std::fs::read_to_string(&file.path) else {
        return Value::Bool(false);
    };
    let offset = if end { span.end } else { span.start } as usize;
    let offset = offset.min(source.len());
    let line = source.as_bytes()[..offset]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1;
    Value::Int(line as i64)
}

fn reflection_bool_property(object: &ObjectRef, property: &str) -> Value {
    object.get_property(property).unwrap_or(Value::Bool(false))
}

fn reflection_class_object(compiled: &CompiledUnit, class_name: &str) -> Result<ObjectRef, String> {
    let target = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    Ok(reflection_object(
        "ReflectionClass",
        vec![
            (
                "name",
                Value::String(PhpString::from_test_str(&target.display_name)),
            ),
            (
                "class",
                Value::String(PhpString::from_test_str(&target.name)),
            ),
            (
                "attributes",
                reflection_attributes_value(compiled, &target.attributes)?,
            ),
            ("is_interface", Value::Bool(target.flags.is_interface)),
            ("is_trait", Value::Bool(false)),
            ("is_enum", Value::Bool(target.flags.is_enum)),
            ("is_abstract", Value::Bool(target.flags.is_abstract)),
            ("is_final", Value::Bool(target.flags.is_final)),
            (
                "interfaces",
                reflection_string_array(target.interfaces.iter().map(|interface| {
                    compiled
                        .lookup_class(interface)
                        .map(|entry| entry.display_name.clone())
                        .unwrap_or_else(|| interface.clone())
                })),
            ),
            ("file", reflection_span_file(compiled, target.span)),
            (
                "start_line",
                reflection_span_line(compiled, target.span, false),
            ),
            (
                "end_line",
                reflection_span_line(compiled, target.span, true),
            ),
        ],
    ))
}

fn reflection_new_object(
    compiled: &CompiledUnit,
    class_name: &str,
    args: Vec<Value>,
) -> Result<ObjectRef, String> {
    match normalize_class_name(class_name).as_str() {
        "reflectionclass" => {
            let class = reflection_string_arg(&args, 0, "ReflectionClass::__construct")?;
            reflection_class_object(compiled, &class)
        }
        "reflectionfunction" => {
            let Some(target) = args.first() else {
                return Err(
                    "E_PHP_VM_REFLECTION_ARITY: ReflectionFunction::__construct missing argument 0"
                        .to_owned(),
                );
            };
            match target {
                Value::Callable(CallableValue::UserFunction { name }) => {
                    let function_id =
                        compiled
                            .lookup_function(&normalize_function_name(name))
                            .ok_or_else(|| {
                                format!(
                                    "E_PHP_VM_REFLECTION_UNKNOWN_FUNCTION: function {name} is not defined"
                                )
                            })?;
                    let function_entry = &compiled.unit().functions[function_id.index()];
                    Ok(reflection_function_object(compiled, function_entry)?)
                }
                Value::Callable(CallableValue::Closure { function, captures }) => {
                    let function = FunctionId::new(*function);
                    let function_entry = compiled.unit().functions.get(function.index()).ok_or_else(|| {
                        format!(
                            "E_PHP_VM_REFLECTION_UNKNOWN_FUNCTION: closure function {} is not defined",
                            function.raw()
                        )
                    })?;
                    Ok(reflection_closure_object(compiled, function_entry, captures)?)
                }
                Value::Callable(_) => Err(
                    "E_PHP_VM_REFLECTION_UNSUPPORTED_CALLABLE: callable reflection supports user functions and closures in the Prompt 28 MVP"
                        .to_owned(),
                ),
                _ => {
                    let function = reflection_string_arg(&args, 0, "ReflectionFunction::__construct")?;
                    let function_id = compiled
                        .lookup_function(&normalize_function_name(&function))
                        .ok_or_else(|| {
                            format!(
                                "E_PHP_VM_REFLECTION_UNKNOWN_FUNCTION: function {function} is not defined"
                            )
                        })?;
                    let function_entry = &compiled.unit().functions[function_id.index()];
                    Ok(reflection_function_object(compiled, function_entry)?)
                }
            }
        }
        "reflectionmethod" => {
            let class = reflection_string_arg(&args, 0, "ReflectionMethod::__construct")?;
            let method = reflection_string_arg(&args, 1, "ReflectionMethod::__construct")?;
            reflection_method_object(compiled, &class, &method)
        }
        "reflectionproperty" => {
            let class = reflection_string_arg(&args, 0, "ReflectionProperty::__construct")?;
            let property = reflection_string_arg(&args, 1, "ReflectionProperty::__construct")?;
            reflection_property_object(compiled, &class, &property)
        }
        "reflectionclassconstant" => {
            let class = reflection_string_arg(&args, 0, "ReflectionClassConstant::__construct")?;
            let constant = reflection_string_arg(&args, 1, "ReflectionClassConstant::__construct")?;
            reflection_class_constant_object(compiled, &class, &constant)
        }
        "reflectionenum" => {
            let class = reflection_string_arg(&args, 0, "ReflectionEnum::__construct")?;
            reflection_enum_object(compiled, &class)
        }
        "reflectionenumunitcase" => {
            let class = reflection_string_arg(&args, 0, "ReflectionEnumUnitCase::__construct")?;
            let case = reflection_string_arg(&args, 1, "ReflectionEnumUnitCase::__construct")?;
            reflection_enum_case_object(compiled, &class, &case)
        }
        "reflectionattribute" => Err(
            "E_PHP_VM_REFLECTION_ATTRIBUTE_CONSTRUCTION: ReflectionAttribute is created by getAttributes"
                .to_owned(),
        ),
        "reflectionparameter" => Err(
            "E_PHP_VM_REFLECTION_PARAMETER_CONSTRUCTION: ReflectionParameter direct construction is outside the Prompt 27 MVP"
                .to_owned(),
        ),
        "reflectionnamedtype" => Err(
            "E_PHP_VM_REFLECTION_NAMED_TYPE_CONSTRUCTION: ReflectionNamedType is created by metadata accessors"
                .to_owned(),
        ),
        _ => Err(format!(
            "E_PHP_VM_REFLECTION_UNKNOWN_CLASS: unsupported reflection class {class_name}"
        )),
    }
}

fn reflection_method_value(
    compiled: &CompiledUnit,
    object: &ObjectRef,
    method: &str,
    args: Vec<Value>,
) -> Result<Value, String> {
    let method = normalize_method_name(method);
    match normalize_class_name(&object.class_name()).as_str() {
        "reflectionattribute" => match method.as_str() {
            "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "getarguments" => Ok(object
                .get_property("arguments")
                .unwrap_or_else(empty_array_value)),
            "newinstance" => Err(
                "E_PHP_RUNTIME_UNSUPPORTED_ATTRIBUTE_NEWINSTANCE: attribute instantiation needs class constructor semantics"
                    .to_owned(),
            ),
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        "reflectionnamedtype" => match method.as_str() {
            "getname" | "__tostring" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "allowsnull" => Ok(reflection_bool_property(object, "allows_null")),
            "isbuiltin" => Ok(reflection_bool_property(object, "builtin")),
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        "reflectionclass" => match method.as_str() {
            "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "getattributes" => Ok(object
                .get_property("attributes")
                .unwrap_or_else(empty_array_value)),
            "isinterface" => Ok(reflection_bool_property(object, "is_interface")),
            "istrait" => Ok(reflection_bool_property(object, "is_trait")),
            "isenum" => Ok(reflection_bool_property(object, "is_enum")),
            "isabstract" => Ok(reflection_bool_property(object, "is_abstract")),
            "isfinal" => Ok(reflection_bool_property(object, "is_final")),
            "isinstantiable" => Ok(Value::Bool(
                !matches!(object.get_property("is_interface"), Some(Value::Bool(true)))
                    && !matches!(object.get_property("is_abstract"), Some(Value::Bool(true))),
            )),
            "getinterfacenames" => Ok(object
                .get_property("interfaces")
                .unwrap_or_else(empty_array_value)),
            "getfilename" => Ok(object.get_property("file").unwrap_or(Value::Bool(false))),
            "getstartline" => Ok(object.get_property("start_line").unwrap_or(Value::Bool(false))),
            "getendline" => Ok(object.get_property("end_line").unwrap_or(Value::Bool(false))),
            "getdoccomment" => Ok(Value::Bool(false)),
            "getmethods" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(reflection_class_methods_value(compiled, &class)?)
            }
            "getproperties" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(reflection_class_properties_value(compiled, &class)?)
            }
            "getconstants" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(reflection_class_constants_value(compiled, &class)?)
            }
            "getconstant" => {
                let class = reflection_object_string_property(object, "class")?;
                let constant = reflection_string_arg(&args, 0, "ReflectionClass::getConstant")?;
                let constants = reflection_class_constants_value(compiled, &class)?;
                if let Value::Array(constants) = constants {
                    return Ok(constants
                        .get(&ArrayKey::String(PhpString::from_test_str(&constant)))
                        .cloned()
                        .unwrap_or(Value::Bool(false)));
                }
                Ok(Value::Bool(false))
            }
            "getreflectionconstants" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(reflection_class_reflection_constants_value(compiled, &class)?)
            }
            "getmethod" => {
                let class = reflection_object_string_property(object, "class")?;
                let method = reflection_string_arg(&args, 0, "ReflectionClass::getMethod")?;
                Ok(Value::Object(reflection_method_object(compiled, &class, &method)?))
            }
            "getproperty" => {
                let class = reflection_object_string_property(object, "class")?;
                let property = reflection_string_arg(&args, 0, "ReflectionClass::getProperty")?;
                Ok(Value::Object(reflection_property_object(
                    compiled, &class, &property,
                )?))
            }
            "getreflectionconstant" => {
                let class = reflection_object_string_property(object, "class")?;
                let constant =
                    reflection_string_arg(&args, 0, "ReflectionClass::getReflectionConstant")?;
                Ok(Value::Object(reflection_class_constant_object(
                    compiled, &class, &constant,
                )?))
            }
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        "reflectionfunction" | "reflectionmethod" => match method.as_str() {
            "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "getattributes" => Ok(object
                .get_property("attributes")
                .unwrap_or_else(empty_array_value)),
            "getparameters" => Ok(object
                .get_property("parameters")
                .unwrap_or_else(empty_array_value)),
            "getnumberofparameters" => Ok(object
                .get_property("parameter_count")
                .unwrap_or(Value::Int(0))),
            "getnumberofrequiredparameters" => Ok(object
                .get_property("required_parameter_count")
                .unwrap_or(Value::Int(0))),
            "getreturntype" => Ok(object.get_property("return_type").unwrap_or(Value::Null)),
            "getfilename" => Ok(object.get_property("file").unwrap_or(Value::Bool(false))),
            "getstartline" => Ok(object.get_property("start_line").unwrap_or(Value::Bool(false))),
            "getendline" => Ok(object.get_property("end_line").unwrap_or(Value::Bool(false))),
            "getdoccomment" => Ok(Value::Bool(false)),
            "ispublic" => Ok(object.get_property("is_public").unwrap_or(Value::Bool(true))),
            "isprivate" => Ok(reflection_bool_property(object, "is_private")),
            "isprotected" => Ok(reflection_bool_property(object, "is_protected")),
            "isstatic" => Ok(reflection_bool_property(object, "is_static")),
            "isabstract" => Ok(reflection_bool_property(object, "is_abstract")),
            "isfinal" => Ok(reflection_bool_property(object, "is_final")),
            "isclosure" => Ok(reflection_bool_property(object, "is_closure")),
            "getstaticvariables" => Ok(object
                .get_property("static_variables")
                .unwrap_or_else(empty_array_value)),
            "getclosurescopeclass" => Ok(object
                .get_property("closure_scope_class")
                .unwrap_or(Value::Bool(false))),
            "getdeclaringclass" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(Value::Object(reflection_class_object(compiled, &class)?))
            }
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        "reflectionproperty" | "reflectionclassconstant" | "reflectionenumunitcase" => {
            match method.as_str() {
                "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
                "getattributes" => Ok(object
                    .get_property("attributes")
                    .unwrap_or_else(empty_array_value)),
                "getdeclaringclass" => {
                    let class = reflection_object_string_property(object, "class")?;
                    Ok(Value::Object(reflection_class_object(compiled, &class)?))
                }
                "gettype" => Ok(object.get_property("type").unwrap_or(Value::Null)),
                "hasdefaultvalue" => Ok(reflection_bool_property(object, "has_default")),
                "getdefaultvalue" | "getvalue" => {
                    Ok(object.get_property("default").unwrap_or(Value::Null))
                }
                "getbackingvalue" => Ok(object
                    .get_property("backing_value")
                    .unwrap_or(Value::Bool(false))),
                "ispublic" => Ok(object.get_property("is_public").unwrap_or(Value::Bool(true))),
                "isprivate" => Ok(reflection_bool_property(object, "is_private")),
                "isprotected" => Ok(reflection_bool_property(object, "is_protected")),
                "isstatic" => Ok(reflection_bool_property(object, "is_static")),
                "isreadonly" => Ok(reflection_bool_property(object, "is_readonly")),
                _ => Err(format!(
                    "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                    object.class_name(),
                    method
                )),
            }
        }
        "reflectionenum" => match method.as_str() {
            "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "getattributes" => Ok(object
                .get_property("attributes")
                .unwrap_or_else(empty_array_value)),
            "isbacked" => Ok(reflection_bool_property(object, "is_backed")),
            "getbackingtype" => Ok(object.get_property("backing_type").unwrap_or(Value::Null)),
            "getcases" => {
                let class = reflection_object_string_property(object, "class")?;
                Ok(reflection_enum_cases_value(compiled, &class)?)
            }
            "getfilename" => Ok(object.get_property("file").unwrap_or(Value::Bool(false))),
            "getstartline" => Ok(object.get_property("start_line").unwrap_or(Value::Bool(false))),
            "getendline" => Ok(object.get_property("end_line").unwrap_or(Value::Bool(false))),
            "getdoccomment" => Ok(Value::Bool(false)),
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        "reflectionparameter" => match method.as_str() {
            "getname" => Ok(object.get_property("name").unwrap_or(Value::Null)),
            "getattributes" => Ok(object
                .get_property("attributes")
                .unwrap_or_else(empty_array_value)),
            "gettype" => Ok(object.get_property("type").unwrap_or(Value::Null)),
            "hasdefaultvalue" | "isdefaultvalueavailable" => {
                Ok(reflection_bool_property(object, "has_default"))
            }
            "getdefaultvalue" => Ok(object.get_property("default").unwrap_or(Value::Null)),
            "isoptional" => Ok(reflection_bool_property(object, "optional")),
            "allowsnull" => Ok(reflection_bool_property(object, "allows_null")),
            _ => Err(format!(
                "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
                object.class_name(),
                method
            )),
        },
        _ => Err(format!(
            "E_PHP_VM_UNKNOWN_METHOD: method {}::{} is not defined",
            object.class_name(),
            method
        )),
    }
}

fn reflection_class_methods_value(
    compiled: &CompiledUnit,
    class_name: &str,
) -> Result<Value, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let methods = class
        .methods
        .iter()
        .map(|method| reflection_method_object(compiled, &class.name, &method.name))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(reflection_objects_array(methods))
}

fn reflection_class_properties_value(
    compiled: &CompiledUnit,
    class_name: &str,
) -> Result<Value, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let properties = class
        .properties
        .iter()
        .map(|property| reflection_property_object(compiled, &class.name, &property.name))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(reflection_objects_array(properties))
}

fn reflection_class_constants_value(
    compiled: &CompiledUnit,
    class_name: &str,
) -> Result<Value, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let mut array = PhpArray::new();
    for constant in &class.constants {
        let value = constant
            .value
            .map(|constant| constant_value(compiled.unit(), constant))
            .transpose()?
            .unwrap_or(Value::Null);
        reflection_assoc_insert(&mut array, &constant.name, value);
    }
    Ok(Value::Array(array))
}

fn reflection_class_reflection_constants_value(
    compiled: &CompiledUnit,
    class_name: &str,
) -> Result<Value, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let constants = class
        .constants
        .iter()
        .map(|constant| reflection_class_constant_object(compiled, &class.name, &constant.name))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(reflection_objects_array(constants))
}

fn reflection_enum_cases_value(compiled: &CompiledUnit, class_name: &str) -> Result<Value, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let cases = class
        .enum_cases
        .iter()
        .map(|case| reflection_enum_case_object(compiled, &class.name, &case.name))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(reflection_objects_array(cases))
}

fn reflection_enum_backing_type_value(
    backing_type: Option<php_ir::module::ClassEnumBackingType>,
) -> Value {
    let Some(backing_type) = backing_type else {
        return Value::Null;
    };
    let name = match backing_type {
        php_ir::module::ClassEnumBackingType::Int => "int",
        php_ir::module::ClassEnumBackingType::String => "string",
    };
    Value::Object(reflection_object(
        "ReflectionNamedType",
        vec![
            ("name", reflection_string(name)),
            ("allows_null", Value::Bool(false)),
            ("builtin", Value::Bool(true)),
        ],
    ))
}

fn reflection_static_variables_value(captures: &[ClosureCaptureValue]) -> Value {
    let mut array = PhpArray::new();
    for capture in captures {
        let value = capture
            .value()
            .cloned()
            .or_else(|| capture.reference().map(|reference| reference.get()))
            .unwrap_or(Value::Null);
        reflection_assoc_insert(&mut array, &capture.name, value);
    }
    Value::Array(array)
}

fn reflection_function_object(
    compiled: &CompiledUnit,
    function: &IrFunction,
) -> Result<ObjectRef, String> {
    let parameters = reflection_parameters_value(compiled, &function.params)?;
    let parameter_count = function.params.len() as i64;
    let required_parameter_count = function
        .params
        .iter()
        .filter(|param| param.required)
        .count() as i64;
    Ok(reflection_object(
        "ReflectionFunction",
        vec![
            (
                "name",
                Value::String(PhpString::from_test_str(&function.name)),
            ),
            (
                "attributes",
                reflection_attributes_value(compiled, &function.attributes)?,
            ),
            ("parameters", parameters),
            ("parameter_count", Value::Int(parameter_count)),
            (
                "required_parameter_count",
                Value::Int(required_parameter_count),
            ),
            (
                "return_type",
                reflection_type_value(function.return_type.as_ref()),
            ),
            ("file", reflection_span_file(compiled, function.span)),
            (
                "start_line",
                reflection_span_line(compiled, function.span, false),
            ),
            (
                "end_line",
                reflection_span_line(compiled, function.span, true),
            ),
            ("is_closure", Value::Bool(function.flags.is_closure)),
            ("static_variables", empty_array_value()),
            ("closure_scope_class", Value::Null),
        ],
    ))
}

fn reflection_closure_object(
    compiled: &CompiledUnit,
    function: &IrFunction,
    captures: &[ClosureCaptureValue],
) -> Result<ObjectRef, String> {
    let object = reflection_function_object(compiled, function)?;
    object.set_property("name", reflection_string("{closure}"));
    object.set_property("is_closure", Value::Bool(true));
    object.set_property(
        "static_variables",
        reflection_static_variables_value(captures),
    );
    object.set_property("closure_scope_class", Value::Null);
    Ok(object)
}

fn reflection_method_object(
    compiled: &CompiledUnit,
    class_name: &str,
    method_name: &str,
) -> Result<ObjectRef, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let method = class
        .methods
        .iter()
        .find(|method| method.name == normalize_method_name(method_name))
        .ok_or_else(|| {
            format!(
                "E_PHP_VM_REFLECTION_UNKNOWN_METHOD: method {}::{} is not defined",
                class.name, method_name
            )
        })?;
    let function = &compiled.unit().functions[method.function.index()];
    let parameters = reflection_parameters_value(compiled, &function.params)?;
    let parameter_count = function.params.len() as i64;
    let required_parameter_count = function
        .params
        .iter()
        .filter(|param| param.required)
        .count() as i64;
    Ok(reflection_object(
        "ReflectionMethod",
        vec![
            (
                "class",
                Value::String(PhpString::from_test_str(&class.name)),
            ),
            ("name", Value::String(PhpString::from_test_str(method_name))),
            (
                "attributes",
                reflection_attributes_value(compiled, &method.attributes)?,
            ),
            ("parameters", parameters),
            ("parameter_count", Value::Int(parameter_count)),
            (
                "required_parameter_count",
                Value::Int(required_parameter_count),
            ),
            (
                "return_type",
                reflection_type_value(function.return_type.as_ref()),
            ),
            ("file", reflection_span_file(compiled, function.span)),
            (
                "start_line",
                reflection_span_line(compiled, function.span, false),
            ),
            (
                "end_line",
                reflection_span_line(compiled, function.span, true),
            ),
            (
                "is_public",
                Value::Bool(!method.flags.is_private && !method.flags.is_protected),
            ),
            ("is_private", Value::Bool(method.flags.is_private)),
            ("is_protected", Value::Bool(method.flags.is_protected)),
            ("is_static", Value::Bool(method.flags.is_static)),
            ("is_abstract", Value::Bool(method.flags.is_abstract)),
            ("is_final", Value::Bool(method.flags.is_final)),
        ],
    ))
}

fn reflection_property_object(
    compiled: &CompiledUnit,
    class_name: &str,
    property_name: &str,
) -> Result<ObjectRef, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let property = class
        .properties
        .iter()
        .find(|property| property.name == property_name)
        .ok_or_else(|| {
            format!(
                "E_PHP_VM_REFLECTION_UNKNOWN_PROPERTY: property {}::{} is not defined",
                class.name, property_name
            )
        })?;
    let default = property
        .default
        .map(|constant| constant_value(compiled.unit(), constant))
        .transpose()?;
    Ok(reflection_object(
        "ReflectionProperty",
        vec![
            (
                "class",
                Value::String(PhpString::from_test_str(&class.name)),
            ),
            (
                "name",
                Value::String(PhpString::from_test_str(property_name)),
            ),
            (
                "attributes",
                reflection_attributes_value(compiled, &property.attributes)?,
            ),
            ("type", reflection_type_value(property.type_.as_ref())),
            ("has_default", Value::Bool(default.is_some())),
            ("default", default.unwrap_or(Value::Null)),
            (
                "is_public",
                Value::Bool(!property.flags.is_private && !property.flags.is_protected),
            ),
            ("is_private", Value::Bool(property.flags.is_private)),
            ("is_protected", Value::Bool(property.flags.is_protected)),
            ("is_static", Value::Bool(property.flags.is_static)),
            ("is_readonly", Value::Bool(property.flags.is_readonly)),
        ],
    ))
}

fn reflection_class_constant_object(
    compiled: &CompiledUnit,
    class_name: &str,
    constant_name: &str,
) -> Result<ObjectRef, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let constant = class
        .constants
        .iter()
        .find(|constant| constant.name == constant_name)
        .ok_or_else(|| {
            format!(
                "E_PHP_VM_REFLECTION_UNKNOWN_CONSTANT: constant {}::{} is not defined",
                class.name, constant_name
            )
        })?;
    let value = constant
        .value
        .map(|constant| constant_value(compiled.unit(), constant))
        .transpose()?;
    Ok(reflection_object(
        "ReflectionClassConstant",
        vec![
            (
                "class",
                Value::String(PhpString::from_test_str(&class.name)),
            ),
            (
                "name",
                Value::String(PhpString::from_test_str(constant_name)),
            ),
            (
                "attributes",
                reflection_attributes_value(compiled, &constant.attributes)?,
            ),
            ("has_default", Value::Bool(value.is_some())),
            ("default", value.unwrap_or(Value::Null)),
            (
                "is_public",
                Value::Bool(!constant.flags.is_private && !constant.flags.is_protected),
            ),
            ("is_private", Value::Bool(constant.flags.is_private)),
            ("is_protected", Value::Bool(constant.flags.is_protected)),
            ("is_static", Value::Bool(true)),
        ],
    ))
}

fn reflection_enum_object(compiled: &CompiledUnit, class_name: &str) -> Result<ObjectRef, String> {
    let target = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    if !target.flags.is_enum {
        return Err(format!(
            "E_PHP_VM_REFLECTION_NOT_ENUM: class {class_name} is not an enum"
        ));
    }
    Ok(reflection_object(
        "ReflectionEnum",
        vec![
            (
                "name",
                Value::String(PhpString::from_test_str(&target.display_name)),
            ),
            (
                "class",
                Value::String(PhpString::from_test_str(&target.name)),
            ),
            (
                "attributes",
                reflection_attributes_value(compiled, &target.attributes)?,
            ),
            ("is_backed", Value::Bool(target.enum_backing_type.is_some())),
            (
                "backing_type",
                reflection_enum_backing_type_value(target.enum_backing_type),
            ),
            ("file", reflection_span_file(compiled, target.span)),
            (
                "start_line",
                reflection_span_line(compiled, target.span, false),
            ),
            (
                "end_line",
                reflection_span_line(compiled, target.span, true),
            ),
        ],
    ))
}

fn reflection_enum_case_object(
    compiled: &CompiledUnit,
    class_name: &str,
    case_name: &str,
) -> Result<ObjectRef, String> {
    let class = compiled.lookup_class(class_name).ok_or_else(|| {
        format!("E_PHP_VM_REFLECTION_UNKNOWN_CLASS: class {class_name} is not defined")
    })?;
    let case = class
        .enum_cases
        .iter()
        .find(|case| case.name == case_name)
        .ok_or_else(|| {
            format!(
                "E_PHP_VM_REFLECTION_UNKNOWN_ENUM_CASE: case {}::{} is not defined",
                class.name, case_name
            )
        })?;
    let backing_value = case
        .value
        .map(|constant| constant_value(compiled.unit(), constant))
        .transpose()?;
    Ok(reflection_object(
        "ReflectionEnumUnitCase",
        vec![
            (
                "class",
                Value::String(PhpString::from_test_str(&class.name)),
            ),
            ("name", Value::String(PhpString::from_test_str(case_name))),
            (
                "attributes",
                reflection_attributes_value(compiled, &case.attributes)?,
            ),
            ("backing_value", backing_value.unwrap_or(Value::Bool(false))),
        ],
    ))
}

fn reflection_parameters_value(
    compiled: &CompiledUnit,
    params: &[IrParam],
) -> Result<Value, String> {
    let mut array = PhpArray::new();
    for param in params {
        let default = param.default.as_ref().map(inline_constant_value);
        array.append(Value::Object(reflection_object(
            "ReflectionParameter",
            vec![
                ("name", Value::String(PhpString::from_test_str(&param.name))),
                (
                    "attributes",
                    reflection_attributes_value(compiled, &param.attributes)?,
                ),
                ("type", reflection_type_value(param.type_.as_ref())),
                ("has_default", Value::Bool(default.is_some())),
                ("default", default.unwrap_or(Value::Null)),
                ("optional", Value::Bool(!param.required)),
                (
                    "allows_null",
                    Value::Bool(param.type_.as_ref().is_none_or(ir_type_allows_null)),
                ),
            ],
        )));
    }
    Ok(Value::Array(array))
}

fn reflection_attributes_value(
    compiled: &CompiledUnit,
    attributes: &[php_ir::module::AttributeEntry],
) -> Result<Value, String> {
    let mut array = PhpArray::new();
    for attribute in runtime_attributes(compiled.unit(), attributes)? {
        array.append(Value::Object(reflection_attribute_object(attribute)));
    }
    Ok(Value::Array(array))
}

fn reflection_attribute_object(attribute: RuntimeAttributeEntry) -> ObjectRef {
    let mut arguments = PhpArray::new();
    for argument in attribute.arguments {
        arguments.append(argument);
    }
    let name = attribute.name;
    reflection_object(
        "ReflectionAttribute",
        vec![
            ("name", Value::String(PhpString::from_test_str(&name))),
            ("arguments", Value::Array(arguments)),
        ],
    )
}

fn reflection_object_string_property(object: &ObjectRef, property: &str) -> Result<String, String> {
    let Some(value) = object.get_property(property) else {
        return Err(format!(
            "E_PHP_VM_REFLECTION_METADATA_MISSING: {} missing property {property}",
            object.class_name()
        ));
    };
    Ok(to_string(&value)?.to_string_lossy())
}

fn empty_array_value() -> Value {
    Value::Array(PhpArray::new())
}

fn enum_case_object(
    compiled: &CompiledUnit,
    state: &mut ExecutionState,
    class: &php_ir::module::ClassEntry,
    case: &php_ir::module::ClassEnumCaseEntry,
) -> Result<ObjectRef, String> {
    let key = (
        normalize_class_name(&class.name),
        case.name.to_ascii_lowercase(),
    );
    if let Some(object) = state.enum_cases.get(&key) {
        return Ok(object.clone());
    }
    let runtime_class = runtime_class_entry(compiled, class)?;
    let object = ObjectRef::new(&runtime_class);
    object.set_property("name", Value::String(PhpString::from_test_str(&case.name)));
    if runtime_class.enum_backing_type.is_some() {
        let value = case
            .value
            .map(|value| constant_value(compiled.unit(), value))
            .transpose()?
            .ok_or_else(|| {
                format!(
                    "E_PHP_VM_ENUM_CASE_MISSING_VALUE: backed enum case {}::{} has no value",
                    class.name, case.name
                )
            })?;
        object.set_property("value", value);
    }
    state.enum_cases.insert(key, object.clone());
    Ok(object)
}

fn enum_static_method(
    compiled: &CompiledUnit,
    state: &mut ExecutionState,
    class: &php_ir::module::ClassEntry,
    method: &str,
    args: Vec<CallArgument>,
) -> Result<Value, String> {
    match normalize_method_name(method).as_str() {
        "cases" => {
            if !args.is_empty() {
                return Err(format!(
                    "E_PHP_VM_TOO_MANY_ARGS: enum {}::cases expects no arguments",
                    class.name
                ));
            }
            let mut array = PhpArray::new();
            for case in &class.enum_cases {
                array.append(Value::Object(enum_case_object(
                    compiled, state, class, case,
                )?));
            }
            Ok(Value::Array(array))
        }
        "from" | "tryfrom" => enum_backed_lookup(compiled, state, class, method, args),
        _ => unreachable!("enum_static_method called for non-enum method"),
    }
}

fn enum_backed_lookup(
    compiled: &CompiledUnit,
    state: &mut ExecutionState,
    class: &php_ir::module::ClassEntry,
    method: &str,
    args: Vec<CallArgument>,
) -> Result<Value, String> {
    let normalized_method = normalize_method_name(method);
    if args.len() != 1 {
        return Err(format!(
            "E_PHP_VM_ENUM_LOOKUP_ARITY: enum {}::{} expects exactly one argument",
            class.name, method
        ));
    }
    if class.enum_backing_type.is_none() {
        return Err(format!(
            "E_PHP_VM_ENUM_LOOKUP_ON_UNIT_ENUM: enum {} has no backing values",
            class.name
        ));
    }
    let needle = &args[0].value;
    for case in &class.enum_cases {
        let Some(value_id) = case.value else {
            continue;
        };
        let value = constant_value(compiled.unit(), value_id)?;
        if identical(&value, needle) {
            return Ok(Value::Object(enum_case_object(
                compiled, state, class, case,
            )?));
        }
    }
    if normalized_method == "tryfrom" {
        Ok(Value::Null)
    } else {
        Err(format!(
            "E_PHP_VM_ENUM_VALUE_ERROR: value is not a valid backing value for enum {}",
            class.name
        ))
    }
}

fn callable_resolve_reference(value: Value) -> Value {
    match value {
        Value::Reference(cell) => callable_resolve_reference(cell.get()),
        value => value,
    }
}

fn callable_string_value(value: Value) -> Option<String> {
    match callable_resolve_reference(value) {
        Value::String(value) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn magic_args_array(args: Vec<CallArgument>) -> Value {
    let mut array = PhpArray::new();
    for arg in args {
        if let Some(name) = arg.name {
            array.insert(ArrayKey::String(PhpString::from_test_str(&name)), arg.value);
        } else {
            array.append(arg.value);
        }
    }
    Value::Array(array)
}

fn debug_info_gap_message(compiled: &CompiledUnit, values: &[Value]) -> Option<String> {
    for value in values {
        let value = match value {
            Value::Reference(cell) => cell.get(),
            value => value.clone(),
        };
        let Value::Object(object) = value else {
            continue;
        };
        let Some(class) = compiled.lookup_class(&object.class_name()) else {
            continue;
        };
        let Ok(Some(resolved)) = lookup_method_in_hierarchy(compiled, class, "__debugInfo", None)
        else {
            continue;
        };
        if !resolved.method.flags.is_static
            && !resolved.method.flags.is_private
            && !resolved.method.flags.is_protected
        {
            return Some(format!(
                "E_PHP_RUNTIME_UNSUPPORTED_DEBUGINFO: var_dump __debugInfo for {} is not implemented",
                object.class_name()
            ));
        }
    }
    None
}

fn validate_class_table(compiled: &CompiledUnit) -> Result<(), String> {
    for class in compiled.class_table() {
        if class.flags.is_final && class.flags.is_abstract {
            return Err(format!(
                "E_PHP_VM_INVALID_CLASS_MODIFIER: class {} cannot be both abstract and final",
                class.name
            ));
        }
        if class.flags.is_interface {
            for interface in &class.interfaces {
                let Some(parent) = compiled.lookup_class(interface) else {
                    return Err(format!(
                        "E_PHP_VM_UNKNOWN_INTERFACE: interface {} extends missing interface {}",
                        class.name, interface
                    ));
                };
                if !parent.flags.is_interface {
                    return Err(format!(
                        "E_PHP_VM_INTERFACE_EXTENDS_CLASS: interface {} cannot extend non-interface {}",
                        class.name, interface
                    ));
                }
            }
            continue;
        }

        if let Some(parent_name) = class.parent.as_deref() {
            let Some(parent) = compiled.lookup_class(parent_name) else {
                return Err(format!(
                    "E_PHP_VM_UNKNOWN_PARENT_CLASS: class {} extends missing class {}",
                    class.name, parent_name
                ));
            };
            if parent.flags.is_interface {
                return Err(format!(
                    "E_PHP_VM_CLASS_EXTENDS_INTERFACE: class {} cannot extend interface {}",
                    class.name, parent_name
                ));
            }
            if parent.flags.is_final {
                return Err(format!(
                    "E_PHP_VM_FINAL_CLASS_EXTEND: class {} cannot extend final class {}",
                    class.name, parent.name
                ));
            }
            validate_final_method_overrides(compiled, class, parent)?;
        }

        for interface in &class.interfaces {
            let Some(interface_class) = compiled.lookup_class(interface) else {
                return Err(format!(
                    "E_PHP_VM_UNKNOWN_INTERFACE: class {} implements missing interface {}",
                    class.name, interface
                ));
            };
            if !interface_class.flags.is_interface {
                return Err(format!(
                    "E_PHP_VM_IMPLEMENTS_NON_INTERFACE: class {} implements non-interface {}",
                    class.name, interface
                ));
            }
            validate_interface_implementation(compiled, class, interface_class)?;
        }

        if !class.flags.is_abstract {
            validate_no_unimplemented_abstract_methods(compiled, class)?;
        }
    }
    Ok(())
}

fn validate_final_method_overrides(
    compiled: &CompiledUnit,
    class: &php_ir::module::ClassEntry,
    parent: &php_ir::module::ClassEntry,
) -> Result<(), String> {
    for method in &class.methods {
        if let Some(parent_method) =
            lookup_method_in_hierarchy(compiled, parent, &method.name, None)?
            && parent_method.method.flags.is_final
        {
            return Err(format!(
                "E_PHP_VM_FINAL_METHOD_OVERRIDE: class {} cannot override final method {}::{}",
                class.name, parent_method.class.name, parent_method.method.name
            ));
        }
    }
    Ok(())
}

fn validate_no_unimplemented_abstract_methods(
    compiled: &CompiledUnit,
    class: &php_ir::module::ClassEntry,
) -> Result<(), String> {
    let mut lineage = Vec::new();
    collect_class_lineage(compiled, class, &mut lineage)?;
    for declaring in lineage {
        for method in &declaring.methods {
            if !method.flags.is_abstract {
                continue;
            }
            let resolved = lookup_method_in_hierarchy(compiled, class, &method.name, None)?
                .ok_or_else(|| {
                    format!(
                        "E_PHP_VM_ABSTRACT_METHOD_NOT_IMPLEMENTED: class {} does not implement {}::{}",
                        class.name, declaring.name, method.name
                    )
                })?;
            if resolved.method.flags.is_abstract {
                return Err(format!(
                    "E_PHP_VM_ABSTRACT_METHOD_NOT_IMPLEMENTED: class {} does not implement {}::{}",
                    class.name, declaring.name, method.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_interface_implementation(
    compiled: &CompiledUnit,
    class: &php_ir::module::ClassEntry,
    interface: &php_ir::module::ClassEntry,
) -> Result<(), String> {
    for parent_name in &interface.interfaces {
        let Some(parent) = compiled.lookup_class(parent_name) else {
            return Err(format!(
                "E_PHP_VM_UNKNOWN_INTERFACE: interface {} extends missing interface {}",
                interface.name, parent_name
            ));
        };
        validate_interface_implementation(compiled, class, parent)?;
    }
    for expected in &interface.methods {
        let resolved = lookup_method_in_hierarchy(compiled, class, &expected.name, None)?
            .ok_or_else(|| {
                format!(
                    "E_PHP_VM_INTERFACE_METHOD_MISSING: class {} must implement {}::{}",
                    class.name, interface.name, expected.name
                )
            })?;
        if resolved.method.flags.is_private || resolved.method.flags.is_protected {
            return Err(format!(
                "E_PHP_VM_INTERFACE_METHOD_VISIBILITY: class {} method {} must be public for interface {}",
                class.name, expected.name, interface.name
            ));
        }
        if !method_signature_compatible(compiled, expected, resolved.method) {
            return Err(format!(
                "E_PHP_VM_INTERFACE_METHOD_SIGNATURE: class {} method {} does not match interface {}",
                class.name, expected.name, interface.name
            ));
        }
    }
    Ok(())
}

fn method_signature_compatible(
    compiled: &CompiledUnit,
    expected: &php_ir::module::ClassMethodEntry,
    actual: &php_ir::module::ClassMethodEntry,
) -> bool {
    let Some(expected_fn) = compiled.unit().functions.get(expected.function.index()) else {
        return false;
    };
    let Some(actual_fn) = compiled.unit().functions.get(actual.function.index()) else {
        return false;
    };
    if expected_fn.returns_by_ref != actual_fn.returns_by_ref
        || expected_fn.return_type != actual_fn.return_type
    {
        return false;
    }
    if actual_fn.params.len() < expected_fn.params.len() {
        return false;
    }
    if actual_fn.params[expected_fn.params.len()..]
        .iter()
        .any(|param| param.required)
    {
        return false;
    }
    expected_fn
        .params
        .iter()
        .zip(&actual_fn.params)
        .all(|(expected_param, actual_param)| {
            (!actual_param.required || expected_param.required)
                && expected_param.type_ == actual_param.type_
                && expected_param.by_ref == actual_param.by_ref
                && expected_param.variadic == actual_param.variadic
        })
}

fn validate_object_mvp(class: &RuntimeClassEntry) -> Result<(), String> {
    if class.flags.is_enum {
        return Err(format!(
            "E_PHP_VM_ENUM_INSTANTIATION: enum {} cannot be instantiated",
            class.name
        ));
    }
    if class.flags.is_interface {
        return Err(format!(
            "E_PHP_VM_INTERFACE_INSTANTIATION: interface {} cannot be instantiated",
            class.name
        ));
    }
    if class.flags.is_abstract {
        return Err(format!(
            "E_PHP_VM_ABSTRACT_CLASS_INSTANTIATION: class {} is abstract",
            class.name
        ));
    }
    for method in &class.methods {
        if method.flags.is_abstract {
            return Err(format!(
                "E_PHP_VM_UNSUPPORTED_METHOD_MODIFIER: method {}::{} is abstract outside the Prompt 16 concrete method MVP",
                class.name, method.name
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct ResolvedMethod<'a> {
    class: &'a php_ir::module::ClassEntry,
    method: &'a php_ir::module::ClassMethodEntry,
}

#[derive(Clone, Copy)]
struct ResolvedProperty<'a> {
    class: &'a php_ir::module::ClassEntry,
    property: &'a php_ir::module::ClassPropertyEntry,
}

#[derive(Clone, Copy)]
struct ResolvedConstant<'a> {
    class: &'a php_ir::module::ClassEntry,
    constant: &'a php_ir::module::ClassConstantEntry,
}

fn validate_method_callable(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class: &php_ir::module::ClassEntry,
    method: &php_ir::module::ClassMethodEntry,
) -> Result<(), String> {
    if method.flags.is_abstract {
        return Err(format!(
            "E_PHP_VM_ABSTRACT_METHOD_CALL: method {}::{} is abstract",
            class.name, method.name
        ));
    }
    if method.flags.is_private {
        let scope = current_scope_class(compiled, stack);
        if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
            return Err(format!(
                "E_PHP_VM_PRIVATE_METHOD_ACCESS: method {}::{} is private",
                class.name, method.name
            ));
        }
    }
    if method.flags.is_protected {
        let Some(scope) = current_scope_class(compiled, stack) else {
            return Err(format!(
                "E_PHP_VM_PROTECTED_METHOD_ACCESS: method {}::{} is protected",
                class.name, method.name
            ));
        };
        if !class_is_or_extends(compiled, &scope, &class.name)? {
            return Err(format!(
                "E_PHP_VM_PROTECTED_METHOD_ACCESS: method {}::{} is protected",
                class.name, method.name
            ));
        }
    }
    Ok(())
}

fn lookup_method_in_hierarchy<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    method: &str,
    caller_scope: Option<&str>,
) -> Result<Option<ResolvedMethod<'a>>, String> {
    lookup_method_in_hierarchy_inner(compiled, class, method, caller_scope, &mut Vec::new())
}

fn lookup_method_in_hierarchy_inner<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    method: &str,
    caller_scope: Option<&str>,
    seen: &mut Vec<String>,
) -> Result<Option<ResolvedMethod<'a>>, String> {
    let class_name = normalize_class_name(&class.name);
    if seen.iter().any(|name| name == &class_name) {
        return Err(format!(
            "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
            class.name
        ));
    }
    seen.push(class_name.clone());
    let normalized = normalize_method_name(method);
    if let Some(method) = class
        .methods
        .iter()
        .find(|entry| normalize_method_name(&entry.name) == normalized)
    {
        if method.flags.is_private
            && caller_scope.is_some_and(|scope| normalize_class_name(scope) != class_name)
            && class.parent.is_some()
            && let Some(parent) = parent_class(compiled, class)?
            && let Some(parent_method) = lookup_method_in_hierarchy_inner(
                compiled,
                parent,
                method.name.as_str(),
                caller_scope,
                seen,
            )?
        {
            seen.pop();
            return Ok(Some(parent_method));
        }
        seen.pop();
        return Ok(Some(ResolvedMethod { class, method }));
    }
    if let Some(parent) = parent_class(compiled, class)? {
        let resolved =
            lookup_method_in_hierarchy_inner(compiled, parent, method, caller_scope, seen)?;
        seen.pop();
        return Ok(resolved);
    }
    seen.pop();
    Ok(None)
}

fn parent_class<'a>(
    compiled: &'a CompiledUnit,
    class: &php_ir::module::ClassEntry,
) -> Result<Option<&'a php_ir::module::ClassEntry>, String> {
    let Some(parent) = class.parent.as_deref() else {
        return Ok(None);
    };
    compiled.lookup_class(parent).map(Some).ok_or_else(|| {
        format!(
            "E_PHP_VM_UNKNOWN_PARENT_CLASS: class {} extends missing class {}",
            class.name, parent
        )
    })
}

fn lookup_property_in_hierarchy<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    property: &str,
    caller_scope: Option<&str>,
) -> Result<Option<ResolvedProperty<'a>>, String> {
    lookup_property_in_hierarchy_inner(compiled, class, property, caller_scope, &mut Vec::new())
}

fn lookup_property_in_hierarchy_inner<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    property: &str,
    caller_scope: Option<&str>,
    seen: &mut Vec<String>,
) -> Result<Option<ResolvedProperty<'a>>, String> {
    let class_name = normalize_class_name(&class.name);
    if seen.iter().any(|name| name == &class_name) {
        return Err(format!(
            "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
            class.name
        ));
    }
    seen.push(class_name.clone());
    if let Some(entry) = class.properties.iter().find(|entry| entry.name == property) {
        if entry.flags.is_private
            && caller_scope.is_some_and(|scope| normalize_class_name(scope) != class_name)
            && class.parent.is_some()
            && let Some(parent) = parent_class(compiled, class)?
            && let Some(parent_property) = lookup_property_in_hierarchy_inner(
                compiled,
                parent,
                entry.name.as_str(),
                caller_scope,
                seen,
            )?
        {
            seen.pop();
            return Ok(Some(parent_property));
        }
        seen.pop();
        return Ok(Some(ResolvedProperty {
            class,
            property: entry,
        }));
    }
    if let Some(parent) = parent_class(compiled, class)? {
        let resolved =
            lookup_property_in_hierarchy_inner(compiled, parent, property, caller_scope, seen)?;
        seen.pop();
        return Ok(resolved);
    }
    seen.pop();
    Ok(None)
}

fn lookup_constant_in_hierarchy<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    constant: &str,
    caller_scope: Option<&str>,
) -> Result<Option<ResolvedConstant<'a>>, String> {
    lookup_constant_in_hierarchy_inner(compiled, class, constant, caller_scope, &mut Vec::new())
}

fn lookup_constant_in_hierarchy_inner<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    constant: &str,
    caller_scope: Option<&str>,
    seen: &mut Vec<String>,
) -> Result<Option<ResolvedConstant<'a>>, String> {
    let class_name = normalize_class_name(&class.name);
    if seen.iter().any(|name| name == &class_name) {
        return Err(format!(
            "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
            class.name
        ));
    }
    seen.push(class_name.clone());
    if let Some(entry) = class.constants.iter().find(|entry| entry.name == constant) {
        if entry.flags.is_private
            && caller_scope.is_some_and(|scope| normalize_class_name(scope) != class_name)
            && class.parent.is_some()
            && let Some(parent) = parent_class(compiled, class)?
            && let Some(parent_constant) = lookup_constant_in_hierarchy_inner(
                compiled,
                parent,
                entry.name.as_str(),
                caller_scope,
                seen,
            )?
        {
            seen.pop();
            return Ok(Some(parent_constant));
        }
        seen.pop();
        return Ok(Some(ResolvedConstant {
            class,
            constant: entry,
        }));
    }
    if let Some(parent) = parent_class(compiled, class)? {
        let resolved =
            lookup_constant_in_hierarchy_inner(compiled, parent, constant, caller_scope, seen)?;
        seen.pop();
        return Ok(resolved);
    }
    seen.pop();
    Ok(None)
}

fn validate_property_access(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> Result<(), String> {
    if property.flags.is_private {
        let scope = current_scope_class(compiled, stack);
        if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
            return Err(format!(
                "E_PHP_VM_PRIVATE_PROPERTY_ACCESS: property {}::${} is private",
                class.name, property.name
            ));
        }
    }
    if property.flags.is_protected {
        let Some(scope) = current_scope_class(compiled, stack) else {
            return Err(format!(
                "E_PHP_VM_PROTECTED_PROPERTY_ACCESS: property {}::${} is protected",
                class.name, property.name
            ));
        };
        if !class_is_or_extends(compiled, &scope, &class.name)? {
            return Err(format!(
                "E_PHP_VM_PROTECTED_PROPERTY_ACCESS: property {}::${} is protected",
                class.name, property.name
            ));
        }
    }
    Ok(())
}

fn validate_property_set_access(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> Result<(), String> {
    if property.flags.set_is_private {
        let scope = current_scope_class(compiled, stack);
        if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
            return Err(format!(
                "E_PHP_VM_PRIVATE_PROPERTY_SET_ACCESS: property {}::${} setter is private",
                class.name, property.name
            ));
        }
    }
    if property.flags.set_is_protected {
        let Some(scope) = current_scope_class(compiled, stack) else {
            return Err(format!(
                "E_PHP_VM_PROTECTED_PROPERTY_SET_ACCESS: property {}::${} setter is protected",
                class.name, property.name
            ));
        };
        if !class_is_or_extends(compiled, &scope, &class.name)? {
            return Err(format!(
                "E_PHP_VM_PROTECTED_PROPERTY_SET_ACCESS: property {}::${} setter is protected",
                class.name, property.name
            ));
        }
    }
    Ok(())
}

fn validate_constant_access(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class: &php_ir::module::ClassEntry,
    constant: &php_ir::module::ClassConstantEntry,
) -> Result<(), String> {
    if constant.flags.is_private {
        let scope = current_scope_class(compiled, stack);
        if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
            return Err(format!(
                "E_PHP_VM_PRIVATE_CLASS_CONSTANT_ACCESS: constant {}::{} is private",
                class.name, constant.name
            ));
        }
    }
    if constant.flags.is_protected {
        let Some(scope) = current_scope_class(compiled, stack) else {
            return Err(format!(
                "E_PHP_VM_PROTECTED_CLASS_CONSTANT_ACCESS: constant {}::{} is protected",
                class.name, constant.name
            ));
        };
        if !class_is_or_extends(compiled, &scope, &class.name)? {
            return Err(format!(
                "E_PHP_VM_PROTECTED_CLASS_CONSTANT_ACCESS: constant {}::{} is protected",
                class.name, constant.name
            ));
        }
    }
    Ok(())
}

fn validate_property_write(
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
    object: &ObjectRef,
    stack: &CallStack,
    compiled: &CompiledUnit,
) -> Result<(), String> {
    if !(property.flags.is_readonly || class.flags.is_readonly) {
        return Ok(());
    }
    let scope = current_scope_class(compiled, stack);
    if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
        return Err(format!(
            "E_PHP_VM_READONLY_PROPERTY_WRITE: property {}::${} is readonly",
            class.name, property.name
        ));
    }
    let storage_name = property_storage_name(class, property);
    if !matches!(
        object.get_property(&storage_name),
        None | Some(Value::Uninitialized)
    ) {
        return Err(format!(
            "E_PHP_VM_READONLY_PROPERTY_WRITE: property {}::${} is already initialized",
            class.name, property.name
        ));
    }
    Ok(())
}

fn property_state_value(
    compiled: &CompiledUnit,
    stack: &CallStack,
    object: &ObjectRef,
    property: &str,
) -> Option<Value> {
    let Some(class) = compiled.lookup_class(&object.class_name()) else {
        return object.get_property(property);
    };
    let scope = current_scope_class(compiled, stack);
    let Some(resolved) =
        lookup_property_in_hierarchy(compiled, class, property, scope.as_deref()).ok()?
    else {
        return object.get_property(property);
    };
    if validate_property_access(compiled, stack, resolved.class, resolved.property).is_err() {
        return None;
    }
    let storage_name = property_storage_name(resolved.class, resolved.property);
    object
        .get_property(&storage_name)
        .or_else(|| object.get_property(property))
}

fn validate_static_property_write(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
    current: &Value,
) -> Result<(), String> {
    if !(property.flags.is_readonly || class.flags.is_readonly) {
        return Ok(());
    }
    let scope = current_scope_class(compiled, stack);
    if scope.as_deref() != Some(normalize_class_name(&class.name).as_str()) {
        return Err(format!(
            "E_PHP_VM_READONLY_STATIC_PROPERTY_WRITE: property {}::${} is readonly",
            class.name, property.name
        ));
    }
    if !matches!(current, Value::Uninitialized) {
        return Err(format!(
            "E_PHP_VM_READONLY_STATIC_PROPERTY_WRITE: property {}::${} is already initialized",
            class.name, property.name
        ));
    }
    Ok(())
}

fn static_property_key(
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> (String, String) {
    (normalize_class_name(&class.name), property.name.clone())
}

fn static_property_default(
    unit: &IrUnit,
    _class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> Result<Value, String> {
    if let Some(default) = property.default {
        constant_value(unit, default)
    } else if property.flags.is_typed {
        Ok(Value::Uninitialized)
    } else {
        Ok(Value::Null)
    }
}

fn resolve_static_class_name<'a>(
    compiled: &'a CompiledUnit,
    stack: &CallStack,
    class_name: &str,
) -> Result<&'a php_ir::module::ClassEntry, String> {
    match normalize_class_name(class_name).as_str() {
        "self" => {
            let Some(scope) = current_scope_class(compiled, stack) else {
                return Err(format!(
                    "E_PHP_VM_INVALID_STATIC_SCOPE: {class_name}:: is not available outside class scope"
                ));
            };
            compiled
                .lookup_class(&scope)
                .ok_or_else(|| format!("E_PHP_VM_UNKNOWN_CLASS: class {scope} is not defined"))
        }
        "static" => {
            let Some(called) = current_called_class(compiled, stack)
                .or_else(|| current_scope_class(compiled, stack))
            else {
                return Err(
                    "E_PHP_VM_INVALID_STATIC_SCOPE: static:: is not available outside class scope"
                        .to_owned(),
                );
            };
            compiled
                .lookup_class(&called)
                .ok_or_else(|| format!("E_PHP_VM_UNKNOWN_CLASS: class {called} is not defined"))
        }
        "parent" => {
            let Some(scope) = current_scope_class(compiled, stack) else {
                return Err(
                    "E_PHP_VM_INVALID_STATIC_SCOPE: parent:: is not available outside class scope"
                        .to_owned(),
                );
            };
            let Some(class) = compiled.lookup_class(&scope) else {
                return Err(format!(
                    "E_PHP_VM_UNKNOWN_CLASS: class {scope} is not defined"
                ));
            };
            let Some(parent) = parent_class(compiled, class)? else {
                return Err(format!(
                    "E_PHP_VM_NO_PARENT_CLASS: class {} has no parent",
                    class.name
                ));
            };
            Ok(parent)
        }
        _ => compiled
            .lookup_class(class_name)
            .ok_or_else(|| format!("E_PHP_VM_UNKNOWN_CLASS: class {class_name} is not defined")),
    }
}

fn called_class_for_static_call(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class_name: &str,
    resolved_class: &php_ir::module::ClassEntry,
) -> String {
    match normalize_class_name(class_name).as_str() {
        "self" | "static" | "parent" => current_called_class(compiled, stack)
            .or_else(|| current_scope_class(compiled, stack))
            .unwrap_or_else(|| normalize_class_name(&resolved_class.name)),
        _ => normalize_class_name(&resolved_class.name),
    }
}

fn method_lookup_scope_for_static_call(
    compiled: &CompiledUnit,
    stack: &CallStack,
    class_name: &str,
) -> Option<String> {
    if normalize_class_name(class_name) == "static" {
        None
    } else {
        current_scope_class(compiled, stack)
    }
}

fn current_scope_class(compiled: &CompiledUnit, stack: &CallStack) -> Option<String> {
    let frame = stack.current()?;
    if let Some(scope) = frame.scope_class.as_deref() {
        return Some(normalize_class_name(scope));
    }
    let function = compiled.unit().functions.get(frame.function.index())?;
    function
        .flags
        .is_method
        .then(|| {
            function
                .name
                .split_once("::")
                .map(|(class, _)| normalize_class_name(class))
        })
        .flatten()
}

fn current_called_class(compiled: &CompiledUnit, stack: &CallStack) -> Option<String> {
    let frame = stack.current()?;
    frame
        .called_class
        .as_deref()
        .map(normalize_class_name)
        .or_else(|| current_scope_class(compiled, stack))
}

fn class_is_or_extends(
    compiled: &CompiledUnit,
    class_name: &str,
    ancestor_name: &str,
) -> Result<bool, String> {
    let ancestor_name = normalize_class_name(ancestor_name);
    let Some(mut class) = compiled.lookup_class(class_name) else {
        return Ok(false);
    };
    let mut seen = Vec::new();
    loop {
        let current = normalize_class_name(&class.name);
        if current == ancestor_name {
            return Ok(true);
        }
        if seen.iter().any(|name| name == &current) {
            return Err(format!(
                "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
                class.name
            ));
        }
        seen.push(current);
        let Some(parent) = parent_class(compiled, class)? else {
            return Ok(false);
        };
        class = parent;
    }
}

fn class_is_or_implements(
    compiled: &CompiledUnit,
    class_name: &str,
    target_name: &str,
) -> Result<bool, String> {
    if class_is_or_extends(compiled, class_name, target_name)? {
        return Ok(true);
    }
    class_implements_interface(compiled, class_name, target_name, &mut Vec::new())
}

fn class_implements_interface(
    compiled: &CompiledUnit,
    class_name: &str,
    interface_name: &str,
    seen: &mut Vec<String>,
) -> Result<bool, String> {
    let interface_name = normalize_class_name(interface_name);
    let Some(class) = compiled.lookup_class(class_name) else {
        return Ok(false);
    };
    let current = normalize_class_name(&class.name);
    if seen.iter().any(|name| name == &current) {
        return Err(format!(
            "E_PHP_VM_CLASS_INHERITANCE_CYCLE: class {} participates in an inheritance cycle",
            class.name
        ));
    }
    seen.push(current);
    for interface in &class.interfaces {
        if interface_or_extends(compiled, interface, &interface_name, &mut Vec::new())? {
            seen.pop();
            return Ok(true);
        }
    }
    if let Some(parent) = parent_class(compiled, class)?
        && class_implements_interface(compiled, &parent.name, &interface_name, seen)?
    {
        seen.pop();
        return Ok(true);
    }
    seen.pop();
    Ok(false)
}

fn interface_or_extends(
    compiled: &CompiledUnit,
    interface_name: &str,
    target_name: &str,
    seen: &mut Vec<String>,
) -> Result<bool, String> {
    let interface_name = normalize_class_name(interface_name);
    let target_name = normalize_class_name(target_name);
    if interface_name == target_name {
        return Ok(true);
    }
    let Some(interface) = compiled.lookup_class(&interface_name) else {
        return Ok(false);
    };
    if seen.iter().any(|name| name == &interface_name) {
        return Err(format!(
            "E_PHP_VM_INTERFACE_INHERITANCE_CYCLE: interface {} participates in an inheritance cycle",
            interface.name
        ));
    }
    seen.push(interface_name);
    for parent in &interface.interfaces {
        if interface_or_extends(compiled, parent, &target_name, seen)? {
            seen.pop();
            return Ok(true);
        }
    }
    seen.pop();
    Ok(false)
}

fn object_instanceof(
    compiled: &CompiledUnit,
    value: &Value,
    class_name: &str,
) -> Result<bool, String> {
    match value {
        Value::Reference(cell) => object_instanceof(compiled, &cell.get(), class_name),
        Value::Fiber(_) => Ok(normalize_class_name(class_name) == "fiber"),
        Value::Object(object) => {
            if let Some(result) = internal_throwable_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            class_is_or_implements(compiled, &object.class_name(), class_name)
        }
        _ => Ok(false),
    }
}

fn normalize_class_name(name: &str) -> String {
    name.trim_start_matches('\\').to_ascii_lowercase()
}

fn property_storage_name(
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> String {
    if property.flags.is_private {
        format!(
            "private:{}:{}",
            normalize_class_name(&class.name),
            property.name
        )
    } else {
        property.name.clone()
    }
}

fn property_hook_is_active(
    state: &ExecutionState,
    object: &ObjectRef,
    class: &php_ir::module::ClassEntry,
    property: &php_ir::module::ClassPropertyEntry,
) -> bool {
    let class_name = normalize_class_name(&class.name);
    state.property_hook_stack.iter().any(|active| {
        active.object_id == object.id()
            && active.class_name == class_name
            && active.property == property.name
    })
}

fn normalize_method_name(method: &str) -> String {
    method.to_ascii_lowercase()
}

fn is_fiber_runtime_class(class_name: &str) -> bool {
    normalize_class_name(class_name) == "fiber"
}

fn new_fiber_object(args: Vec<CallArgument>) -> Result<FiberRef, String> {
    if let Some(name) = args.iter().find_map(|arg| arg.name.as_deref()) {
        return Err(format!(
            "E_PHP_VM_UNKNOWN_NAMED_ARG: Fiber::__construct has no builtin parameter ${name}"
        ));
    }
    if args.len() != 1 {
        let id = if args.is_empty() {
            "E_PHP_VM_TOO_FEW_ARGS"
        } else {
            "E_PHP_VM_TOO_MANY_ARGS"
        };
        return Err(format!(
            "{id}: Fiber::__construct expects exactly 1 argument(s), {} given",
            args.len()
        ));
    }
    let callable = args
        .into_iter()
        .next()
        .expect("checked exactly one argument")
        .value;
    if !fiber_constructor_accepts_callable(&callable) {
        return Err(format!(
            "E_PHP_VM_FIBER_CONSTRUCTOR_NOT_CALLABLE: Fiber::__construct expects callable, {} given",
            value_type_name(&callable)
        ));
    }
    Ok(FiberRef::new(callable))
}

fn fiber_constructor_accepts_callable(value: &Value) -> bool {
    matches!(
        value,
        Value::Callable(_) | Value::String(_) | Value::Array(_) | Value::Object(_)
    )
}

fn validate_fiber_arg_count(
    method: &str,
    args: &[CallArgument],
    expected: usize,
) -> Result<(), String> {
    if let Some(name) = args.iter().find_map(|arg| arg.name.as_deref()) {
        return Err(format!(
            "E_PHP_VM_UNKNOWN_NAMED_ARG: Fiber::{method} has no builtin parameter ${name}"
        ));
    }
    if args.len() != expected {
        let id = if args.len() < expected {
            "E_PHP_VM_TOO_FEW_ARGS"
        } else {
            "E_PHP_VM_TOO_MANY_ARGS"
        };
        return Err(format!(
            "{id}: Fiber::{method} expects exactly {expected} argument(s), {} given",
            args.len()
        ));
    }
    Ok(())
}

fn validate_generator_arg_count(
    method: &str,
    args: &[CallArgument],
    expected: usize,
) -> Result<(), String> {
    if let Some(name) = args.iter().find_map(|arg| arg.name.as_deref()) {
        return Err(format!(
            "E_PHP_VM_UNKNOWN_NAMED_ARG: Generator::{method} has no builtin parameter ${name}"
        ));
    }
    if args.len() != expected {
        let id = if args.len() < expected {
            "E_PHP_VM_TOO_FEW_ARGS"
        } else {
            "E_PHP_VM_TOO_MANY_ARGS"
        };
        return Err(format!(
            "{id}: Generator::{method} expects exactly {expected} argument(s), {} given",
            args.len()
        ));
    }
    Ok(())
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

fn is_autoload_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "spl_autoload_register"
            | "spl_autoload_unregister"
            | "spl_autoload_functions"
            | "spl_autoload_call"
    )
}

fn is_class_probe_builtin_name(name: &str) -> bool {
    matches!(name, "class_exists" | "interface_exists")
}

fn autoload_callback_from_value(
    compiled: &CompiledUnit,
    value: Value,
) -> Result<CallableValue, String> {
    match value {
        Value::Callable(CallableValue::UserFunction { name }) => {
            let normalized = normalize_function_name(&name);
            if compiled.lookup_function(&normalized).is_some()
                || BuiltinRegistry::new().contains(&normalized)
            {
                Ok(CallableValue::UserFunction { name: normalized })
            } else {
                Err(format!(
                    "E_PHP_VM_AUTOLOAD_INVALID_CALLBACK: function {name} is not callable"
                ))
            }
        }
        Value::Callable(CallableValue::Closure { function, captures }) => {
            Ok(CallableValue::Closure { function, captures })
        }
        Value::Callable(CallableValue::InternalBuiltin { name }) => {
            if BuiltinRegistry::new().contains(&name) {
                Ok(CallableValue::InternalBuiltin { name })
            } else {
                Err(format!(
                    "E_PHP_VM_AUTOLOAD_INVALID_CALLBACK: builtin {name} is not callable"
                ))
            }
        }
        Value::String(name) => {
            let name = normalize_function_name(&name.to_string_lossy());
            if compiled.lookup_function(&name).is_some() {
                Ok(CallableValue::UserFunction { name })
            } else if BuiltinRegistry::new().contains(&name) {
                Ok(CallableValue::InternalBuiltin { name })
            } else {
                Err(format!(
                    "E_PHP_VM_AUTOLOAD_INVALID_CALLBACK: function {name} is not callable"
                ))
            }
        }
        other => Err(format!(
            "E_PHP_VM_AUTOLOAD_INVALID_CALLBACK: value of type {} is not callable",
            value_type_name(&other)
        )),
    }
}

fn register_dynamic_classes(state: &mut ExecutionState, unit: &IrUnit) {
    for class in unit
        .classes
        .iter()
        .filter(|class| class.span != php_ir::source_map::IrSpan::default())
    {
        let normalized = normalize_class_name(&class.name);
        if state
            .dynamic_classes
            .iter()
            .any(|existing| normalize_class_name(&existing.name) == normalized)
        {
            continue;
        }
        state.dynamic_classes.push(class.clone());
    }
}

fn lookup_class_in_state(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class_name: &str,
) -> Option<php_ir::module::ClassEntry> {
    compiled.lookup_class(class_name).cloned().or_else(|| {
        let normalized = normalize_class_name(class_name);
        state
            .dynamic_classes
            .iter()
            .find(|class| normalize_class_name(&class.name) == normalized)
            .cloned()
    })
}

fn eval_failure(
    output: &OutputBuffer,
    message: impl Into<String>,
    stack_trace: Vec<RuntimeStackFrame>,
) -> VmResult {
    let message = message.into();
    VmResult::runtime_error_with_diagnostic(
        output.clone(),
        message.clone(),
        RuntimeDiagnostic::new(
            eval_failure_id(&message).to_owned(),
            RuntimeSeverity::FatalError,
            message,
            RuntimeSourceSpan::default(),
            stack_trace,
            None,
        ),
    )
}

fn eval_failure_id(message: &str) -> &str {
    message
        .split_once(':')
        .and_then(|(id, _)| id.starts_with("E_").then_some(id))
        .unwrap_or("E_PHP_VM_EVAL_ERROR")
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

fn seed_runtime_globals(globals: &mut GlobalSymbolTable, context: &RuntimeContext) {
    for name in [
        "argc", "argv", "_SERVER", "_ENV", "_GET", "_POST", "_COOKIE", "_FILES", "_REQUEST",
    ] {
        if let Some(value) = context.global_value(name) {
            globals.set(name, value);
        }
    }
}

fn bind_top_level_global_locals(
    function: &IrFunction,
    stack: &mut CallStack,
    state: &mut ExecutionState,
) {
    let Some(frame) = stack.current_mut() else {
        return;
    };
    for (index, name) in function.locals.iter().enumerate() {
        if name == "GLOBALS" {
            continue;
        }
        let cell = state
            .globals
            .ensure_slot(name.clone(), Value::Uninitialized);
        let _ = frame
            .locals
            .bind_reference_cell(LocalId::new(index as u32), cell);
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

fn prepare_arguments(
    function: &IrFunction,
    args: Vec<CallArgument>,
    stack: &mut CallStack,
) -> Result<Vec<PreparedArg>, String> {
    let min = function
        .params
        .iter()
        .filter(|param| param.required)
        .count();
    let variadic_index = function.params.iter().position(|param| param.variadic);
    let max = variadic_index.unwrap_or(function.params.len());
    let mut bound: Vec<Option<CallArgument>> = (0..function.params.len()).map(|_| None).collect();
    let mut variadic_tail = Vec::new();
    let mut positional_index = 0usize;
    let mut saw_named = false;
    let mut supplied_count = 0usize;

    for arg in args {
        if let Some(name) = arg.name.clone() {
            saw_named = true;
            let Some(index) = function.params.iter().position(|param| param.name == name) else {
                if variadic_index.is_some() {
                    variadic_tail.push(VariadicTailArg {
                        key: Some(name),
                        value: arg.value,
                    });
                    supplied_count += 1;
                    continue;
                }
                return Err(format!(
                    "E_PHP_VM_UNKNOWN_NAMED_ARG: function {} has no parameter ${name}",
                    function.name
                ));
            };
            if function.params[index].variadic {
                variadic_tail.push(VariadicTailArg {
                    key: Some(name),
                    value: arg.value,
                });
                supplied_count += 1;
                continue;
            }
            if bound[index].is_some() {
                return Err(format!(
                    "E_PHP_VM_DUPLICATE_NAMED_ARG: function {} argument ${name} was already provided",
                    function.name
                ));
            }
            bound[index] = Some(CallArgument {
                name: None,
                value: arg.value,
                by_ref_local: arg.by_ref_local,
            });
            supplied_count += 1;
            continue;
        }

        if saw_named {
            return Err(format!(
                "E_PHP_VM_POSITIONAL_AFTER_NAMED_ARG: function {} cannot use positional argument after named argument",
                function.name
            ));
        }
        if variadic_index.is_some_and(|index| positional_index >= index) {
            variadic_tail.push(VariadicTailArg {
                key: None,
                value: arg.value,
            });
            positional_index += 1;
            supplied_count += 1;
            continue;
        }
        if positional_index >= max {
            return Err(format!(
                "E_PHP_VM_TOO_MANY_ARGS: function {} expects at most {} argument(s), got {}",
                function.name,
                max,
                supplied_count + 1
            ));
        }
        if bound[positional_index].is_some() {
            return Err(format!(
                "E_PHP_VM_DUPLICATE_NAMED_ARG: function {} argument ${} was already provided",
                function.name, function.params[positional_index].name
            ));
        }
        bound[positional_index] = Some(CallArgument {
            name: None,
            value: arg.value,
            by_ref_local: arg.by_ref_local,
        });
        positional_index += 1;
        supplied_count += 1;
    }

    if supplied_count < min {
        return Err(format!(
            "E_PHP_VM_TOO_FEW_ARGS: function {} expects at least {} argument(s), got {}",
            function.name, min, supplied_count
        ));
    }

    let mut prepared = Vec::with_capacity(function.params.len());
    for (index, param) in function.params.iter().enumerate() {
        if param.variadic {
            prepared.push(PreparedArg {
                value: variadic_array(variadic_tail),
                reference: None,
            });
            break;
        }
        if let Some(arg) = bound[index].take() {
            let reference = if param.by_ref {
                let Some(local) = arg.by_ref_local else {
                    return Err(format!(
                        "E_PHP_VM_BY_REF_ARG_NOT_REFERENCEABLE: function {} argument ${} must be a variable",
                        function.name, param.name
                    ));
                };
                let frame = stack.current_mut().ok_or("no active frame")?;
                Some(frame.locals.ensure_reference_cell(local)?)
            } else {
                None
            };
            prepared.push(PreparedArg {
                value: arg.value,
                reference,
            });
        } else if let Some(default) = &param.default {
            if param.by_ref {
                return Err(format!(
                    "E_PHP_VM_BY_REF_ARG_NOT_REFERENCEABLE: function {} argument ${} must be a variable",
                    function.name, param.name
                ));
            }
            prepared.push(PreparedArg {
                value: inline_constant_value(default),
                reference: None,
            });
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

struct VariadicTailArg {
    key: Option<String>,
    value: Value,
}

fn variadic_array(args: Vec<VariadicTailArg>) -> Value {
    let mut array = PhpArray::new();
    for arg in args {
        if let Some(key) = arg.key {
            array.insert(ArrayKey::String(PhpString::from(key.as_str())), arg.value);
        } else {
            array.append(arg.value);
        }
    }
    Value::Array(array)
}

fn coerce_or_check_param_type(
    compiled: &CompiledUnit,
    unit: &IrUnit,
    function: &IrFunction,
    param: &IrParam,
    value: &mut Value,
    by_ref_arg: bool,
) -> Result<(), String> {
    if param.variadic {
        return Ok(());
    }
    let Some(runtime_type) = ir_runtime_type(param.type_.as_ref()) else {
        return Ok(());
    };
    if !unit.strict_types
        && !by_ref_arg
        && let Some(coerced) = coerce_value_to_runtime_type(value, &runtime_type)
    {
        *value = coerced;
        return Ok(());
    }
    if vm_value_matches_runtime_type(compiled, value, &runtime_type)? {
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
    stack: &mut CallStack,
    captures: &[ClosureCaptureArg],
) -> Result<Vec<ClosureCaptureValue>, String> {
    let mut values = Vec::with_capacity(captures.len());
    for capture in captures {
        if capture.by_ref {
            let Operand::Local(local) = capture.src else {
                return Err(format!(
                    "E_PHP_VM_BY_REF_CAPTURE_NOT_REFERENCEABLE: closure capture ${} is not a local variable",
                    capture.name
                ));
            };
            let cell = stack
                .current_mut()
                .ok_or("no active frame")?
                .locals
                .ensure_reference_cell(local)?;
            values.push(ClosureCaptureValue::by_reference(
                capture.name.clone(),
                cell,
            ));
            continue;
        }
        values.push(ClosureCaptureValue::by_value(
            capture.name.clone(),
            read_operand(unit, stack, capture.src)?,
        ));
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
        let capture = captures
            .iter()
            .find(|capture| capture.name == metadata.name)
            .cloned();
        let locals = &mut stack.current_mut().expect("frame was pushed").locals;
        if metadata.by_ref {
            let Some(cell) = capture.and_then(|capture| capture.reference()) else {
                return Err(format!(
                    "E_PHP_VM_BY_REF_CAPTURE_MISSING_CELL: closure capture ${} has no reference cell",
                    metadata.name
                ));
            };
            locals.bind_reference_cell(metadata.local, cell)?;
        } else {
            let value = capture
                .and_then(|capture| capture.value().cloned())
                .unwrap_or(Value::Null);
            locals.set(metadata.local, value)?;
        }
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

fn check_return_type(
    compiled: &CompiledUnit,
    function: &IrFunction,
    value: Option<&Value>,
) -> Result<(), String> {
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
    if vm_value_matches_runtime_type(compiled, value, &return_type)? {
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
    compiled: &CompiledUnit,
    class_name: &str,
    property: &str,
    runtime_type: &Option<RuntimeType>,
    value: &Value,
) -> Result<(), String> {
    let Some(runtime_type) = runtime_type else {
        return Ok(());
    };
    if vm_value_matches_runtime_type(compiled, value, runtime_type)? {
        Ok(())
    } else {
        Err(format!(
            "E_PHP_VM_PROPERTY_TYPE_MISMATCH: property {class_name}::${property} got {}, expected {}",
            value_type_name(value),
            runtime_type_name(runtime_type)
        ))
    }
}

fn vm_value_matches_runtime_type(
    compiled: &CompiledUnit,
    value: &Value,
    runtime_type: &RuntimeType,
) -> Result<bool, String> {
    if let Value::Reference(cell) = value {
        return vm_value_matches_runtime_type(compiled, &cell.get(), runtime_type);
    }
    Ok(match runtime_type {
        RuntimeType::Class { name } => object_instanceof(compiled, value, name)?,
        RuntimeType::Nullable { inner } => {
            matches!(value, Value::Null) || vm_value_matches_runtime_type(compiled, value, inner)?
        }
        RuntimeType::Union { members } => {
            for member in members {
                if vm_value_matches_runtime_type(compiled, value, member)? {
                    return Ok(true);
                }
            }
            false
        }
        RuntimeType::Intersection { members } => {
            for member in members {
                if !vm_value_matches_runtime_type(compiled, value, member)? {
                    return Ok(false);
                }
            }
            true
        }
        RuntimeType::Dnf { clauses } => {
            for clause in clauses {
                if vm_value_matches_runtime_type(compiled, value, clause)? {
                    return Ok(true);
                }
            }
            false
        }
        _ => value_matches_runtime_type(value, runtime_type),
    })
}

fn ir_runtime_type(return_type: Option<&IrReturnType>) -> Option<RuntimeType> {
    Some(match return_type? {
        IrReturnType::Int => RuntimeType::Int,
        IrReturnType::Float => RuntimeType::Float,
        IrReturnType::String => RuntimeType::String,
        IrReturnType::Array => RuntimeType::Array,
        IrReturnType::Callable => RuntimeType::Callable,
        IrReturnType::Iterable => RuntimeType::Iterable,
        IrReturnType::Object => RuntimeType::Object,
        IrReturnType::Bool => RuntimeType::Bool,
        IrReturnType::Null => RuntimeType::Null,
        IrReturnType::Void => RuntimeType::Void,
        IrReturnType::Mixed => RuntimeType::Mixed,
        IrReturnType::Never => RuntimeType::Never,
        IrReturnType::False => RuntimeType::False,
        IrReturnType::True => RuntimeType::True,
        IrReturnType::Class { name } => RuntimeType::Class { name: name.clone() },
        IrReturnType::Nullable { inner } => RuntimeType::Nullable {
            inner: Box::new(ir_runtime_type(Some(inner))?),
        },
        IrReturnType::Union { members } => RuntimeType::Union {
            members: members
                .iter()
                .map(|member| ir_runtime_type(Some(member)))
                .collect::<Option<Vec<_>>>()?,
        },
        IrReturnType::Intersection { members } => RuntimeType::Intersection {
            members: members
                .iter()
                .map(|member| ir_runtime_type(Some(member)))
                .collect::<Option<Vec<_>>>()?,
        },
        IrReturnType::Dnf { members } => RuntimeType::Dnf {
            clauses: members
                .iter()
                .map(|member| ir_runtime_type(Some(member)))
                .collect::<Option<Vec<_>>>()?,
        },
    })
}

fn coerce_value_to_runtime_type(value: &Value, runtime_type: &RuntimeType) -> Option<Value> {
    if value_matches_runtime_type(value, runtime_type) {
        return Some(value.clone());
    }
    if !matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_)
    ) {
        return None;
    }
    match runtime_type {
        RuntimeType::Nullable { inner } if matches!(value, Value::Null) => Some(Value::Null),
        RuntimeType::Nullable { inner } => coerce_value_to_runtime_type(value, inner),
        RuntimeType::Union { members } => members
            .iter()
            .find_map(|member| coerce_value_to_runtime_type(value, member)),
        RuntimeType::Int if matches!(value, Value::String(_)) => {
            to_number(value).ok().map(|number| match number {
                NumericValue::Int(value) => Value::Int(value),
                NumericValue::Float(value) => Value::Int(value as i64),
            })
        }
        RuntimeType::Int => to_int(value).ok().map(Value::Int),
        RuntimeType::Float if matches!(value, Value::String(_)) => to_number(value)
            .ok()
            .map(|number| Value::float(number.as_f64())),
        RuntimeType::Float => to_float(value).ok().map(Value::float),
        RuntimeType::String => to_string(value).ok().map(Value::String),
        RuntimeType::Bool => to_bool(value).ok().map(Value::Bool),
        _ => None,
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
    if let Value::Reference(cell) = array {
        return fetch_dim_value(&cell.get(), key);
    }
    let Value::Array(array) = array else {
        return Err("E_PHP_VM_ARRAY_FETCH_TYPE: value is not an array".to_owned());
    };
    Ok(array.get(key).map(effective_value))
}

fn effective_value(value: &Value) -> Value {
    match value {
        Value::Reference(cell) => cell.get(),
        value => value.clone(),
    }
}

fn fetch_dim_path_value(value: &Value, dims: &[ArrayKey]) -> Result<Option<Value>, String> {
    let mut current = effective_value(value);
    for key in dims {
        let Value::Array(array) = &current else {
            return Ok(None);
        };
        let Some(next) = array.get(key) else {
            return Ok(None);
        };
        current = effective_value(next);
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

fn foreach_array_keys_from_local(
    stack: &CallStack,
    local: LocalId,
) -> Result<Vec<ArrayKey>, String> {
    let value = read_local_value(stack, local).unwrap_or(Value::Null);
    let value = effective_value(&value);
    let Value::Array(array) = value else {
        return Err(format!(
            "E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE: foreach by reference over {} is not implemented; Phase 5 supports local arrays only",
            value_type_name(&value)
        ));
    };
    Ok(array.iter().map(|(key, _)| key.clone()).collect())
}

fn object_property_iteration_keys(
    compiled: &CompiledUnit,
    object: &ObjectRef,
) -> Result<Vec<String>, String> {
    let class_name = object.class_name();
    let Some(class) = compiled.lookup_class(&class_name) else {
        return Err(format!(
            "E_PHP_VM_UNKNOWN_CLASS: class {} is not defined",
            class_name
        ));
    };
    let mut keys = Vec::new();
    for property in &class.properties {
        if property.flags.is_static
            || property.flags.is_private
            || property.flags.is_protected
            || object.get_property(&property.name).is_none()
        {
            continue;
        }
        if !keys.iter().any(|name| name == &property.name) {
            keys.push(property.name.clone());
        }
    }
    for (name, _) in object.properties_snapshot() {
        if name.contains(':') {
            continue;
        }
        if !keys.iter().any(|existing| existing == &name) {
            keys.push(name);
        }
    }
    Ok(keys)
}

fn is_this_local(function: &IrFunction, local: LocalId) -> bool {
    function
        .locals
        .get(local.index())
        .is_some_and(|name| name == "this")
}

fn is_globals_local(function: &IrFunction, local: LocalId) -> bool {
    function
        .locals
        .get(local.index())
        .is_some_and(|name| name == "GLOBALS")
}

fn read_call_args(
    unit: &IrUnit,
    stack: &CallStack,
    args: &[IrCallArg],
) -> Result<Vec<CallArgument>, String> {
    let mut out = Vec::new();
    for arg in args {
        let value = read_operand(unit, stack, arg.value)?;
        if arg.unpack {
            if arg.name.is_some() {
                return Err(
                    "E_PHP_VM_NAMED_UNPACK_ARG: unpacked arguments cannot have an explicit name"
                        .to_owned(),
                );
            }
            let Value::Array(array) = value else {
                return Err(format!(
                    "E_PHP_VM_UNPACK_NON_ARRAY: cannot unpack {} as call arguments",
                    value_type_name(&value)
                ));
            };
            for (key, value) in array.iter() {
                let name = match key {
                    ArrayKey::Int(_) => None,
                    ArrayKey::String(key) => Some(key.to_string()),
                };
                out.push(CallArgument {
                    name,
                    value: value.clone(),
                    by_ref_local: None,
                });
            }
            continue;
        }
        out.push(CallArgument {
            name: arg.name.clone(),
            value,
            by_ref_local: arg.by_ref_local,
        });
    }
    Ok(out)
}

fn call_args_to_positional(function: &str, args: Vec<CallArgument>) -> Result<Vec<Value>, String> {
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        if let Some(name) = arg.name {
            return Err(format!(
                "E_PHP_VM_UNKNOWN_NAMED_ARG: function {function} has no builtin parameter ${name}"
            ));
        }
        values.push(arg.value);
    }
    Ok(values)
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

fn assign_globals_dim(
    globals: &mut GlobalSymbolTable,
    dims: &[ArrayKey],
    value: Value,
    append: bool,
) -> Result<(), String> {
    if append {
        return Err(
            "E_PHP_VM_GLOBALS_APPEND_GAP: appending directly to $GLOBALS is not implemented"
                .to_owned(),
        );
    }
    let Some((first, rest)) = dims.split_first() else {
        return Err("E_PHP_VM_GLOBALS_ASSIGN_DIM: missing $GLOBALS key".to_owned());
    };
    let ArrayKey::String(name) = first else {
        return Err(
            "E_PHP_VM_GLOBALS_ASSIGN_KEY: $GLOBALS keys must be strings in Phase 5".to_owned(),
        );
    };
    let name = name.to_string();
    if rest.is_empty() {
        globals.set(name, value);
        return Ok(());
    }
    let cell = globals.ensure_slot(name, Value::Array(PhpArray::new()));
    let mut current = cell.get();
    if matches!(current, Value::Uninitialized | Value::Null) {
        current = Value::Array(PhpArray::new());
    }
    assign_dim_value(&mut current, rest, value, false)?;
    cell.set(current);
    Ok(())
}

fn assign_dim_value(
    container: &mut Value,
    dims: &[ArrayKey],
    value: Value,
    append: bool,
) -> Result<(), String> {
    if let Value::Reference(cell) = container {
        let mut current = cell.get();
        assign_dim_value(&mut current, dims, value, append)?;
        cell.set(current);
        return Ok(());
    }
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
        if let Some(existing) = array.get_mut(first) {
            write_lvalue(existing, value);
        } else {
            array.insert(first.clone(), value);
        }
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

fn bind_dim_local_to_reference_cell(
    stack: &mut CallStack,
    local: LocalId,
    dims: &[ArrayKey],
    append: bool,
    cell: ReferenceCell,
) -> Result<(), String> {
    let frame = stack.current_mut().ok_or("no active frame")?;
    let Some(slot) = frame.locals.get_slot_mut(local) else {
        return Err(format!("invalid local local:{}", local.raw()));
    };
    let mut current = slot.read();
    if matches!(current, Value::Uninitialized | Value::Null) {
        current = Value::Array(PhpArray::new());
    }
    bind_dim_value_to_reference_cell(&mut current, dims, append, cell)?;
    slot.write(current);
    Ok(())
}

fn bind_dim_value_to_reference_cell(
    container: &mut Value,
    dims: &[ArrayKey],
    append: bool,
    cell: ReferenceCell,
) -> Result<(), String> {
    if let Value::Reference(container_cell) = container {
        let mut current = container_cell.get();
        bind_dim_value_to_reference_cell(&mut current, dims, append, cell)?;
        container_cell.set(current);
        return Ok(());
    }
    let Value::Array(array) = container else {
        return Err(format!(
            "E_PHP_VM_ARRAY_BIND_DIM_TYPE: cannot bind dimension on {}",
            value_type_name(container)
        ));
    };
    let Some((first, rest)) = dims.split_first() else {
        if append {
            array.append(Value::Reference(cell));
            return Ok(());
        }
        return Err("E_PHP_VM_ARRAY_BIND_DIM: missing array dimension".to_owned());
    };
    if rest.is_empty() && !append {
        array.insert(first.clone(), Value::Reference(cell));
        return Ok(());
    }
    if array.get(first).is_none() {
        array.insert(first.clone(), Value::Array(PhpArray::new()));
    }
    let Some(child) = array.get_mut(first) else {
        return Err("E_PHP_VM_ARRAY_BIND_DIM: failed to create nested array".to_owned());
    };
    if matches!(child, Value::Uninitialized | Value::Null) {
        *child = Value::Array(PhpArray::new());
    }
    bind_dim_value_to_reference_cell(child, rest, append, cell)
}

fn ensure_dim_reference_cell(
    stack: &mut CallStack,
    local: LocalId,
    dims: &[ArrayKey],
) -> Result<ReferenceCell, String> {
    let frame = stack.current_mut().ok_or("no active frame")?;
    let Some(slot) = frame.locals.get_slot_mut(local) else {
        return Err(format!("invalid local local:{}", local.raw()));
    };
    let mut current = slot.read();
    if matches!(current, Value::Uninitialized | Value::Null) {
        current = Value::Array(PhpArray::new());
    }
    let cell = ensure_dim_reference_cell_value(&mut current, dims)?;
    slot.write(current);
    Ok(cell)
}

fn ensure_dim_reference_cell_value(
    container: &mut Value,
    dims: &[ArrayKey],
) -> Result<ReferenceCell, String> {
    if let Value::Reference(container_cell) = container {
        let mut current = container_cell.get();
        let cell = ensure_dim_reference_cell_value(&mut current, dims)?;
        container_cell.set(current);
        return Ok(cell);
    }
    let Value::Array(array) = container else {
        return Err(format!(
            "E_PHP_VM_ARRAY_REF_DIM_TYPE: cannot reference dimension on {}",
            value_type_name(container)
        ));
    };
    let Some((first, rest)) = dims.split_first() else {
        return Err("E_PHP_VM_ARRAY_REF_DIM: missing array dimension".to_owned());
    };
    if array.get(first).is_none() {
        array.insert(first.clone(), Value::Null);
    }
    let Some(child) = array.get_mut(first) else {
        return Err("E_PHP_VM_ARRAY_REF_DIM: failed to create array element".to_owned());
    };
    if rest.is_empty() {
        return Ok(ensure_value_reference_cell(child));
    }
    if matches!(child, Value::Uninitialized | Value::Null) {
        *child = Value::Array(PhpArray::new());
    }
    ensure_dim_reference_cell_value(child, rest)
}

fn ensure_value_reference_cell(value: &mut Value) -> ReferenceCell {
    match value {
        Value::Reference(cell) => cell.clone(),
        value => {
            let cell = ReferenceCell::new(value.clone());
            *value = Value::Reference(cell.clone());
            cell
        }
    }
}

fn write_lvalue(target: &mut Value, value: Value) {
    match target {
        Value::Reference(cell) => cell.set(value),
        target => *target = value,
    }
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
    if let Value::Reference(cell) = container {
        let mut current = cell.get();
        unset_dim_value(&mut current, dims);
        cell.set(current);
        return;
    }
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
        Value::Reference(cell) => php_empty(&cell.get()),
        Value::Uninitialized | Value::Null => Ok(true),
        Value::Bool(value) => Ok(!*value),
        Value::Int(value) => Ok(*value == 0),
        Value::Float(value) => {
            let value = value.to_f64();
            Ok(value == 0.0 || value.is_nan())
        }
        Value::String(value) => Ok(value.is_empty() || value.as_bytes() == b"0"),
        Value::Array(array) => Ok(array.is_empty()),
        Value::Object(_) | Value::Fiber(_) | Value::Generator(_) | Value::Callable(_) => Ok(false),
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
            _ => {
                let rhs = rhs.as_f64() as i64;
                if rhs == 0 {
                    return Err("modulo by zero".to_owned());
                }
                Ok(Value::Int((lhs.as_f64() as i64) % rhs))
            }
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

        let private = execute_source(
            "<?php class PrivateSlot { private $value; function set($value) { $this->value = $value; } function get() { return $this->value; } } $slot = new PrivateSlot(); $slot->set(4); echo $slot->get();",
        );
        assert!(private.status.is_success(), "{:?}", private.status);
        assert_eq!(private.output.as_bytes(), b"4");
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
    fn destructors_run_at_shutdown_in_reverse_registration_order() {
        let result = execute_source(
            "<?php class D { public $name; function __construct($name) { $this->name = $name; } function __destruct() { echo 'd:', $this->name, '|'; } } $a = new D('a'); $b = new D('b'); echo 'body|';",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"body|d:b|d:a|");
    }

    #[test]
    fn destructors_register_clones_and_reentrant_objects() {
        let result = execute_source(
            "<?php class D { public $name; function __construct($name) { $this->name = $name; } function __clone() { $this->name = 'clone'; } function __destruct() { echo 'd:', $this->name, '|'; if ($this->name === 'clone') { new D('late'); } } } $a = new D('a'); $b = clone $a; echo 'body|';",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"body|d:clone|d:a|d:late|");
    }

    #[test]
    fn destructor_throw_becomes_shutdown_runtime_error() {
        let result = execute_source(
            "<?php class D { function __destruct() { echo 'destruct|'; throw new Exception('boom'); } } new D(); echo 'body|';",
        );

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(result.output.as_bytes(), b"body|destruct|");
        assert_eq!(result.diagnostics[0].id(), "E_PHP_VM_UNCAUGHT_EXCEPTION");
    }

    #[test]
    fn gc_snapshot_tracks_vm_roots_and_cycle_candidates() {
        let class = RuntimeClassEntry {
            name: "GcBox".to_owned(),
            parent: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            constants: Vec::new(),
            enum_cases: Vec::new(),
            attributes: Vec::new(),
            enum_backing_type: None,
            constructor_id: None,
            flags: RuntimeClassFlags::default(),
        };
        let object = ObjectRef::new(&class);
        object.set_property("self", Value::Object(object.clone()));

        let mut frame = Frame::new(FunctionId::new(0), 1, 1);
        frame
            .registers
            .set(RegId::new(0), Value::Object(object.clone()))
            .expect("register");
        frame
            .locals
            .set(LocalId::new(0), Value::Object(object.clone()))
            .expect("local");
        let mut stack = CallStack::new();
        stack.push(frame);

        let mut state = ExecutionState::default();
        state.static_locals.insert(
            (0, "cached".to_owned()),
            ReferenceCell::new(Value::Object(object.clone())),
        );
        state.static_properties.insert(
            ("GcBox".to_owned(), "slot".to_owned()),
            Value::Object(object.clone()),
        );
        state
            .enum_cases
            .insert(("GcEnum".to_owned(), "A".to_owned()), object.clone());
        state
            .destructor_queue
            .register(object.clone(), "GcBox".to_owned(), FunctionId::new(0));

        let snapshot = gc_snapshot_from_vm_roots(&stack, &state);
        let object_id = GcEntityId::new(GcEntityKind::Object, object.id());

        assert!(snapshot.contains(object_id));
        let node = &snapshot.nodes[&object_id];
        assert!(node.roots.contains(&"frame0.r0".to_owned()));
        assert!(node.roots.contains(&"frame0.local0".to_owned()));
        assert!(
            node.roots
                .contains(&"static-property:GcBox::slot".to_owned())
        );
        assert!(node.roots.contains(&"enum-case:GcEnum::A".to_owned()));
        assert!(node.roots.contains(&"destructor-queue:0".to_owned()));
        assert!(
            snapshot
                .cycle_candidates
                .iter()
                .any(|candidate| candidate.root == object_id)
        );
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
        let readonly = execute_source(
            "<?php class Locked { public readonly $value; } $original = new Locked(); $copy = clone($original, ['value' => 1]);",
        );
        assert_eq!(readonly.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            readonly.diagnostics[0].id(),
            "E_PHP_VM_UNSUPPORTED_PROPERTY_MODIFIER"
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
    fn methods_execute_inheritance_visibility_and_static_scope() {
        let inherited = execute_source(
            "<?php class Base { public $value; function set($value) { $this->value = $value; } function get() { return $this->value; } } class Child extends Base { function label() { return 'child'; } } $child = new Child(); $child->set(9); echo $child->get(), '|', $child->label();",
        );
        assert!(inherited.status.is_success(), "{:?}", inherited.status);
        assert_eq!(inherited.output.as_bytes(), b"9|child");

        let private_scope = execute_source(
            "<?php class A { private function x() { return 'A'; } public function call() { return $this->x(); } } class B extends A { private function x() { return 'B'; } public function own() { return $this->x(); } } $b = new B(); echo $b->call(), '|', $b->own();",
        );
        assert!(
            private_scope.status.is_success(),
            "{:?}",
            private_scope.status
        );
        assert_eq!(private_scope.output.as_bytes(), b"A|B");

        let protected_scope = execute_source(
            "<?php class A { protected function x() { return 'A'; } } class B extends A { public function call() { return $this->x(); } } echo (new B())->call();",
        );
        assert!(
            protected_scope.status.is_success(),
            "{:?}",
            protected_scope.status
        );
        assert_eq!(protected_scope.output.as_bytes(), b"A");

        let static_scope = execute_source(
            "<?php class A { static function name() { return 'A'; } } class B extends A { static function own() { return self::name() . parent::name(); } } echo B::own();",
        );
        assert!(
            static_scope.status.is_success(),
            "{:?}",
            static_scope.status
        );
        assert_eq!(static_scope.output.as_bytes(), b"AA");
    }

    #[test]
    fn methods_execute_private_and_protected_property_scope() {
        let private_scope = execute_source(
            "<?php class A { private $x; public function setA($x) { $this->x = $x; } public function getA() { return $this->x; } } class B extends A { private $x; public function setB($x) { $this->x = $x; } public function getB() { return $this->x; } } $b = new B(); $b->setA('A'); $b->setB('B'); echo $b->getA(), '|', $b->getB();",
        );
        assert!(
            private_scope.status.is_success(),
            "{:?}",
            private_scope.status
        );
        assert_eq!(private_scope.output.as_bytes(), b"A|B");

        let protected_scope = execute_source(
            "<?php class A { protected $x; public function setA($x) { $this->x = $x; } } class B extends A { public function read() { return $this->x; } } $b = new B(); $b->setA('ok'); echo $b->read();",
        );
        assert!(
            protected_scope.status.is_success(),
            "{:?}",
            protected_scope.status
        );
        assert_eq!(protected_scope.output.as_bytes(), b"ok");
    }

    #[test]
    fn property_execute_defaults_readonly_static_dynamic_and_state_ops() {
        let defaults = execute_source(
            "<?php class C { public $name = 'box'; public int $count; } $c = new C(); echo $c->name, '|'; $c->count = 3; echo $c->count;",
        );
        assert!(defaults.status.is_success(), "{:?}", defaults.status);
        assert_eq!(defaults.output.as_bytes(), b"box|3");

        let uninitialized =
            execute_source("<?php class C { public int $count; } echo (new C())->count;");
        assert_eq!(uninitialized.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            uninitialized.diagnostics[0].id(),
            "E_PHP_VM_UNINITIALIZED_PROPERTY"
        );

        let readonly = execute_source(
            "<?php class C { public readonly int $x; public function set($x) { $this->x = $x; } } $c = new C(); $c->set(1); echo $c->x; $c->set(2);",
        );
        assert_eq!(readonly.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            readonly.diagnostics[0].id(),
            "E_PHP_VM_READONLY_PROPERTY_WRITE"
        );
        assert_eq!(readonly.output.as_bytes(), b"1");

        let static_property = execute_source(
            "<?php class C { public static int $count; public static $name = 'x'; } C::$count = 2; echo C::$count, '|', C::$name;",
        );
        assert!(
            static_property.status.is_success(),
            "{:?}",
            static_property.status
        );
        assert_eq!(static_property.output.as_bytes(), b"2|x");

        let dynamic = execute_source("<?php class C {} $c = new C(); $c->x = 5; echo $c->x;");
        assert!(dynamic.status.is_success(), "{:?}", dynamic.status);
        assert_eq!(dynamic.output.as_bytes(), b"5");
        assert_eq!(
            dynamic.diagnostics[0].id(),
            "E_PHP_VM_DYNAMIC_PROPERTY_DEPRECATED"
        );

        let state_ops = execute_source(
            "<?php class C { public $x = 0; public $y = null; } $c = new C(); echo isset($c->x), isset($c->y), empty($c->x), empty($c->missing); unset($c->x); echo isset($c->x), empty($c->x);",
        );
        assert!(state_ops.status.is_success(), "{:?}", state_ops.status);
        assert_eq!(state_ops.output.as_bytes(), b"1111");
    }

    #[test]
    fn methods_report_visibility_errors() {
        let private = execute_source(
            "<?php class Secret { private function hidden() { return 1; } } (new Secret())->hidden();",
        );
        assert_eq!(private.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            private.diagnostics[0].id(),
            "E_PHP_VM_PRIVATE_METHOD_ACCESS"
        );

        let protected = execute_source(
            "<?php class Secret { protected function hidden() { return 1; } } (new Secret())->hidden();",
        );
        assert_eq!(protected.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            protected.diagnostics[0].id(),
            "E_PHP_VM_PROTECTED_METHOD_ACCESS"
        );

        let private_property = execute_source(
            "<?php class Secret { private $hidden; public function __construct() { $this->hidden = 1; } } echo (new Secret())->hidden;",
        );
        assert_eq!(
            private_property.status.exit_status(),
            ExitStatus::RuntimeError
        );
        assert_eq!(
            private_property.diagnostics[0].id(),
            "E_PHP_VM_PRIVATE_PROPERTY_ACCESS"
        );

        let protected_property = execute_source(
            "<?php class Secret { protected $hidden; public function __construct() { $this->hidden = 1; } } echo (new Secret())->hidden;",
        );
        assert_eq!(
            protected_property.status.exit_status(),
            ExitStatus::RuntimeError
        );
        assert_eq!(
            protected_property.diagnostics[0].id(),
            "E_PHP_VM_PROTECTED_PROPERTY_ACCESS"
        );
    }

    #[test]
    fn methods_classify_visibility_static_and_this_gaps() {
        let this_outside = execute_source("<?php echo $this;");
        assert_eq!(this_outside.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            this_outside.diagnostics[0].id(),
            "E_PHP_VM_THIS_OUTSIDE_METHOD"
        );
    }

    #[test]
    fn expressions_modulo_coerces_numeric_operands() {
        let result = execute_source("<?php echo 5.5 % 2;");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1");
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
    fn references_execute_chains_rebinding_and_unset_name_semantics() {
        let chain = execute_source("<?php $a = 1; $b =& $a; $c =& $b; $c = 3; echo $a, $b, $c;");

        assert!(chain.status.is_success(), "{:?}", chain.status);
        assert_eq!(chain.output.as_bytes(), b"333");

        let rebind =
            execute_source("<?php $a = 1; $b = 2; $c =& $a; $c =& $b; $c = 4; echo $a, $b, $c;");

        assert!(rebind.status.is_success(), "{:?}", rebind.status);
        assert_eq!(rebind.output.as_bytes(), b"144");

        let unset_name = execute_source(
            "<?php $a = 1; $b =& $a; unset($a); $b = 2; echo isset($a) ? 'bad' : 'unset', '|', $b;",
        );

        assert!(unset_name.status.is_success(), "{:?}", unset_name.status);
        assert_eq!(unset_name.output.as_bytes(), b"unset|2");
    }

    #[test]
    fn references_reject_unsupported_property_category_with_stable_id() {
        let object_ref = php_ir::lower_frontend_result(
            &php_semantics::analyze_source(
                "<?php class Box { public $p = 1; } $box = new Box(); $alias =& $box->p;",
            ),
            php_ir::LoweringOptions::default(),
        );
        assert_eq!(
            object_ref.diagnostics[0].id,
            "E_PHP_IR_UNSUPPORTED_PROPERTY_REFERENCE"
        );
    }

    #[test]
    fn lvalue_array_element_references_bind_selected_cell() {
        let read_through =
            execute_source("<?php $a = []; $b = 1; $a[\"x\"] =& $b; $b = 3; echo $a[\"x\"]; ");

        assert!(
            read_through.status.is_success(),
            "{:?}",
            read_through.status
        );
        assert_eq!(read_through.output.as_bytes(), b"3");

        let write_through = execute_source(
            "<?php $a = []; $b = 1; $a[\"x\"] =& $b; $a[\"x\"] = 4; echo $b, \"|\", $a[\"x\"];",
        );

        assert!(
            write_through.status.is_success(),
            "{:?}",
            write_through.status
        );
        assert_eq!(write_through.output.as_bytes(), b"4|4");
    }

    #[test]
    fn lvalue_array_append_by_reference_binds_new_element() {
        let result = execute_source("<?php $a = []; $b = 2; $a[] =& $b; $b = 5; echo $a[0];");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"5");
    }

    #[test]
    fn lvalue_nested_dim_increment_and_auto_creation_execute() {
        let nested = execute_source(
            "<?php $a = [\"x\" => [\"y\" => 1]]; $a[\"x\"][\"y\"]++; echo $a[\"x\"][\"y\"];",
        );

        assert!(nested.status.is_success(), "{:?}", nested.status);
        assert_eq!(nested.output.as_bytes(), b"2");

        let auto_create =
            execute_source("<?php $a = []; $a[\"x\"][\"y\"] = 6; echo $a[\"x\"][\"y\"];");

        assert!(auto_create.status.is_success(), "{:?}", auto_create.status);
        assert_eq!(auto_create.output.as_bytes(), b"6");
    }

    #[test]
    fn lvalue_unset_dimension_preserves_other_elements_and_alias_cells() {
        let result = execute_source(
            "<?php $a = [\"x\" => 1, \"y\" => 2]; $r =& $a[\"x\"]; unset($a[\"x\"]); $r = 7; echo isset($a[\"x\"]) ? \"bad\" : \"unset\", \"|\", $a[\"y\"], \"|\", $r;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"unset|2|7");
    }

    #[test]
    fn lvalue_array_element_reference_separates_cow_copy() {
        let result = execute_source(
            "<?php $a = [\"x\" => 1]; $b = $a; $r = 9; $b[\"x\"] =& $r; echo $a[\"x\"], \"|\", $b[\"x\"];",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|9");
    }

    #[test]
    fn lvalue_trace_records_array_dimension_paths() {
        let result = execute_source_with_options(
            "<?php $a = []; $b = 1; $a[\"x\"] =& $b;",
            VmOptions {
                trace: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert!(
            result.trace.iter().any(
                |event| event.contains("lvalue operation=bind-reference-dim")
                    && event.contains("path=[string(\"x\")]")
            ),
            "{:?}",
            result.trace
        );
    }

    #[test]
    fn trace_runtime_records_reference_cow_snapshot() {
        let result = execute_source_with_options(
            "<?php $a = [\"x\" => 1]; $b = $a; $r = 9; $b[\"x\"] =& $r; echo $a[\"x\"], \"|\", $b[\"x\"];",
            VmOptions {
                trace_runtime: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|9");
        let events = runtime_trace_events(&result.trace);
        assert_eq!(
            events,
            vec![
                "lvalue operation=bind-reference-dim local=1 path=[string(\"x\")]".to_owned(),
                "gc-roots roots=0 entities=0 cycle_candidates=0".to_owned(),
            ]
        );
        assert_trace_is_normalized(&result.trace);
    }

    #[test]
    fn trace_runtime_records_foreach_snapshot() {
        let result = execute_source_with_options(
            "<?php foreach ([\"a\" => 1, \"b\" => 2] as $key => $value) { echo $key, $value; }",
            VmOptions {
                trace_runtime: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"a1b2");
        let events = runtime_trace_events(&result.trace);
        assert_eq!(
            events,
            vec![
                "foreach init iterator=r5 kind=snapshot".to_owned(),
                "foreach next iterator=r5 status=value key=String(\"a\") value=Int(1)".to_owned(),
                "foreach next iterator=r5 status=value key=String(\"b\") value=Int(2)".to_owned(),
                "foreach next iterator=r5 status=done".to_owned(),
                "gc-roots roots=0 entities=0 cycle_candidates=0".to_owned(),
            ]
        );
        assert_trace_is_normalized(&result.trace);
    }

    #[test]
    fn trace_runtime_records_generator_suspend_snapshot() {
        let result = execute_source_with_options(
            "<?php function gen() { yield \"k\" => \"v\"; } $g = gen(); echo $g->current();",
            VmOptions {
                trace_runtime: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"v");
        let events = runtime_trace_events(&result.trace);
        assert_eq!(
            events,
            vec![
                "generator state function=1 transition=created->running".to_owned(),
                "generator suspend function=1 key=String(\"k\") value=String(\"v\")".to_owned(),
                "gc-roots roots=0 entities=0 cycle_candidates=0".to_owned(),
            ]
        );
        assert_trace_is_normalized(&result.trace);
    }

    #[test]
    fn trace_runtime_records_fiber_suspend_snapshot() {
        let result = execute_source_with_options(
            "<?php $fiber = new Fiber(function() { echo \"a\"; Fiber::suspend(\"s\"); echo \"b\"; }); echo $fiber->start(); echo \"|\"; $fiber->resume(\"r\");",
            VmOptions {
                trace_runtime: true,
                ..VmOptions::default()
            },
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"as|b");
        let events = runtime_trace_events(&result.trace);
        assert_eq!(
            events,
            vec![
                "fiber start transition=not-started->running".to_owned(),
                "fiber suspend transition=running->suspended state=Running value=String(\"s\")"
                    .to_owned(),
                "fiber start transition=running->suspended value=String(\"s\")".to_owned(),
                "fiber resume transition=suspended->running input=String(\"r\")".to_owned(),
                "fiber resume transition=running->terminated".to_owned(),
                "gc-roots roots=0 entities=0 cycle_candidates=0".to_owned(),
            ]
        );
        assert_trace_is_normalized(&result.trace);
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
    fn call_binding_named_defaults_unpacks_variadics_and_callables_execute() {
        let result = execute_source(
            "<?php function joiner($first, $second = \"B\", ...$rest) { echo $first, \"|\", $second, \"|\", $rest[0], \"|\", $rest[\"third\"]; } joiner(\"A\", ...[\"C\", \"D\"], third: \"E\"); echo \";\"; function defaults($a = \"A\", $b = \"B\", $c = \"C\") { echo $a, $b, $c; } defaults(c: \"Z\"); echo \";\"; $len = strlen(...); echo $len(...[\"hello\"]);",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"A|C|D|E;ABZ;5");
    }

    #[test]
    fn call_binding_named_by_ref_arguments_mutate_caller_local() {
        let result = execute_source(
            "<?php function set_named(&$value, $next = 5) { $value = $next; } $a = 1; set_named(value: $a, next: 7); echo $a;",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"7");
    }

    #[test]
    fn call_binding_named_argument_errors_are_stable() {
        let unknown =
            execute_source("<?php function one($value) { return $value; } one(missing: 1);");
        assert_eq!(unknown.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(unknown.diagnostics[0].id(), "E_PHP_VM_UNKNOWN_NAMED_ARG");

        let duplicate =
            execute_source("<?php function one($value) { return $value; } one(1, value: 2);");
        assert_eq!(duplicate.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            duplicate.diagnostics[0].id(),
            "E_PHP_VM_DUPLICATE_NAMED_ARG"
        );

        let positional_after_named =
            execute_source("<?php function pair($a, $b) { return $a; } pair(a: 1, 2);");
        assert_eq!(
            positional_after_named.status.exit_status(),
            ExitStatus::RuntimeError
        );
        assert_eq!(
            positional_after_named.diagnostics[0].id(),
            "E_PHP_VM_POSITIONAL_AFTER_NAMED_ARG"
        );

        let unpack_non_array =
            execute_source("<?php function one($value) { return $value; } one(...4);");
        assert_eq!(
            unpack_non_array.status.exit_status(),
            ExitStatus::RuntimeError
        );
        assert_eq!(
            unpack_non_array.diagnostics[0].id(),
            "E_PHP_VM_UNPACK_NON_ARRAY"
        );
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
    fn closures_capture_by_reference_and_static_locals_execute() {
        let by_ref = execute_source(
            "<?php $x = 1; $f = function() use (&$x) { return $x; }; $x = 4; echo $f();",
        );
        assert!(by_ref.status.is_success(), "{:?}", by_ref.status);
        assert_eq!(by_ref.output.as_bytes(), b"4");

        let write_through =
            execute_source("<?php $x = 1; $f = function() use (&$x) { $x = 7; }; $f(); echo $x;");
        assert!(
            write_through.status.is_success(),
            "{:?}",
            write_through.status
        );
        assert_eq!(write_through.output.as_bytes(), b"7");

        let static_local = execute_source(
            "<?php function next_id() { static $x = 0; $x++; return $x; } echo next_id(), '|', next_id();",
        );
        assert!(
            static_local.status.is_success(),
            "{:?}",
            static_local.status
        );
        assert_eq!(static_local.output.as_bytes(), b"1|2");

        let closure_static = execute_source(
            "<?php $f = function() { static $x = 0; $x++; return $x; }; echo $f(), '|', $f();",
        );
        assert!(
            closure_static.status.is_success(),
            "{:?}",
            closure_static.status
        );
        assert_eq!(closure_static.output.as_bytes(), b"1|2");

        let by_ref_return = execute_source(
            "<?php function &counter() { static $x = 0; return $x; } $a =& counter(); $a = 5; echo counter();",
        );
        assert!(
            by_ref_return.status.is_success(),
            "{:?}",
            by_ref_return.status
        );
        assert_eq!(by_ref_return.output.as_bytes(), b"5");
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
            "<?php try { throw new Exception(\"boom\"); } catch (Exception $e) { echo \"caught:\", $e->getMessage(); }",
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
    fn exceptions_catch_throwable_interface() {
        let result = execute_source(
            "<?php try { throw new Exception(\"boom\"); } catch (Throwable $e) { echo \"throwable:\", $e->getMessage(); }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"throwable:boom");
    }

    #[test]
    fn exceptions_catch_error_parent_for_type_error() {
        let result = execute_source(
            "<?php try { throw new TypeError(\"bad\"); } catch (Error $e) { echo \"error:\", $e->getMessage(); }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"error:bad");
    }

    #[test]
    fn exceptions_skip_nonmatching_catch_and_run_finally() {
        let result = execute_source(
            "<?php try { try { throw new TypeError(\"bad\"); } catch (Exception $e) { echo \"wrong\"; } finally { echo \"finally|\"; } } catch (Throwable $e) { echo \"outer:\", $e->getMessage(); }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"finally|outer:bad");
    }

    #[test]
    fn exceptions_internal_throwable_hierarchy_supports_instanceof() {
        let result = execute_source(
            "<?php $e = new TypeError(\"bad\"); echo ($e instanceof Throwable) ? \"throwable|\" : \"no|\"; echo ($e instanceof Error) ? \"error|\" : \"no|\"; echo ($e instanceof Exception) ? \"exception\" : \"not-exception\";",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"throwable|error|not-exception");
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
    fn foreach_by_ref_executes_local_array_and_lingering_reference() {
        let result = execute_source(
            "<?php $items = [1, 2]; foreach ($items as &$value) { $value = $value + 10; } unset($value); foreach ($items as $value) { echo $value; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1112");

        let lingering = execute_source(
            "<?php $items = [1, 2]; foreach ($items as &$value) { } $value = 9; echo $items[1];",
        );

        assert!(lingering.status.is_success(), "{:?}", lingering.status);
        assert_eq!(lingering.output.as_bytes(), b"9");
    }

    #[test]
    fn foreach_by_ref_executes_key_value_and_appended_entries() {
        let key_value = execute_source(
            "<?php $items = [\"a\" => 1, \"b\" => 2]; foreach ($items as $key => &$value) { echo $key, \":\", $value, \";\"; $value = $value + 1; } unset($value); echo \"|\", $items[\"a\"], \":\", $items[\"b\"];",
        );

        assert!(key_value.status.is_success(), "{:?}", key_value.status);
        assert_eq!(key_value.output.as_bytes(), b"a:1;b:2;|2:3");

        let appended = execute_source(
            "<?php $items = [1, 2]; $done = false; foreach ($items as &$value) { echo $value; if (!$done) { $items[] = 3; $done = true; } } unset($value);",
        );

        assert!(appended.status.is_success(), "{:?}", appended.status);
        assert_eq!(appended.output.as_bytes(), b"123");
    }

    #[test]
    fn foreach_by_value_snapshots_reference_elements_without_aliasing() {
        let result = execute_source(
            "<?php $items = [1]; $alias =& $items[0]; foreach ($items as $value) { $value = 9; echo $items[0], \":\", $alias, \":\", $value; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1:1:9");
    }

    #[test]
    fn foreach_by_ref_nonlocal_source_is_stable_known_gap() {
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
    fn generator_call_is_lazy_and_foreach_runs_to_first_yield() {
        let result = execute_source(
            "<?php function gen() { echo 'body|'; yield 1; } $g = gen(); echo 'created|'; foreach ($g as $value) { echo 'v:', $value, '|'; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"created|body|v:1|");
    }

    #[test]
    fn generator_foreach_uses_key_and_value() {
        let result = execute_source(
            "<?php function gen() { yield 'a' => 7; } foreach (gen() as $key => $value) { echo $key, ':', $value; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"a:7");
    }

    #[test]
    fn generator_methods_use_same_state_handle() {
        let result = execute_source(
            "<?php function gen() { yield 'a' => 7; } $g = gen(); echo $g->valid() ? 'T' : 'F'; echo '|', $g->current(), '|', $g->key(); $g->next(); echo '|', $g->valid() ? 'T' : 'F';",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"T|7|a|F");
    }

    #[test]
    fn generator_get_return_after_no_yield_completion() {
        let result = execute_source(
            "<?php function gen() { return 9; yield 1; } $g = gen(); $g->rewind(); echo $g->getReturn();",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"9");
    }

    #[test]
    fn generator_send_resumes_with_yield_expression_value() {
        let result = execute_source(
            "<?php function gen() { $value = yield 1; echo $value; } $g = gen(); $g->rewind(); $g->send(7);",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"7");
    }

    #[test]
    fn generator_throw_injects_exception_at_suspend_point() {
        let result = execute_source(
            "<?php function gen() { try { yield 1; } catch (Exception $e) { echo $e->getMessage(); } } $g = gen(); $g->rewind(); $g->throw(new Exception('boom'));",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"boom");
    }

    #[test]
    fn generator_foreach_resumes_to_return_value() {
        let result = execute_source(
            "<?php function gen() { yield 1; return 9; } $g = gen(); foreach ($g as $value) { echo $value, '|'; } echo $g->getReturn();",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|9");
    }

    #[test]
    fn generator_yield_from_array_delegates_keys_and_values() {
        let result = execute_source(
            "<?php function gen() { yield from ['a' => 1, 'b' => 2]; } foreach (gen() as $key => $value) { echo $key, ':', $value, ';'; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"a:1;b:2;");
    }

    #[test]
    fn generator_yield_from_generator_returns_delegate_return_value() {
        let result = execute_source(
            "<?php function inner() { yield 'x' => 3; return 9; } function outer() { $result = yield from inner(); echo 'return:', $result; } foreach (outer() as $key => $value) { echo $key, ':', $value, '|'; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"x:3|return:9");
    }

    #[test]
    fn generator_yield_from_runs_finally_on_completion() {
        let result = execute_source(
            "<?php function gen() { try { yield from [1]; } finally { echo 'cleanup'; } } foreach (gen() as $value) { echo $value, '|'; }",
        );

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"1|cleanup");
    }

    #[test]
    fn generator_get_return_before_completion_is_runtime_error() {
        let result = execute_source(
            "<?php function gen() { yield 1; return 9; } $g = gen(); $g->rewind(); echo $g->getReturn();",
        );

        assert_eq!(result.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            result.diagnostics[0].id(),
            "E_PHP_VM_GENERATOR_GET_RETURN_BEFORE_CLOSE"
        );
    }

    #[test]
    fn normal_functions_are_not_treated_as_generators() {
        let result = execute_source("<?php function f() { return 3; } echo f();");

        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"3");
    }

    #[test]
    fn eval_executes_code_and_returns_value() {
        let result = execute_source(
            "<?php echo \"before|\", eval('echo \"inner|\"; return 7;'), \"|after\";",
        );

        assert!(result.status.is_success(), "{:#?}", result);
        assert_eq!(result.output.as_bytes(), b"before|inner|7|after");
    }

    #[test]
    fn eval_shares_top_level_locals() {
        let result = execute_source(
            "<?php $message = \"parent\"; eval('$message = $message . \"|eval\";'); echo $message;",
        );

        assert!(result.status.is_success(), "{:#?}", result);
        assert_eq!(result.output.as_bytes(), b"parent|eval");
    }

    #[test]
    fn eval_parse_errors_are_runtime_diagnostics() {
        let result = execute_source("<?php eval('if (');");

        assert!(!result.status.is_success());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == "E_PHP_VM_EVAL_PARSE_ERROR"),
            "diagnostics: {:#?}",
            result.diagnostics
        );
    }

    #[test]
    fn eval_declarations_are_specific_known_gap() {
        let result = execute_source("<?php eval('function prompt39_eval_fn() { return 1; }');");

        assert!(!result.status.is_success());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == "E_PHP_VM_EVAL_DECLARATION_GAP"),
            "diagnostics: {:#?}",
            result.diagnostics
        );
    }

    #[test]
    fn eval_recursion_limit_is_runtime_diagnostic() {
        let mut code = "echo \"done\";".to_owned();
        for _ in 0..=MAX_EVAL_DEPTH {
            code = format!("eval({code:?});");
        }
        let source = format!("<?php {code}");
        let result = execute_source(&source);

        assert!(!result.status.is_success());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == "E_PHP_VM_EVAL_RECURSION_LIMIT"),
            "diagnostics: {:#?}",
            result.diagnostics
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
    fn call_by_ref_param_mutates_caller_local() {
        let result = execute_source(
            "<?php function inc_ref(&$x) { $x = $x + 1; } $a = 1; inc_ref($a); echo $a;",
        );
        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"2");
    }

    #[test]
    fn call_by_ref_return_binds_to_caller_local() {
        let result = execute_source(
            "<?php function &identity_ref(&$x) { return $x; } $a = 1; $b =& identity_ref($a); $b = 4; echo $a, '|', $b;",
        );
        assert!(result.status.is_success(), "{:?}", result.status);
        assert_eq!(result.output.as_bytes(), b"4|4");
    }

    #[test]
    fn call_by_ref_errors_for_temporaries() {
        let arg =
            execute_source("<?php function inc_ref(&$x) { $x = $x + 1; } $a = 1; inc_ref($a + 1);");
        assert_eq!(arg.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(
            arg.diagnostics[0].id(),
            "E_PHP_VM_BY_REF_ARG_NOT_REFERENCEABLE"
        );

        let ret = execute_source("<?php function &bad_ref() { return 1; } $x =& bad_ref();");
        assert_eq!(ret.status.exit_status(), ExitStatus::RuntimeError);
        assert_eq!(ret.diagnostics[0].id(), "E_PHP_VM_BY_REF_RETURN_TEMPORARY");
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

    fn runtime_trace_events(trace: &[String]) -> Vec<String> {
        trace
            .iter()
            .filter_map(|line| {
                line.split_once(" runtime ")
                    .map(|(_, event)| event.to_owned())
            })
            .collect()
    }

    fn assert_trace_is_normalized(trace: &[String]) {
        assert!(
            trace.iter().all(|line| {
                !line.contains("0x")
                    && !line.contains(" at ")
                    && !line.contains("id:")
                    && !line.contains("id=")
            }),
            "{trace:#?}"
        );
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
