//! Iterative Dense activation scheduling and caller resumption.

use super::prelude::*;
use super::result::FrameOutcome;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DenseResumeState {
    pub(super) function_id: FunctionId,
    pub(super) block_index: u32,
    pub(super) instruction_offset: usize,
    pub(super) call_instruction_index: u32,
    pub(super) destination: RegId,
    pub(super) frame_index: usize,
    pub(super) foreach_iterators: HashMap<RegId, ForeachIterator>,
    pub(super) diagnostics: Vec<RuntimeDiagnostic>,
    pub(super) steps: usize,
    pub(super) post_return: DensePostReturn,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum DensePostReturn {
    #[default]
    None,
    Constructor {
        object: ObjectRef,
        class: Rc<php_ir::module::ClassEntry>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DenseActivationSignal {
    pub(super) callee: FunctionId,
    pub(super) direct: DirectCallOwned,
    pub(super) resume: DenseResumeState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DirectCallOwned {
    pub(super) strict_types: bool,
    pub(super) span: Option<IrSpan>,
    pub(super) receiver: Option<ObjectRef>,
    pub(super) class_context: CompactClassContext,
    pub(super) move_source: Option<RegId>,
}

#[derive(Debug)]
pub(super) struct DenseFrameCompletion {
    pub(super) outcome: FrameOutcome,
    pub(super) diagnostics: Vec<RuntimeDiagnostic>,
}

#[derive(Debug)]
pub(super) enum DenseActivationResult {
    Boundary(VmResult),
    Frame(DenseFrameCompletion),
    Transfer(DenseActivationSignal),
}

impl From<VmResult> for DenseActivationResult {
    fn from(result: VmResult) -> Self {
        Self::Boundary(result)
    }
}

impl DenseActivationResult {
    pub(super) fn throwing() -> Self {
        Self::Frame(DenseFrameCompletion {
            outcome: FrameOutcome::Throw,
            diagnostics: Vec::new(),
        })
    }
}

impl Vm {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_dense_direct_function_call<'a>(
        &self,
        opcode: DenseOpcode,
        bare_positional_shape: bool,
        target: &FunctionCallCacheTarget,
        plan: Option<&DenseExecutionPlan>,
        compiled: &CompiledUnit,
        args: &'a [DenseCallArg],
        dst: u32,
        instruction: &DenseInstruction,
        move_plan: Option<&crate::last_use::LastUseMovePlan>,
        dense_instruction_index: u32,
        frame_index: usize,
    ) -> Option<DirectCall<'a>> {
        let plan = plan?;
        let FunctionCallCacheTarget::CurrentUnit {
            unit_identity,
            function,
        } = target
        else {
            return None;
        };
        if opcode != DenseOpcode::CallFunction
            || !bare_positional_shape
            || *unit_identity != compiled.cache_identity()
            || !matches!(
                plan.function_plan(function.index()),
                Some(DenseFunctionPlan::Dense)
            )
        {
            return None;
        }
        let callee = compiled.unit().functions.get(function.index())?;
        let callee_meta = plan.call_shape_meta.get(function.index()).copied()?;
        if args.len() != callee.params.len()
            || !callee_meta.params_bind_direct
            || !callee_meta.elide_frame_args
            || args.len() > 8
        {
            return None;
        }
        let move_source = args.iter().find_map(|arg| {
            (arg.value.kind == DenseOperandKind::Register
                && move_plan.is_some_and(|plan| {
                    plan.is_move_eligible(dense_instruction_index, arg.value.index)
                }))
            .then(|| RegId::new(arg.value.index))
        });
        self.record_counter_dense_call_bare_args_hit();
        Some(DirectCall {
            caller_frame: frame_index,
            argument_sources: DirectArgumentSources::Dense(args),
            destination: CallDestination::Register(RegId::new(dst)),
            strict_types: compiled.unit().strict_types,
            span: plan.unit.spans.get(instruction.span.index()).copied(),
            receiver: None,
            class_context: CompactClassContext::default(),
            move_source,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn dense_call_activation_signal(
        callee: FunctionId,
        direct: DirectCall<'_>,
        function_id: FunctionId,
        block_index: u32,
        instruction_offset: usize,
        call_instruction_index: u32,
        frame_index: usize,
        foreach_iterators: HashMap<RegId, ForeachIterator>,
        diagnostics: Vec<RuntimeDiagnostic>,
        steps: usize,
    ) -> DenseActivationResult {
        let CallDestination::Register(destination) = direct.destination else {
            unreachable!("discard direct calls stay on the complex path");
        };
        DenseActivationResult::Transfer(DenseActivationSignal {
            callee,
            direct: DirectCallOwned {
                strict_types: direct.strict_types,
                span: direct.span,
                receiver: direct.receiver,
                class_context: direct.class_context,
                move_source: direct.move_source,
            },
            resume: DenseResumeState {
                function_id,
                block_index,
                instruction_offset,
                call_instruction_index,
                destination,
                frame_index,
                foreach_iterators,
                diagnostics,
                steps,
                post_return: DensePostReturn::None,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn dense_constructor_activation_signal(
        callee: FunctionId,
        direct: DirectCall<'_>,
        object: ObjectRef,
        class: Rc<php_ir::module::ClassEntry>,
        function_id: FunctionId,
        block_index: u32,
        instruction_offset: usize,
        call_instruction_index: u32,
        frame_index: usize,
        foreach_iterators: HashMap<RegId, ForeachIterator>,
        diagnostics: Vec<RuntimeDiagnostic>,
        steps: usize,
    ) -> DenseActivationResult {
        let DenseActivationResult::Transfer(mut signal) = Self::dense_call_activation_signal(
            callee,
            direct,
            function_id,
            block_index,
            instruction_offset,
            call_instruction_index,
            frame_index,
            foreach_iterators,
            diagnostics,
            steps,
        ) else {
            unreachable!();
        };
        signal.resume.post_return = DensePostReturn::Constructor { object, class };
        DenseActivationResult::Transfer(signal)
    }

    pub(super) fn execute_bytecode_function(
        &self,
        request: DenseExecutionRequest<'_, '_>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> VmResult {
        let compiled = request.compiled;
        let dense = request.dense;
        let plan = request.plan;
        let mut result = self.execute_dense_activation(request, output, stack, state);
        let mut resumes = Vec::new();
        loop {
            if let DenseActivationResult::Transfer(signal) = result {
                self.record_counter_dense_activation_transfer();
                let Some(plan) = plan else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_PLAN_MISSING: direct activation requires a dense plan",
                    );
                };
                let Some(caller_dense) = plan.unit.functions.get(signal.resume.function_id.index())
                else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_CALLER_MISSING: caller dense function is missing",
                    );
                };
                let Some(call_instruction) = caller_dense
                    .instructions
                    .get(signal.resume.call_instruction_index as usize)
                else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_SITE_MISSING: caller call site is missing",
                    );
                };
                let Some(args) = dense_activation_call_args(call_instruction) else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_SITE_SHAPE: resumed site is not a function call",
                    );
                };
                let Some(dense_function) = plan.unit.functions.get(signal.callee.index()) else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_CALLEE_MISSING: callee dense function is missing",
                    );
                };
                let Some(ir_function) = compiled.unit().functions.get(signal.callee.index()) else {
                    return self.runtime_error(
                        output,
                        compiled,
                        stack,
                        "E_PHP_VM_DENSE_ACTIVATION_CALLEE_IR_MISSING: callee IR function is missing",
                    );
                };
                let direct_call = DirectCall {
                    caller_frame: signal.resume.frame_index,
                    argument_sources: DirectArgumentSources::Dense(args),
                    destination: CallDestination::Register(signal.resume.destination),
                    strict_types: signal.direct.strict_types,
                    span: signal.direct.span,
                    receiver: signal.direct.receiver,
                    class_context: signal.direct.class_context,
                    move_source: signal.direct.move_source,
                };
                resumes.push(signal.resume);
                result = self.execute_dense_activation(
                    DenseExecutionRequest {
                        compiled,
                        dense,
                        plan: Some(plan),
                        dense_function,
                        ir_function,
                        function_id: signal.callee,
                        call: FunctionCall::new(Vec::new(), Vec::new()),
                        direct_call: Some(direct_call),
                        resume: None,
                    },
                    output,
                    stack,
                    state,
                );
                continue;
            }

            let Some(mut resume) = resumes.pop() else {
                return match result {
                    DenseActivationResult::Boundary(result) => result,
                    DenseActivationResult::Frame(completion) => frame_completion_result(completion),
                    DenseActivationResult::Transfer(_) => unreachable!(),
                };
            };
            let DenseActivationResult::Frame(mut completion) = result else {
                let DenseActivationResult::Boundary(result) = result else {
                    unreachable!();
                };
                stack.pop_frame_recycle(resume.frame_index);
                for pending in resumes.into_iter().rev() {
                    stack.pop_frame_recycle(pending.frame_index);
                }
                return result;
            };
            let (value, explicit) = match completion.outcome {
                super::result::FrameOutcome::Return { value, explicit } => (value, explicit),
                outcome => {
                    stack.pop_frame_recycle(resume.frame_index);
                    for pending in resumes.into_iter().rev() {
                        stack.pop_frame_recycle(pending.frame_index);
                    }
                    return frame_completion_result(DenseFrameCompletion {
                        outcome,
                        diagnostics: completion.diagnostics,
                    });
                }
            };
            let _ = explicit;
            resume
                .diagnostics
                .extend(std::mem::take(&mut completion.diagnostics));
            let Some(caller_dense) =
                plan.and_then(|plan| plan.unit.functions.get(resume.function_id.index()))
            else {
                return self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_DENSE_ACTIVATION_RESUME_MISSING: caller function is missing",
                );
            };
            let Some(ir_function) = compiled.unit().functions.get(resume.function_id.index())
            else {
                return self.runtime_error(
                    output,
                    compiled,
                    stack,
                    "E_PHP_VM_DENSE_ACTIVATION_RESUME_IR_MISSING: caller IR is missing",
                );
            };
            let return_value = match std::mem::take(&mut resume.post_return) {
                DensePostReturn::None => value.unwrap_or(Value::Null),
                DensePostReturn::Constructor { object, class } => {
                    self.register_destructor_if_needed(compiled, &class, object.clone(), state);
                    Value::Object(object)
                }
            };
            let write_result = match stack.frame_mut(resume.frame_index) {
                Some(frame) => frame
                    .registers
                    .set(resume.destination, return_value)
                    .map_err(|error| error.to_string()),
                None => Err("caller frame is not active".to_owned()),
            };
            if let Err(message) = write_result {
                return self.runtime_error(output, compiled, stack, message);
            }
            if let Some(instruction) = caller_dense
                .instructions
                .get(resume.call_instruction_index as usize)
                && let Some(args) = dense_activation_call_args(instruction)
                && let Err(message) = unset_consumed_dense_call_arg_registers_at_frame(
                    stack,
                    resume.frame_index,
                    args,
                    Some(resume.destination),
                )
            {
                return self.runtime_error(output, compiled, stack, message);
            }
            result = self.execute_dense_activation(
                DenseExecutionRequest {
                    compiled,
                    dense,
                    plan,
                    dense_function: caller_dense,
                    ir_function,
                    function_id: resume.function_id,
                    call: FunctionCall::new(Vec::new(), Vec::new()),
                    direct_call: None,
                    resume: Some(resume),
                },
                output,
                stack,
                state,
            );
        }
    }
}

fn dense_activation_call_args(instruction: &DenseInstruction) -> Option<&[DenseCallArg]> {
    match &instruction.operands {
        DenseOperands::Call { args, .. }
        | DenseOperands::MethodCall { args, .. }
        | DenseOperands::StaticCall { args, .. }
        | DenseOperands::NewObject { args, .. } => Some(args),
        _ => None,
    }
}

fn frame_completion_result(completion: DenseFrameCompletion) -> VmResult {
    let mut result = match completion.outcome {
        FrameOutcome::Return { value, explicit } => {
            let mut result = VmResult::success_no_output(value);
            result.returned_explicitly = explicit;
            result
        }
        FrameOutcome::Throw => VmResult::propagating_exception(OutputBuffer::new()),
        FrameOutcome::Exit(code) => VmResult::script_exit(OutputBuffer::new(), code, false),
        FrameOutcome::Yield(yielded) => {
            let mut result = VmResult::success_no_output(None);
            result.yielded = Some(yielded);
            result
        }
        FrameOutcome::FiberSuspend(suspension) => {
            let mut result = VmResult::success_no_output(None);
            result.fiber_suspension = Some(suspension);
            result
        }
    };
    result.diagnostics = completion.diagnostics;
    result
}
