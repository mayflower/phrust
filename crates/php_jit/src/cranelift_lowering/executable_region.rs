use super::*;
use crate::region_ir::{RegionPropertyName, RegionSemanticOp};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
struct NativeFragmentLayout {
    id: u32,
    blocks: BTreeSet<BlockId>,
    normal_entries: BTreeSet<BlockId>,
    external_targets: BTreeSet<BlockId>,
    locals: BTreeSet<LocalId>,
    registers: BTreeSet<RegId>,
    stored_registers: BTreeSet<RegId>,
}

#[derive(Clone, Debug)]
struct NativeFunctionFragmentLayout {
    fragments: Vec<NativeFragmentLayout>,
    block_owner: BTreeMap<BlockId, u32>,
    resume_owner: BTreeMap<i32, u32>,
    frame: NativeFragmentFrameLayout,
    register_liveness: NativeRegisterLiveness,
}

#[derive(Clone, Debug)]
struct NativeFragmentFrameLayout {
    local_slots: BTreeMap<LocalId, usize>,
    register_slots: BTreeMap<(u32, RegId), usize>,
    shared_register_slots: usize,
    scratch_register_slots: usize,
    value_slots: usize,
}

#[derive(Clone, Copy)]
struct NativeFragmentDefinition<'a> {
    layout: &'a NativeFunctionFragmentLayout,
    fragment: &'a NativeFragmentLayout,
    functions: &'a BTreeMap<u32, FuncId>,
}

impl NativeFragmentFrameLayout {
    fn for_fragments(
        region: &RegionGraph,
        fragments: &[NativeFragmentLayout],
        shared_registers: &BTreeSet<RegId>,
    ) -> Self {
        let mut locals = (0..region.local_count)
            .map(LocalId::new)
            .collect::<BTreeSet<_>>();
        for block in &region.blocks {
            locals.extend(block.entry_state_locals.iter().copied());
            locals.extend(block.terminator_state_locals.iter().copied());
            locals.extend(block.terminator_live_locals.iter().copied());
            for instruction in &block.instructions {
                locals.extend(instruction.live_locals.iter().copied());
            }
        }
        let local_slots = locals
            .into_iter()
            .enumerate()
            .map(|(slot, local)| (local, slot))
            .collect::<BTreeMap<_, _>>();
        let shared_base = local_slots.len();
        let shared_slots = shared_registers
            .iter()
            .enumerate()
            .map(|(slot, register)| (*register, shared_base.saturating_add(slot)))
            .collect::<BTreeMap<_, _>>();
        let scratch_base = shared_base.saturating_add(shared_slots.len());
        let mut register_slots = BTreeMap::new();
        let mut scratch_register_slots = 0_usize;
        for fragment in fragments {
            let mut next_scratch = 0_usize;
            for register in &fragment.stored_registers {
                let slot = shared_slots.get(register).copied().unwrap_or_else(|| {
                    let slot = scratch_base.saturating_add(next_scratch);
                    next_scratch = next_scratch.saturating_add(1);
                    slot
                });
                register_slots.insert((fragment.id, *register), slot);
            }
            scratch_register_slots = scratch_register_slots.max(next_scratch);
        }
        let value_slots = scratch_base.saturating_add(scratch_register_slots);
        Self {
            local_slots,
            register_slots,
            shared_register_slots: shared_slots.len(),
            scratch_register_slots,
            value_slots,
        }
    }

    fn frame_bytes(&self) -> Result<u32, CraneliftLoweringError> {
        let slots = u64::try_from(self.value_slots)
            .unwrap_or(u64::MAX)
            .saturating_add(8);
        let bytes = slots.saturating_mul(8);
        let bytes = u32::try_from(bytes).map_err(|_| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_FRAGMENT_FRAME_SIZE",
                format!("native fragment frame requires {bytes} bytes"),
            )
        })?;
        if bytes > MAX_NATIVE_SPILL_FRAME_BYTES {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_FRAGMENT_FRAME_LIMIT",
                format!(
                    "native fragment frame requires {bytes} bytes; limit is {MAX_NATIVE_SPILL_FRAME_BYTES}"
                ),
            ));
        }
        Ok(bytes.max(16))
    }

    fn local_offset(&self, local: LocalId) -> Result<i32, CraneliftLoweringError> {
        self.local_slots
            .get(&local)
            .copied()
            .and_then(|slot| i32::try_from(slot.saturating_mul(8)).ok())
            .ok_or_else(|| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_FRAGMENT_LOCAL_SLOT",
                    format!("local {} has no compact fragment-frame slot", local.raw()),
                )
            })
    }

    fn register_offset(
        &self,
        fragment: u32,
        register: RegId,
    ) -> Result<i32, CraneliftLoweringError> {
        self.register_slots
            .get(&(fragment, register))
            .copied()
            .and_then(|slot| i32::try_from(slot.saturating_mul(8)).ok())
            .ok_or_else(|| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_FRAGMENT_REGISTER_SLOT",
                    format!(
                        "register {} has no compact slot in fragment {fragment}",
                        register.raw(),
                    ),
                )
            })
    }

    fn register_offset_if_present(&self, fragment: u32, register: RegId) -> Option<i32> {
        self.register_slots
            .get(&(fragment, register))
            .copied()
            .and_then(|slot| i32::try_from(slot.saturating_mul(8)).ok())
    }

    fn control_offset(&self, index: usize) -> i32 {
        i32::try_from(self.value_slots.saturating_add(index).saturating_mul(8)).unwrap_or(i32::MAX)
    }

    fn pending_status_offset(&self) -> i32 {
        self.control_offset(0)
    }
    fn pending_value_offset(&self) -> i32 {
        self.control_offset(1)
    }
    fn entry_id_offset(&self) -> i32 {
        self.control_offset(2)
    }
    fn arguments_offset(&self) -> i32 {
        self.control_offset(3)
    }
    fn result_out_offset(&self) -> i32 {
        self.control_offset(4)
    }
    fn deopt_out_offset(&self) -> i32 {
        self.control_offset(5)
    }
    fn resume_id_offset(&self) -> i32 {
        self.control_offset(6)
    }
    fn resume_state_offset(&self) -> i32 {
        self.control_offset(7)
    }
}

fn region_control_targets(block: &crate::region_ir::RegionBlock) -> BTreeSet<BlockId> {
    let mut targets = native_transition_successors(&block.terminator)
        .into_iter()
        .collect::<BTreeSet<_>>();
    match block.terminator {
        RegionTerminator::Return { finally, .. }
        | RegionTerminator::ReturnReference { finally, .. }
        | RegionTerminator::Exit { finally, .. } => {
            targets.extend(finally);
        }
        RegionTerminator::Jump { .. }
        | RegionTerminator::JumpIfFalse { .. }
        | RegionTerminator::JumpIfTrue { .. }
        | RegionTerminator::JumpIf { .. } => {}
    }
    for instruction in &block.instructions {
        if let RegionInstructionKind::NativeControl(control) = &instruction.kind {
            match control {
                RegionNativeControl::EndFinally {
                    after,
                    outer_finally,
                } => {
                    targets.insert(*after);
                    targets.extend(*outer_finally);
                }
                RegionNativeControl::Throw { .. } => {}
                RegionNativeControl::EnterTry { .. }
                | RegionNativeControl::LeaveTry
                | RegionNativeControl::MakeException { .. } => {}
            }
        }
    }
    targets
}

fn region_block_entry_continuation(block: &crate::region_ir::RegionBlock) -> u32 {
    block.entry_continuation_id
}

impl NativeFunctionFragmentLayout {
    fn for_plan(
        region: &RegionGraph,
        plan: &NativeCompilePlan,
    ) -> Result<Self, CraneliftLoweringError> {
        let mut block_owner = BTreeMap::new();
        for fragment in &plan.fragments {
            for block in &fragment.blocks {
                if block_owner.insert(*block, fragment.id).is_some() {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_DUPLICATE_BLOCK",
                        format!("Region block {} occurs in multiple fragments", block.raw()),
                    ));
                }
            }
        }
        if block_owner.len() != region.blocks.len() {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_FRAGMENT_INCOMPLETE_PLAN",
                format!(
                    "fragment plan owns {} of {} Region blocks",
                    block_owner.len(),
                    region.blocks.len()
                ),
            ));
        }
        let register_liveness = NativeRegisterLiveness::analyze(region);
        let register_live_in = &register_liveness.block_live_in;
        let mut fragments = plan
            .fragments
            .iter()
            .map(|fragment| {
                // Locals carry PHP reference/destructor semantics and every
                // write must remain observable even when classical liveness
                // says the value is dead. Keep the bounded function-local set
                // until the semantic local-access table can distinguish frame
                // cleanup roots from ordinary values. Registers, which drive
                // the pathological regalloc graph, are fragment-local below.
                let mut locals = (0..region.local_count)
                    .map(LocalId::new)
                    .collect::<BTreeSet<_>>();
                let mut registers = BTreeSet::new();
                let mut stored_registers = BTreeSet::new();
                for block_id in &fragment.blocks {
                    let block = &region.blocks[block_id.index()];
                    let mut block_definitions = BTreeSet::new();
                    locals.extend(block.entry_state_locals.iter().copied());
                    locals.extend(block.terminator_state_locals.iter().copied());
                    locals.extend(block.terminator_live_locals.iter().copied());
                    registers.extend(block.terminator.register_uses());
                    registers.extend(
                        register_live_in
                            .get(block_id)
                            .into_iter()
                            .flatten()
                            .copied(),
                    );
                    stored_registers.extend(
                        register_live_in
                            .get(block_id)
                            .into_iter()
                            .flatten()
                            .copied(),
                    );
                    for instruction in &block.instructions {
                        locals.extend(instruction.live_locals.iter().copied());
                        let uses = instruction.register_uses();
                        registers.extend(uses.iter().copied());
                        // Region liveness deliberately models semantic CFG
                        // state, but executable lowering also contains
                        // synthesized/path-dependent operands. Materialize
                        // every use not dominated by a definition in this
                        // real block; same-block definitions remain cached.
                        stored_registers.extend(
                            uses.into_iter()
                                .filter(|register| !block_definitions.contains(register)),
                        );
                        if instruction_has_sparse_snapshot(
                            instruction,
                            region.compile_metadata.tier,
                        ) {
                            registers.extend(
                                register_liveness
                                    .transition_live
                                    .get(&instruction.continuation_id)
                                    .into_iter()
                                    .flatten()
                                    .copied(),
                            );
                            stored_registers.extend(
                                register_liveness
                                    .transition_live
                                    .get(&instruction.continuation_id)
                                    .into_iter()
                                    .flatten()
                                    .copied(),
                            );
                        }
                        block_definitions
                            .extend(region_instruction_defined_registers(&instruction.kind));
                    }
                    stored_registers.extend(
                        block
                            .terminator
                            .register_uses()
                            .into_iter()
                            .filter(|register| !block_definitions.contains(register)),
                    );
                    if block_terminator_has_native_transition(block, region.compile_metadata.tier) {
                        registers.extend(
                            register_liveness
                                .transition_live
                                .get(&block.terminator_continuation_id)
                                .into_iter()
                                .flatten()
                                .copied(),
                        );
                        stored_registers.extend(
                            register_liveness
                                .transition_live
                                .get(&block.terminator_continuation_id)
                                .into_iter()
                                .flatten()
                                .copied(),
                        );
                    }
                }
                // Region lowering can synthesize results that do not exist in
                // the source IR (for example the discarded result of a
                // property unset). Declare the executable definitions even
                // when their first use is outside this fragment.
                for block_id in &fragment.blocks {
                    for instruction in &region.blocks[block_id.index()].instructions {
                        registers.extend(region_instruction_defined_registers(&instruction.kind));
                    }
                }
                NativeFragmentLayout {
                    id: fragment.id,
                    blocks: fragment.blocks.iter().copied().collect(),
                    normal_entries: BTreeSet::new(),
                    external_targets: BTreeSet::new(),
                    locals,
                    registers,
                    stored_registers,
                }
            })
            .collect::<Vec<_>>();
        if let Some(owner) = block_owner.get(&BlockId::new(0)).copied() {
            fragments[owner as usize]
                .normal_entries
                .insert(BlockId::new(0));
        }
        let mut shared_registers = BTreeSet::new();
        for block in &region.blocks {
            let source_owner = block_owner[&block.id];
            for target in region_control_targets(block) {
                let target_owner = block_owner.get(&target).copied().ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_UNKNOWN_TARGET",
                        format!(
                            "Region block {} targets missing block {}",
                            block.id.raw(),
                            target.raw()
                        ),
                    )
                })?;
                if source_owner != target_owner {
                    fragments[source_owner as usize]
                        .external_targets
                        .insert(target);
                    fragments[target_owner as usize]
                        .normal_entries
                        .insert(target);
                    shared_registers
                        .extend(register_live_in.get(&target).into_iter().flatten().copied());
                }
                fragments[source_owner as usize]
                    .stored_registers
                    .extend(register_live_in.get(&target).into_iter().flatten().copied());
            }
        }

        let transition_liveness = &register_liveness.transition_live;
        let mut resume_owner = BTreeMap::new();
        let mut insert_resume = |resume_id: i32, block: BlockId| {
            let owner = block_owner[&block];
            match resume_owner.insert(resume_id, owner) {
                Some(previous) if previous != owner => Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_FRAGMENT_RESUME_COLLISION",
                    format!("resume id {resume_id} belongs to fragments {previous} and {owner}"),
                )),
                _ => Ok(()),
            }
        };
        for handler in &region.exception_regions {
            for target in [handler.catch, handler.finally].into_iter().flatten() {
                insert_resume(crate::native_handler_resume_id(target), target)?;
            }
        }
        for block in &region.blocks {
            if region.compile_metadata.tier == NativeCompilerTier::Optimizing {
                insert_resume(
                    crate::native_optimizing_continuation_resume_id(
                        region_block_entry_continuation(block),
                    ),
                    block.id,
                )?;
            }
            if block_terminator_has_native_transition(block, region.compile_metadata.tier)
                && transition_liveness
                    .get(&block.terminator_continuation_id)
                    .is_some_and(|live| live.len() <= crate::JIT_DEOPT_MAX_REGISTERS)
            {
                insert_resume(
                    crate::native_transition_resume_id(block.terminator_continuation_id),
                    block.id,
                )?;
            }
            for instruction in &block.instructions {
                if matches!(instruction.kind, RegionInstructionKind::NativeSuspend(_)) {
                    insert_resume(
                        crate::native_suspension_resume_id(instruction.continuation_id),
                        block.id,
                    )?;
                }
                if instruction_has_native_resume_entry(instruction, region.compile_metadata.tier)
                    && transition_liveness
                        .get(&instruction.continuation_id)
                        .is_some_and(|live| live.len() <= crate::JIT_DEOPT_MAX_REGISTERS)
                {
                    insert_resume(
                        crate::native_transition_resume_id(instruction.continuation_id),
                        block.id,
                    )?;
                }
            }
        }
        for osr in region.osr_entries() {
            insert_resume(
                i32::try_from(osr.id).map_err(|_| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_OSR_ID",
                        format!("OSR id {} does not fit the native resume ABI", osr.id),
                    )
                })?,
                osr.block,
            )?;
        }
        let frame = NativeFragmentFrameLayout::for_fragments(region, &fragments, &shared_registers);
        Ok(Self {
            fragments,
            block_owner,
            resume_owner,
            frame,
            register_liveness,
        })
    }
}

fn region_contains(
    region: &RegionGraph,
    predicate: impl Fn(&RegionInstructionKind) -> bool,
) -> bool {
    region
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| predicate(&instruction.kind))
}

fn optimizing_compiled_call_params<'a>(
    call: &RegionNativeCall,
    unit: &'a IrUnit,
    external_function_signatures: &'a [crate::JitExternalFunctionSignature],
) -> Option<&'a [php_ir::IrParam]> {
    if let Some(function) = call
        .direct_compiled_target()
        .or_else(|| call.direct_compiled_unpack_target())
    {
        return unit
            .functions
            .get(function.index())
            .map(|function| function.params.as_slice());
    }
    let (name, link_index) = match &call.target {
        RegionCallTarget::Function {
            name,
            function: None,
        } => (Some(name.as_str()), None),
        RegionCallTarget::Method {
            function: None,
            linked_function: Some(link_index),
            receiver_layout_id: Some(_),
            ..
        } => (None, Some(*link_index)),
        _ => return None,
    };
    external_function_signatures
        .iter()
        .find(|signature| {
            signature.published
                && (name.is_some_and(|name| {
                    signature
                        .name
                        .trim_start_matches('\\')
                        .eq_ignore_ascii_case(name.trim_start_matches('\\'))
                }) || link_index == Some(signature.link_index))
        })
        .map(|signature| signature.native_params.as_slice())
}

#[derive(Clone, Copy, Default)]
struct OptimizingCallScalarHelperNeeds {
    numeric_string: bool,
    string_cast: bool,
}

fn direct_fixed_builtin_operand(call: &RegionNativeCall, index: usize) -> Option<RegionOperand> {
    call.args
        .get(index)
        .filter(|argument| argument.name.is_none() && !argument.unpack)
        .and_then(|_| {
            call.operands
                .get(call.argument_operand_offset.saturating_add(index))
                .copied()
                .flatten()
        })
}

fn optimizing_strval_uses_float_handler(
    call: &RegionNativeCall,
    value_flow: &ExecutableValueFlow,
    constants: &[IrConstant],
) -> bool {
    stable_builtin_scalar_consumer(&call.target) == Some(StableScalarConsumerBuiltin::StrVal)
        && call.args.len() == 1
        && direct_fixed_builtin_operand(call, 0).is_some_and(|operand| {
            let fact = lowering_operand_fact(value_flow, constants, operand);
            fact.certainty != crate::region_ir::SsaCertainty::Unknown
                && fact.class == SsaValueClass::Float
        })
}

fn optimizing_call_scalar_helper_needs(
    call: &RegionNativeCall,
    unit: &IrUnit,
    external_function_signatures: &[crate::JitExternalFunctionSignature],
    value_flow: &ExecutableValueFlow,
    constants: &[IrConstant],
) -> OptimizingCallScalarHelperNeeds {
    use php_ir::IrReturnType as Type;

    let Some(parameters) =
        optimizing_compiled_call_params(call, unit, external_function_signatures)
    else {
        return OptimizingCallScalarHelperNeeds::default();
    };
    let mut needs = OptimizingCallScalarHelperNeeds::default();
    let mut consider = |parameter: &php_ir::IrParam, operand: Option<RegionOperand>| {
        let Some(type_) = parameter.type_.as_ref() else {
            return;
        };
        if !parameter.by_ref
            && operand.is_some_and(|operand| {
                optimizing_fact_satisfies_type(
                    lowering_operand_fact(value_flow, constants, operand),
                    type_,
                )
            })
        {
            return;
        }
        let scalar_type = match type_ {
            Type::Nullable { inner } => inner.as_ref(),
            type_ => type_,
        };
        match (call.caller_strict_types, scalar_type) {
            (false, Type::Int | Type::Float) => {
                needs.numeric_string = true;
            }
            (false, Type::String) => needs.string_cast = true,
            _ => {}
        }
    };

    if let Some(unpack) = call.trailing_unpack_argument() {
        for (index, parameter) in parameters.iter().enumerate() {
            let operand = (index < unpack)
                .then(|| {
                    call.operands
                        .get(call.argument_operand_offset.saturating_add(index))
                        .copied()
                        .flatten()
                })
                .flatten();
            consider(parameter, operand);
        }
    } else {
        for (index, operand) in call
            .operands
            .iter()
            .skip(call.argument_operand_offset)
            .enumerate()
        {
            let Some(parameter) = parameters
                .get(index)
                .or_else(|| parameters.last().filter(|parameter| parameter.variadic))
            else {
                continue;
            };
            consider(parameter, *operand);
        }
    }
    needs
}

fn optimizing_return_scalar_helper_needs(
    region: &RegionGraph,
    value_flow: &ExecutableValueFlow,
    constants: &[IrConstant],
    strict: bool,
) -> OptimizingCallScalarHelperNeeds {
    use php_ir::IrReturnType as Type;

    let Some(return_type) = region.return_type.as_ref() else {
        return OptimizingCallScalarHelperNeeds::default();
    };
    let scalar_type = match return_type {
        Type::Nullable { inner } => inner.as_ref(),
        return_type => return_type,
    };
    let mut needs = OptimizingCallScalarHelperNeeds::default();
    for block in &region.blocks {
        let requires_scalar_boundary = match &block.terminator {
            RegionTerminator::Return {
                value,
                finally: None,
            } => !optimizing_fact_satisfies_type(
                lowering_operand_fact(value_flow, constants, *value),
                return_type,
            ),
            // The local's SSA fact describes the reference container. Its
            // authoritative payload still requires the declared return
            // coercion and writeback boundary.
            RegionTerminator::ReturnReference { finally: None, .. } => true,
            _ => false,
        };
        if !requires_scalar_boundary {
            continue;
        }
        match (strict, scalar_type) {
            (false, Type::Int | Type::Float) => needs.numeric_string = true,
            (false, Type::String) => needs.string_cast = true,
            _ => {}
        }
    }
    needs
}

fn native_transition_successors(terminator: &RegionTerminator) -> Vec<BlockId> {
    match terminator {
        RegionTerminator::Jump { target } => vec![*target],
        RegionTerminator::JumpIfFalse {
            target,
            fallthrough,
            ..
        }
        | RegionTerminator::JumpIfTrue {
            target,
            fallthrough,
            ..
        } => vec![*target, *fallthrough],
        RegionTerminator::JumpIf {
            if_true, if_false, ..
        } => vec![*if_true, *if_false],
        RegionTerminator::Return { .. }
        | RegionTerminator::ReturnReference { .. }
        | RegionTerminator::Exit { .. } => Vec::new(),
    }
}

pub(super) fn instruction_has_native_transition(
    instruction: &RegionInstruction,
    tier: NativeCompilerTier,
) -> bool {
    if tier == NativeCompilerTier::Optimizing {
        return instruction.optimizer_transition_entry;
    }
    // Baseline must publish the exact entry used by an optimizing island
    // exit, including the first instruction of a baseline-only family. The
    // old hand-maintained allow-list covered direct guards but omitted such
    // island heads (for example a static-local operation), so valid optimized
    // code produced a state the corresponding baseline artifact could not
    // enter.
    if instruction.optimizer_transition_entry {
        return true;
    }
    // Checked binary operations can request a baseline retry. A userland call
    // also needs a caller continuation when its callee suspends (for example a
    // Fiber::suspend nested below the call); throw and exit still unwind
    // terminally through the handler table. These are real resumable
    // safepoints, not instruction-per-resume entries.
    matches!(
        instruction.kind,
        RegionInstructionKind::Binary { .. }
            | RegionInstructionKind::Unary { .. }
            | RegionInstructionKind::LoadLocal { .. }
            | RegionInstructionKind::StoreLocal { .. }
            | RegionInstructionKind::AssignLocalResult { .. }
            | RegionInstructionKind::Discard { .. }
            | RegionInstructionKind::IssetLocal { .. }
            | RegionInstructionKind::EmptyLocal { .. }
            | RegionInstructionKind::UnsetLocal { .. }
            | RegionInstructionKind::NewArray { .. }
            | RegionInstructionKind::ArrayInsert { .. }
            | RegionInstructionKind::AppendDim { .. }
            | RegionInstructionKind::IssetDim { .. }
            | RegionInstructionKind::EmptyDim { .. }
            | RegionInstructionKind::FetchDim {
                mode: php_ir::instruction::DimFetchMode::Read,
                ..
            }
            | RegionInstructionKind::ForeachInit { .. }
            | RegionInstructionKind::ForeachNext { .. }
            | RegionInstructionKind::ForeachCleanup { .. }
            | RegionInstructionKind::FetchProperty { .. }
            | RegionInstructionKind::ArrayCallback(_)
            | RegionInstructionKind::PregCallbackArray(_)
            | RegionInstructionKind::NativeCall(_)
            | RegionInstructionKind::NativeDynamicCode(_)
    )
}

fn optimizing_instruction_family_is_direct(instruction: &RegionInstruction) -> bool {
    match &instruction.kind {
        RegionInstructionKind::AssignDim { keys, .. }
        | RegionInstructionKind::UnsetDim { keys, .. } => !keys.is_empty(),
        RegionInstructionKind::AppendDim { .. } => true,
        RegionInstructionKind::ArrayInsert {
            key, by_ref_local, ..
        } => key.is_none() && by_ref_local.is_none(),
        RegionInstructionKind::BindReferenceIntoDim { append, keys, .. } => {
            instruction.native_global_name.is_none() && (*append || !keys.is_empty())
        }
        RegionInstructionKind::BindReferenceDim { keys, .. }
        | RegionInstructionKind::BindReferenceFromPropertyDim { keys, .. } => !keys.is_empty(),
        RegionInstructionKind::BindReferenceIntoPropertyDim { append, keys, .. } => {
            *append || !keys.is_empty()
        }
        RegionInstructionKind::NewObject { prepared, .. } => *prepared,
        RegionInstructionKind::CloneObject { plain, .. } => *plain,
        RegionInstructionKind::FetchDim { mode, .. } => {
            *mode == php_ir::instruction::DimFetchMode::Read
        }
        RegionInstructionKind::Nop
        | RegionInstructionKind::Move { .. }
        | RegionInstructionKind::LoadLocal { .. }
        | RegionInstructionKind::StoreLocal { .. }
        | RegionInstructionKind::AssignLocalResult { .. }
        | RegionInstructionKind::BindReference { .. }
        | RegionInstructionKind::BindReferenceProperty { .. }
        | RegionInstructionKind::BindReferenceFromProperty { .. }
        | RegionInstructionKind::BindReferenceDimFromProperty { .. }
        | RegionInstructionKind::InitStaticLocal { .. }
        | RegionInstructionKind::Discard { .. }
        | RegionInstructionKind::Binary { .. }
        | RegionInstructionKind::Unary { .. }
        | RegionInstructionKind::Compare { .. }
        | RegionInstructionKind::Cast { .. }
        | RegionInstructionKind::Echo { .. }
        | RegionInstructionKind::NewArray { .. }
        | RegionInstructionKind::FetchProperty { .. }
        | RegionInstructionKind::FetchObjectClassName { .. }
        | RegionInstructionKind::AssignProperty { .. }
        | RegionInstructionKind::ArraySpread { .. }
        | RegionInstructionKind::FetchConst { .. }
        | RegionInstructionKind::IssetDim { .. }
        | RegionInstructionKind::EmptyDim { .. }
        | RegionInstructionKind::IssetLocal { .. }
        | RegionInstructionKind::EmptyLocal { .. }
        | RegionInstructionKind::UnsetLocal { .. }
        | RegionInstructionKind::ForeachInit { .. }
        | RegionInstructionKind::ForeachInitRef { .. }
        | RegionInstructionKind::ForeachNext { .. }
        | RegionInstructionKind::ForeachNextRef { .. }
        | RegionInstructionKind::ForeachCleanup { .. }
        | RegionInstructionKind::ArrayCallback(_)
        | RegionInstructionKind::PregCallbackArray(_)
        | RegionInstructionKind::NativeDynamicCode(RegionNativeDynamicCode::MakeClosure {
            ..
        })
        | RegionInstructionKind::NativeSuspend(_)
        | RegionInstructionKind::NativeCall(_) => true,
        RegionInstructionKind::FetchDynamicStaticProperty { .. }
        | RegionInstructionKind::CloneWith { .. }
        | RegionInstructionKind::NativeControl(_)
        | RegionInstructionKind::NativeDynamicCode(_)
        | RegionInstructionKind::RuntimeFatal { .. }
        | RegionInstructionKind::CompileTimeFatal { .. } => false,
    }
}

fn optimizing_direct_instruction_may_transition(kind: &RegionInstructionKind) -> bool {
    !matches!(
        kind,
        RegionInstructionKind::Nop | RegionInstructionKind::Move { .. }
    )
}

fn prepare_optimizing_baseline_islands(mut region: RegionGraph) -> RegionGraph {
    let boundaries = region
        .blocks
        .iter()
        .filter_map(|block| {
            let mut previous = None;
            let boundaries = block
                .instructions
                .iter()
                .enumerate()
                .filter_map(|(index, instruction)| {
                    let direct = optimizing_instruction_family_is_direct(instruction);
                    let changed = previous.is_some_and(|previous| previous != direct);
                    previous = Some(direct);
                    (index != 0 && changed).then_some(index)
                })
                .collect::<BTreeSet<_>>();
            (!boundaries.is_empty()).then_some((block.id, boundaries))
        })
        .collect::<BTreeMap<_, _>>();
    region = super::module_layout::split_region_blocks_at_boundaries(region, &boundaries);
    for block in &mut region.blocks {
        let direct = block
            .instructions
            .first()
            .is_none_or(optimizing_instruction_family_is_direct);
        for (index, instruction) in block.instructions.iter_mut().enumerate() {
            instruction.optimizer_transition_entry = if direct {
                optimizing_direct_instruction_may_transition(&instruction.kind)
            } else {
                index == 0
            };
        }
    }
    region
}

fn instruction_has_sparse_snapshot(
    instruction: &RegionInstruction,
    tier: NativeCompilerTier,
) -> bool {
    instruction_has_native_transition(instruction, tier)
        || matches!(instruction.kind, RegionInstructionKind::NativeSuspend(_))
}

/// Whether this artifact can be entered again at the instruction after it
/// has already started executing. Guard failures in optimizing code exit to
/// the baseline artifact; they are not optimizer resume entries. Conflating
/// those two directions forced the normal optimizing path through a distinct
/// CLIF block for every guardable PHP instruction.
fn instruction_has_native_resume_entry(
    instruction: &RegionInstruction,
    tier: NativeCompilerTier,
) -> bool {
    match tier {
        NativeCompilerTier::Baseline => instruction_has_native_transition(instruction, tier),
        NativeCompilerTier::Optimizing => {
            matches!(
                instruction.kind,
                RegionInstructionKind::ArrayCallback(_)
                    | RegionInstructionKind::PregCallbackArray(_)
                    | RegionInstructionKind::NativeCall(_)
            )
        }
    }
}

fn terminator_has_native_transition(terminator: &RegionTerminator) -> bool {
    !matches!(terminator, RegionTerminator::Jump { .. })
}

fn block_terminator_has_native_transition(
    block: &crate::region_ir::RegionBlock,
    _tier: NativeCompilerTier,
) -> bool {
    terminator_has_native_transition(&block.terminator)
        && !block.instructions.iter().any(|instruction| {
            matches!(instruction.kind, RegionInstructionKind::RuntimeFatal { .. })
        })
}

/// Restore the sparse local portion of a native continuation into the compact
/// streaming frame.  The initialization masks are already part of the
/// transition ABI, so one cold loop can serve every handler, suspension, OSR,
/// and tier-transition entry in the fragment.  Emitting the same local-copy
/// sequence into every resume loader made cold state reconstruction dominate
/// the machine code of large baseline functions.
fn emit_streaming_local_restore_loop(
    builder: &mut FunctionBuilder<'_>,
    pointer_type: ir::Type,
    state: ir::Value,
    frame: ir::Value,
    local_count: u32,
    continuation: ir::Block,
) {
    if local_count == 0 {
        builder.ins().jump(continuation, &[]);
        return;
    }

    let header = builder.create_block();
    let test = builder.create_block();
    let copy = builder.create_block();
    let next = builder.create_block();
    for block in [header, test, copy, next] {
        builder.set_cold_block(block);
    }
    builder.append_block_param(header, types::I64);
    builder.append_block_param(test, types::I64);
    builder.append_block_param(copy, types::I64);
    builder.append_block_param(next, types::I64);

    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().jump(header, &[zero.into()]);

    builder.switch_to_block(header);
    let index = builder.block_params(header)[0];
    let in_range = builder
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, index, i64::from(local_count));
    builder
        .ins()
        .brif(in_range, test, &[index.into()], continuation, &[]);

    builder.switch_to_block(test);
    let index = builder.block_params(test)[0];
    let word = builder.ins().ushr_imm(index, 6);
    let word_bytes = builder.ins().ishl_imm(word, 3);
    let word_bytes = if pointer_type == types::I64 {
        word_bytes
    } else {
        builder.ins().ireduce(pointer_type, word_bytes)
    };
    let mask_base = builder.ins().iadd_imm(
        state,
        std::mem::offset_of!(crate::JitDeoptState, initialized_mask) as i64,
    );
    let mask_address = builder.ins().iadd(mask_base, word_bytes);
    let mask = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), mask_address, 0);
    let bit_index = builder.ins().band_imm(index, 63);
    let one = builder.ins().iconst(types::I64, 1);
    let bit = builder.ins().ishl(one, bit_index);
    let initialized = builder.ins().band(mask, bit);
    let initialized = builder.ins().icmp_imm(IntCC::NotEqual, initialized, 0);
    builder
        .ins()
        .brif(initialized, copy, &[index.into()], next, &[index.into()]);

    builder.switch_to_block(copy);
    let index = builder.block_params(copy)[0];
    let slot_bytes = builder.ins().ishl_imm(index, 3);
    let slot_bytes = if pointer_type == types::I64 {
        slot_bytes
    } else {
        builder.ins().ireduce(pointer_type, slot_bytes)
    };
    let state_slots = builder.ins().iadd_imm(
        state,
        std::mem::offset_of!(crate::JitDeoptState, slots) as i64,
    );
    let state_slot = builder.ins().iadd(state_slots, slot_bytes);
    let frame_slot = builder.ins().iadd(frame, slot_bytes);
    let value = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), state_slot, 0);
    builder
        .ins()
        .store(MemFlagsData::new(), value, frame_slot, 0);
    builder.ins().jump(next, &[index.into()]);

    builder.switch_to_block(next);
    let index = builder.block_params(next)[0];
    let index = builder.ins().iadd_imm(index, 1);
    builder.ins().jump(header, &[index.into()]);
}

/// Publish sparse baseline locals from the compact fragment frame. Every
/// callsite supplies only its static live mask; one cold loop performs the
/// actual copy for all call side exits in the fragment.
fn emit_streaming_local_snapshot_loop(
    builder: &mut FunctionBuilder<'_>,
    pointer_type: ir::Type,
    state: ir::Value,
    frame: ir::Value,
    local_count: u32,
    continuation: ir::Block,
) {
    if local_count == 0 {
        builder.ins().jump(continuation, &[]);
        return;
    }

    let header = builder.create_block();
    let test = builder.create_block();
    let copy = builder.create_block();
    let next = builder.create_block();
    for block in [header, test, copy, next] {
        builder.set_cold_block(block);
    }
    builder.append_block_param(header, types::I64);
    builder.append_block_param(test, types::I64);
    builder.append_block_param(copy, types::I64);
    builder.append_block_param(next, types::I64);

    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().jump(header, &[zero.into()]);

    builder.switch_to_block(header);
    let index = builder.block_params(header)[0];
    let in_range = builder
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, index, i64::from(local_count));
    builder
        .ins()
        .brif(in_range, test, &[index.into()], continuation, &[]);

    builder.switch_to_block(test);
    let index = builder.block_params(test)[0];
    let word = builder.ins().ushr_imm(index, 6);
    let word_bytes = builder.ins().ishl_imm(word, 3);
    let word_bytes = if pointer_type == types::I64 {
        word_bytes
    } else {
        builder.ins().ireduce(pointer_type, word_bytes)
    };
    let mask_base = builder.ins().iadd_imm(
        state,
        std::mem::offset_of!(crate::JitDeoptState, initialized_mask) as i64,
    );
    let mask_address = builder.ins().iadd(mask_base, word_bytes);
    let mask = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), mask_address, 0);
    let bit_index = builder.ins().band_imm(index, 63);
    let one = builder.ins().iconst(types::I64, 1);
    let bit = builder.ins().ishl(one, bit_index);
    let initialized = builder.ins().band(mask, bit);
    let initialized = builder.ins().icmp_imm(IntCC::NotEqual, initialized, 0);
    builder
        .ins()
        .brif(initialized, copy, &[index.into()], next, &[index.into()]);

    builder.switch_to_block(copy);
    let index = builder.block_params(copy)[0];
    let slot_bytes = builder.ins().ishl_imm(index, 3);
    let slot_bytes = if pointer_type == types::I64 {
        slot_bytes
    } else {
        builder.ins().ireduce(pointer_type, slot_bytes)
    };
    let state_slots = builder.ins().iadd_imm(
        state,
        std::mem::offset_of!(crate::JitDeoptState, slots) as i64,
    );
    let state_slot = builder.ins().iadd(state_slots, slot_bytes);
    let frame_slot = builder.ins().iadd(frame, slot_bytes);
    let value = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), frame_slot, 0);
    builder
        .ins()
        .store(MemFlagsData::new(), value, state_slot, 0);
    builder.ins().jump(next, &[index.into()]);

    builder.switch_to_block(next);
    let index = builder.block_params(next)[0];
    let index = builder.ins().iadd_imm(index, 1);
    builder.ins().jump(header, &[index.into()]);
}

/// Classical SSA live-in sets for the small set of actual native transition
/// safepoints. This deliberately does not equate "defined earlier" with
/// "live now": doing so creates cumulative register prefixes and quadratic
/// Cranelift move/alias pressure in large PHP functions.
fn native_register_live_in(region: &RegionGraph) -> BTreeMap<BlockId, BTreeSet<RegId>> {
    let block_indices = region
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id, index))
        .collect::<BTreeMap<_, _>>();
    let mut live_in = vec![BTreeSet::<RegId>::new(); region.blocks.len()];
    loop {
        let mut changed = false;
        for (index, block) in region.blocks.iter().enumerate().rev() {
            let mut live = native_transition_successors(&block.terminator)
                .into_iter()
                .filter_map(|successor| block_indices.get(&successor).copied())
                .flat_map(|successor| live_in[successor].iter().copied())
                .collect::<BTreeSet<_>>();
            live.extend(block.terminator.register_uses());
            live.extend(block.terminator_live_registers.iter().flatten().copied());
            for instruction in block.instructions.iter().rev() {
                for defined in region_instruction_defined_registers(&instruction.kind) {
                    live.remove(&defined);
                }
                live.extend(instruction.register_uses());
                live.extend(
                    instruction
                        .transition_live_registers
                        .iter()
                        .flatten()
                        .copied(),
                );
            }
            if live != live_in[index] {
                live_in[index] = live;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    region
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id, live_in[index].clone()))
        .collect()
}

#[derive(Clone, Debug)]
struct NativeRegisterLiveness {
    block_live_in: BTreeMap<BlockId, BTreeSet<RegId>>,
    transition_live: BTreeMap<u32, Vec<RegId>>,
}

impl NativeRegisterLiveness {
    fn analyze(region: &RegionGraph) -> Self {
        let block_live_in = native_register_live_in(region);
        let mut transition_live = BTreeMap::new();
        for block in &region.blocks {
            let mut live = native_transition_successors(&block.terminator)
                .into_iter()
                .filter_map(|successor| block_live_in.get(&successor))
                .flat_map(|registers| registers.iter().copied())
                .collect::<BTreeSet<_>>();
            live.extend(block.terminator.register_uses());
            if block_terminator_has_native_transition(block, region.compile_metadata.tier) {
                transition_live.insert(
                    block.terminator_continuation_id,
                    block
                        .terminator_live_registers
                        .clone()
                        .unwrap_or_else(|| live.iter().copied().collect()),
                );
            }
            for instruction in block.instructions.iter().rev() {
                for defined in region_instruction_defined_registers(&instruction.kind) {
                    live.remove(&defined);
                }
                live.extend(instruction.register_uses());
                live.extend(
                    instruction
                        .transition_live_registers
                        .iter()
                        .flatten()
                        .copied(),
                );
                if instruction_has_sparse_snapshot(instruction, region.compile_metadata.tier) {
                    transition_live.insert(
                        instruction.continuation_id,
                        instruction
                            .transition_live_registers
                            .clone()
                            .unwrap_or_else(|| live.iter().copied().collect()),
                    );
                }
            }
        }
        Self {
            block_live_in,
            transition_live,
        }
    }
}

fn native_register_state_points(region: &RegionGraph) -> BTreeMap<u32, Vec<RegId>> {
    let block_live_in = native_register_live_in(region);
    let mut state_points = BTreeMap::new();
    for block in &region.blocks {
        let mut live = native_transition_successors(&block.terminator)
            .into_iter()
            .filter_map(|successor| block_live_in.get(&successor))
            .flat_map(|registers| registers.iter().copied())
            .collect::<BTreeSet<_>>();
        live.extend(block.terminator.register_uses());
        state_points.insert(
            block.terminator_continuation_id,
            live.iter().copied().collect(),
        );
        for instruction in block.instructions.iter().rev() {
            for defined in region_instruction_defined_registers(&instruction.kind) {
                live.remove(&defined);
            }
            live.extend(instruction.register_uses());
            state_points.insert(instruction.continuation_id, live.iter().copied().collect());
        }
    }
    state_points
}

fn pin_native_transition_registers(
    region: &mut RegionGraph,
    source_state_points: &BTreeMap<u32, Vec<RegId>>,
) {
    for block in &mut region.blocks {
        block.terminator_live_registers = None;
        for instruction in &mut block.instructions {
            instruction.transition_live_registers = None;
        }
    }
    for block in &mut region.blocks {
        if block_terminator_has_native_transition(block, region.compile_metadata.tier) {
            block.terminator_live_registers = Some(
                source_state_points
                    .get(&block.terminator_continuation_id)
                    .cloned()
                    .expect("native terminator continuation belongs to the source region"),
            );
        }
        for instruction in &mut block.instructions {
            if instruction_has_sparse_snapshot(instruction, region.compile_metadata.tier) {
                instruction.transition_live_registers = Some(
                    source_state_points
                        .get(&instruction.continuation_id)
                        .cloned()
                        .expect("native instruction continuation belongs to the source region"),
                );
            }
        }
    }
}

fn ir_function_requires_trampoline(function: &php_ir::IrFunction) -> bool {
    function.params.iter().any(|parameter| parameter.by_ref)
        || function.returns_by_ref
        || ir_function_requires_non_reference_trampoline(function)
}

fn ir_function_requires_non_reference_trampoline(function: &php_ir::IrFunction) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                php_ir::InstructionKind::Yield { .. } | php_ir::InstructionKind::YieldFrom { .. }
            ) || matches!(
                &instruction.kind,
                php_ir::InstructionKind::CallFunction { name, .. }
                    if name.trim_start_matches('\\').eq_ignore_ascii_case("debug_backtrace")
            )
        })
    }) || function.attributes.iter().any(|attribute| {
        attribute
            .resolved_name
            .as_deref()
            .or(attribute.fallback_name.as_deref())
            .unwrap_or(&attribute.name)
            .trim_start_matches('\\')
            .eq_ignore_ascii_case("deprecated")
    })
}

fn ir_function_has_exception_handler(function: &php_ir::IrFunction) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                php_ir::InstructionKind::EnterTry { catch: Some(_), .. }
                    | php_ir::InstructionKind::EnterTry {
                        finally: Some(_),
                        ..
                    }
            )
        })
    })
}

fn declare_baseline_value_operation(
    module: &mut JITModule,
    symbol: &str,
    arity: u8,
    address: usize,
) -> Result<NativeHelper, CraneliftLoweringError> {
    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I32));
    for _ in 0..arity {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    declare_native_helper(module, symbol, &signature, address)
}

fn declare_native_helper(
    module: &mut JITModule,
    symbol: &str,
    signature: &ir::Signature,
    address: usize,
) -> Result<NativeHelper, CraneliftLoweringError> {
    let pointer_type = module.target_config().pointer_type();
    let mut signature = signature.clone();
    signature.params.insert(0, AbiParam::new(pointer_type));
    let import_symbol = native_helper_import_symbol(symbol, address);
    let function = module
        .declare_function(&import_symbol, Linkage::Import, &signature)
        .map_err(|error| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_NATIVE_OPERATION",
                format!("failed to declare {symbol}: {error}"),
            )
        })?;
    Ok(NativeHelper {
        function,
        terminal_exit: None,
        inline_runtime_view: false,
        runtime: None,
    })
}

fn declare_native_pure_handler(
    module: &mut JITModule,
    symbol: &str,
    signature: &ir::Signature,
    address: usize,
) -> Result<NativeHelper, CraneliftLoweringError> {
    let import_symbol = native_helper_import_symbol(symbol, address);
    let function = module
        .declare_function(&import_symbol, Linkage::Import, signature)
        .map_err(|error| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_NATIVE_OPERATION",
                format!("failed to declare pure {symbol}: {error}"),
            )
        })?;
    Ok(NativeHelper {
        function,
        terminal_exit: None,
        inline_runtime_view: false,
        runtime: None,
    })
}

fn declare_native_control_handler(
    module: &mut JITModule,
    needed: bool,
    symbol: &str,
    argument_count: usize,
    address: impl FnOnce() -> usize,
) -> Result<Option<NativeHelper>, CraneliftLoweringError> {
    if !needed {
        return Ok(None);
    }
    let mut signature = module.make_signature();
    for _ in 0..argument_count {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I64));
    declare_native_helper(module, symbol, &signature, address()).map(Some)
}

pub(super) fn compile_region_graph_native(
    unit: &IrUnit,
    region: RegionGraph,
    plan: NativeCompilePlan,
    runtime_helpers: crate::JitRuntimeHelperAddresses,
    request: &JitCompileRequest,
) -> Result<NativeScalarRegionCompileResult, CraneliftLoweringError> {
    validate_region_native_coverage(&region)?;
    region.verify().map_err(|error| {
        CraneliftLoweringError::new("JIT_CRANELIFT_REJECT_REGION_VERIFY", error.to_string())
    })?;
    let function = region.function;
    let runtime_unit_identity = if request.deployment_runtime_identity == 0 {
        u64::from(unit.id.raw())
    } else {
        request.deployment_runtime_identity
    };
    let mut regions = BTreeMap::from([(function, region)]);
    for candidate in regions.values_mut() {
        select_native_region_tier(candidate, &plan, &unit.constants);
    }
    // Admission can deliberately downgrade an optimizing request when even
    // one instruction family still belongs to the baseline-native runtime.
    // The incoming plan was built for the requested tier and may therefore
    // contain one large whole-region job. Re-plan the resulting graph before
    // any CLIF construction so the downgrade cannot bypass baseline fragment
    // ceilings or fail a valid PHP unit merely because its stale optimizing
    // plan was oversized.
    let replanned = split_oversized_region_blocks(
        regions
            .remove(&function)
            .expect("compile group owns its requested function"),
    );
    regions.insert(function, replanned);
    let plan = NativeCompilePlan::for_region(&regions[&function]);
    if regions[&function].compile_metadata.tier == NativeCompilerTier::Baseline
        && let Some(fragment) = plan
            .fragments
            .iter()
            .find(|fragment| !fragment.is_within_budget())
    {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_FRAGMENT_BUDGET",
            format!(
                "fragment {} exceeds the pre-Cranelift budget: blocks={} instructions={} estimated_clif_blocks={}",
                fragment.id,
                fragment.blocks.len(),
                fragment.ir_instructions,
                fragment.estimated_clif_blocks
            ),
        ));
    }
    let region = &regions[&function];
    let compilation_mode = crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
        region.compile_metadata.tier,
    )
    .mode();
    let baseline_helper_imports = compilation_mode
        == crate::cranelift_lowering::baseline_streaming::NativeCompilationMode::StreamingBaseline;
    let fragment_layout = (plan.fragments.len() > 1
        || regions
            .values()
            .any(|candidate| candidate.compile_metadata.tier == NativeCompilerTier::Baseline))
    .then(|| NativeFunctionFragmentLayout::for_plan(region, &plan))
    .transpose()?;
    let selected_plan = std::cell::RefCell::new(plan.clone());
    let selected_fragment_layout = std::cell::RefCell::new(fragment_layout.clone());
    // Value-flow and executable SSA describe the PHP function, not one native
    // fragment. Build them exactly once after tier selection and Region-block
    // splitting. Recomputing the complete dominator/phi graph inside every
    // fragment lowering made fragmentation multiply whole-function analysis.
    let value_flows = regions
        .iter()
        .map(|(function, candidate)| {
            let flow = if candidate.compile_metadata.tier == NativeCompilerTier::Optimizing {
                crate::region_ir::analyze_executable_value_flow(candidate, &unit.constants)
            } else {
                crate::region_ir::analyze_baseline_value_ownership(candidate)
            };
            flow.verify_ownership(candidate).map_err(|error| {
                CraneliftLoweringError::new("JIT_CRANELIFT_REJECT_OWNERSHIP", error)
            })?;
            Ok((*function, flow))
        })
        .collect::<Result<BTreeMap<_, _>, CraneliftLoweringError>>()?;
    let ssa_metrics = regions
        .iter()
        .filter(|(_, candidate)| candidate.compile_metadata.tier == NativeCompilerTier::Optimizing)
        .map(|(function, _)| {
            let flow = &value_flows[function];
            (
                flow.promoted_local_count() as u64,
                flow.promoted_register_count() as u64,
                flow.ownership_move_count() as u64,
            )
        })
        .fold((0_u64, 0_u64, 0_u64), |total, metrics| {
            (
                total.0.saturating_add(metrics.0),
                total.1.saturating_add(metrics.1),
                total.2.saturating_add(metrics.2),
            )
        });
    let arity = region_arity(region)?;
    let fast_path_hits = regions
        .values()
        .map(|region| region.fast_path_operations)
        .sum();
    let has_control_flow = regions.values().any(RegionGraph::has_control_flow);
    let mut trampoline_functions = regions
        .iter()
        .filter_map(|(function, region)| {
            (region.params.iter().any(|parameter| parameter.by_ref)
                || region.returns_by_ref
                || region_contains(region, |kind| {
                    matches!(
                        kind,
                        RegionInstructionKind::NativeControl(RegionNativeControl::Throw { .. })
                            | RegionInstructionKind::NativeDynamicCode(
                                RegionNativeDynamicCode::MakeClosure { .. }
                            )
                    )
                })
                || region.attributes.iter().any(|attribute| {
                    attribute
                        .resolved_name
                        .as_deref()
                        .or(attribute.fallback_name.as_deref())
                        .unwrap_or(&attribute.name)
                        .trim_start_matches('\\')
                        .eq_ignore_ascii_case("deprecated")
                }))
            .then_some(*function)
        })
        .collect::<BTreeSet<_>>();
    loop {
        let callers = regions
            .iter()
            .filter_map(|(function, region)| {
                region
                    .direct_callees()
                    .iter()
                    .any(|callee| trampoline_functions.contains(callee))
                    .then_some(*function)
            })
            .collect::<Vec<_>>();
        let previous = trampoline_functions.len();
        trampoline_functions.extend(callers);
        if trampoline_functions.len() == previous {
            break;
        }
    }
    let resolver_target = |target: FunctionId| {
        runtime_helpers.native_function_resolve != 0
            && !regions.contains_key(&target)
            && unit
                .functions
                .get(target.index())
                .is_some_and(|function| !ir_function_requires_trampoline(function))
    };
    let needs_function_resolver = regions.values().any(|region| {
        region_contains(region, |kind| {
            let RegionInstructionKind::NativeCall(call) = kind else {
                return false;
            };
            !matches!(call.result, RegionCallResult::ReferenceLocal(_))
                && call
                    .args
                    .iter()
                    .all(|argument| argument.name.is_none() && !argument.unpack)
                && call.direct_compiled_target().is_some_and(resolver_target)
        })
    });
    let is_direct_linked_call = |call: &RegionNativeCall| {
        direct_linked_signature(call, &request.external_function_signatures).is_some()
    };
    let is_direct_linked_variadic_call = |call: &RegionNativeCall| {
        direct_linked_signature(call, &request.external_function_signatures)
            .and_then(|signature| signature.native_params.last())
            .is_some_and(|parameter| parameter.variadic)
    };
    let needs_call_trampoline = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::NativeCall(call)
                    if call.direct_compiled_target().is_none()
                        && !matches!(call.target, RegionCallTarget::Semantic { .. })
                        && !is_direct_linked_call(call)
            )
        }) || region
            .direct_callees()
            .iter()
            .any(|callee| !regions.contains_key(callee) && !resolver_target(*callee))
            || region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::NativeCall(call)
                        if !matches!(call.target, RegionCallTarget::Semantic { .. })
                            && (matches!(call.result, RegionCallResult::ReferenceLocal(_))
                            || call.args.iter().any(|argument| {
                                argument.name.is_some() || argument.unpack
                            })
                            || call
                                .direct_compiled_target()
                                .is_some_and(|target| trampoline_functions.contains(&target)))
                )
            })
    });
    let needs_baseline_builtin_dispatch = baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if baseline_builtin_helper_id(&call.target).is_some())
            })
        });
    let needs_exact_symbol_query: [bool; StableSymbolQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_symbol_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_pcre: [bool; StablePcreBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_pcre(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_preg_callback = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::ArrayCallback(call)
                        if call.operation == RegionArrayCallbackOperation::PregReplace
                ) || matches!(kind, RegionInstructionKind::PregCallbackArray(_))
            })
        });
    let needs_exact_json: [bool; StableJsonBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_json(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_format: [bool; StableFormatBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_format(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_hash: [bool; StableHashBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_hash(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_byte_codec: [bool; StableByteCodecBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_byte_codec(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_string_search_compare: [bool; StableStringSearchCompareBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_string_search_compare(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_string_rewrite: [bool; StableStringRewriteBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_string_rewrite(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_html_codec: [bool; StableHtmlCodecBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_html_codec(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_url_query: [bool; StableUrlQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_url_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_array_aggregate: [bool; StableArrayAggregateBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_array_aggregate(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_recursive_array: [bool; StableRecursiveArrayBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_recursive_array(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_array_sort: [bool; StableArraySortBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_array_sort(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_array_multisort = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_array_multisort(&call.target))
            })
        });
    let needs_exact_object_identity: [bool; StableObjectIdentityBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_object_identity(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_callable_query: [bool; StableCallableQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_callable_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_callback_handler: [bool; StableCallbackHandlerBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_callback_handler(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_autoload_callback: [bool; StableAutoloadCallbackBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_autoload_callback(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_shutdown_callback = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_shutdown_callback(&call.target))
            })
        });
    let needs_exact_serialization: [bool; StableSerializationBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_serialization(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_tokenizer: [bool; StableTokenizerBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_tokenizer(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_mbstring: [bool; StableMbstringBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_mbstring(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_bcmath: [bool; StableBcmathBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_bcmath(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_filter: [bool; StableFilterBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_filter(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_session: [bool; StableSessionBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_session(&call.target)
                        .is_some_and(|builtin| {
                            builtin.index() == index
                                && builtin.accepts_arity(call.args.len())
                        }))
                })
            })
    });
    let needs_exact_object_vars: [bool; StableObjectVarsBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_object_vars(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_class_metadata: [bool; StableClassMetadataBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_class_metadata(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_class_lineage: [bool; StableClassLineageBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_class_lineage(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_extension_query: [bool; StableExtensionQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_extension_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_memory_query: [bool; StableMemoryQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_memory_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_gc: [bool; StableGcBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_gc(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_resource_query: [bool; StableResourceQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_resource_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_error_state: [bool; StableErrorStateBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_error_state(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_settype = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_settype(&call.target))
            })
        });
    let needs_exact_configuration: [bool; StableConfigurationBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_configuration(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_http_response: [bool; StableHttpResponseBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_http_response(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_cookie: [bool; StableCookieBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_cookie(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_clock: [bool; StableClockBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_clock(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_date: [bool; StableDateBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_date(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_random: [bool; StableRandomBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_random(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_request_query: [bool; StableRequestQueryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_request_query(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_declaration_inventory: [bool; StableDeclarationInventoryBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_declaration_inventory(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_constant_inventory = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_constant_inventory(&call.target))
            })
        });
    let needs_exact_compact = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_compact(&call.target)
                        || stable_builtin_get_defined_vars(&call.target))
            })
        });
    let needs_exact_frame_introspection: [bool; StableFrameIntrospectionBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_frame_introspection(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_base_conversion: [bool; StableBaseConversionBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_base_conversion(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_intval_base = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_scalar_consumer(&call.target)
                        == Some(StableScalarConsumerBuiltin::IntVal)
                        && call.args.len() == 2)
            })
        });
    let needs_exact_network_address: [bool; StableNetworkAddressBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_network_address(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_compression_codec: [bool; StableCompressionCodecBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_compression_codec(&call.target)
                                .is_some_and(|builtin| builtin.index() == index))
                    })
                })
        });
    let needs_exact_path: [bool; StablePathBuiltin::COUNT] = std::array::from_fn(|index| {
        !baseline_helper_imports
            && regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(kind, RegionInstructionKind::NativeCall(call)
                        if stable_builtin_path(&call.target)
                            .is_some_and(|builtin| builtin.index() == index))
                })
            })
    });
    let needs_exact_output_buffer: [bool; StableOutputBufferBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        matches!(kind, RegionInstructionKind::NativeCall(call)
                            if stable_builtin_output_buffer(&call.target)
                                .is_some_and(|builtin| builtin.index() == index
                                    && builtin.accepts_arity(call.args.len())))
                    })
                })
        });
    let needs_exact_pure_math: [bool; StablePureMathBuiltin::COUNT] =
        std::array::from_fn(|index| {
            !baseline_helper_imports
                && regions.values().any(|region| {
                    region_contains(region, |kind| {
                        let RegionInstructionKind::NativeCall(call) = kind else {
                            return false;
                        };
                        stable_builtin_pure_math(&call.target).is_some_and(|builtin| {
                            builtin.index() == index
                                && builtin.accepts_arity(call.args.len())
                                && call.args.iter().enumerate().all(|(argument, metadata)| {
                                    metadata.name.is_none()
                                        && !metadata.unpack
                                        && call
                                            .operands
                                            .get(
                                                call.argument_operand_offset
                                                    .saturating_add(argument),
                                            )
                                            .is_some_and(Option::is_some)
                                })
                        })
                    })
                })
        });
    let needs_semantic_dispatch = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
                if matches!(call.target, RegionCallTarget::Semantic { .. }))
        })
    });
    let needs_frame_arena = runtime_helpers.native_frame_alloc != 0
        && runtime_helpers.native_frame_release != 0
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::NativeCall(_))
            })
        });
    if baseline_helper_imports
        && needs_call_trampoline
        && runtime_helpers.baseline_call_dispatch == 0
    {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_NATIVE_CALL_TRAMPOLINE",
            "dynamic or complex call requires the typed native dispatch trampoline",
        ));
    }
    if baseline_helper_imports
        && needs_baseline_builtin_dispatch
        && runtime_helpers.baseline_builtin_dispatch == 0
    {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_NATIVE_BUILTIN_DISPATCH",
            "direct builtin call requires the stable-ID native builtin dispatcher",
        ));
    }
    if baseline_helper_imports
        && needs_semantic_dispatch
        && runtime_helpers.baseline_semantic_dispatch == 0
    {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_NATIVE_SEMANTIC_DISPATCH",
            "typed semantic operation requires the direct semantic dispatcher",
        ));
    }
    let needs_dynamic_code = regions.values().any(RegionGraph::has_native_dynamic_code);
    if baseline_helper_imports && needs_dynamic_code && runtime_helpers.native_dynamic_code == 0 {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_NATIVE_DYNAMIC_CODE",
            "include, eval, or runtime declaration requires the native dynamic-code compiler",
        ));
    }
    let baseline_call_symbol = BASELINE_NATIVE_CALL_DISPATCH_SYMBOL.to_owned();
    let native_builtin_dispatch_symbol = BASELINE_NATIVE_BUILTIN_DISPATCH_SYMBOL.to_owned();
    let baseline_semantic_dispatch_symbol = BASELINE_NATIVE_SEMANTIC_DISPATCH_SYMBOL.to_owned();
    let native_function_resolve_symbol = NATIVE_FUNCTION_RESOLVE_SYMBOL.to_owned();
    let native_dynamic_code_symbol = NATIVE_DYNAMIC_CODE_SYMBOL.to_owned();
    let needs_unary = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Unary { .. }
                    | RegionInstructionKind::EmptyDim { .. }
                    | RegionInstructionKind::EmptyLocal { .. }
            )
        })
    });
    let mut needs_exact_unary = [false; NATIVE_EXACT_UNARY_COUNT];
    for operation in NATIVE_EXACT_UNARY_OPERATIONS {
        needs_exact_unary[native_exact_unary_index(operation)] = regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::Unary { op, .. } if *op == operation
                )
            })
        });
    }
    let mut needs_exact_compare = [false; NATIVE_EXACT_COMPARE_COUNT];
    for operation in NATIVE_EXACT_COMPARE_OPERATIONS {
        needs_exact_compare[native_exact_compare_index(operation)] =
            regions.values().any(|region| {
                region_contains(region, |kind| {
                    matches!(
                        kind,
                        RegionInstructionKind::Compare { op, .. } if *op == operation
                    ) || operation == RegionCompareOpCode::Spaceship
                        && matches!(
                            kind,
                            RegionInstructionKind::NativeCall(call)
                                if stable_builtin_extrema(&call.target).is_some()
                        )
                })
            });
    }
    let needs_baseline_binary = baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(kind, RegionInstructionKind::Binary { .. })
            })
        });
    let mut needs_exact_binary = [false; NATIVE_EXACT_BINARY_COUNT];
    for operation in NATIVE_EXACT_BINARY_OPERATIONS {
        needs_exact_binary[native_exact_binary_index(operation)] = regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::Binary { op, .. } if *op == operation
                ) || operation == RegionBinaryOp::Pow
                    && matches!(
                        kind,
                        RegionInstructionKind::NativeCall(call)
                            if stable_builtin_numeric_operator(&call.target)
                                == Some(StableNumericOperatorBuiltin::Pow)
                    )
            })
        });
    }
    let needs_compare = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Compare { .. }
                    | RegionInstructionKind::IssetDim { .. }
                    | RegionInstructionKind::IssetLocal { .. }
            )
        })
    });
    let needs_cast = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Cast { .. }
                    | RegionInstructionKind::EmptyDim { .. }
                    | RegionInstructionKind::EmptyLocal { .. }
            )
        })
    });
    let needs_float_to_string = regions.iter().any(|(function, region)| {
        let value_flow = &value_flows[function];
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Echo { .. } | RegionInstructionKind::Compare { .. }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call)
            if stable_builtin_array_lookup(&call.target).is_some()
                || stable_builtin_extrema(&call.target).is_some()
                || optimizing_strval_uses_float_handler(
                    call,
                    value_flow,
                    &unit.constants,
                ))
        })
    });
    let call_scalar_helper_needs = regions.iter().fold(
        OptimizingCallScalarHelperNeeds::default(),
        |mut needs, (function, region)| {
            let value_flow = &value_flows[function];
            let return_needs = optimizing_return_scalar_helper_needs(
                region,
                value_flow,
                &unit.constants,
                region.strict_types,
            );
            needs.numeric_string |= return_needs.numeric_string;
            needs.string_cast |= return_needs.string_cast;
            for call in region
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .filter_map(|instruction| {
                    let RegionInstructionKind::NativeCall(call) = &instruction.kind else {
                        return None;
                    };
                    Some(call)
                })
            {
                let call_needs = optimizing_call_scalar_helper_needs(
                    call,
                    unit,
                    &request.external_function_signatures,
                    value_flow,
                    &unit.constants,
                );
                needs.numeric_string |= call_needs.numeric_string;
                needs.string_cast |= call_needs.string_cast;
            }
            needs
        },
    );
    let needs_numeric_string = call_scalar_helper_needs.numeric_string
        || regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::Compare { .. }
                        | RegionInstructionKind::Cast {
                            op: RegionCastOp::Int | RegionCastOp::Float,
                            ..
                        }
                ) || matches!(
                kind,
                RegionInstructionKind::NativeCall(call)
                    if stable_builtin_array_lookup(&call.target).is_some()
                        || stable_builtin_extrema(&call.target).is_some()
                        || matches!(
                            stable_builtin_scalar_consumer(&call.target),
                            Some(
                                StableScalarConsumerBuiltin::FloatVal
                                    | StableScalarConsumerBuiltin::IntVal
                                    | StableScalarConsumerBuiltin::StrVal
                            )
                        )
                )
            })
        });
    let needs_fmod_f64 = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
                if stable_builtin_scalar_math(&call.target)
                    == Some(StableScalarMathBuiltin::Fmod))
        })
    });
    let needs_round_f64 = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
                if stable_builtin_numeric_operator(&call.target)
                    == Some(StableNumericOperatorBuiltin::Round))
        })
    });
    let needs_array_cast = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Cast {
                    op: RegionCastOp::Array,
                    ..
                }
            )
        })
    });
    let needs_int_cast = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Cast {
                    op: RegionCastOp::Int,
                    ..
                }
            )
        })
    });
    let needs_float_cast = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Cast {
                    op: RegionCastOp::Float,
                    ..
                }
            )
        })
    });
    let needs_string_cast = call_scalar_helper_needs.string_cast
        || regions.iter().any(|(function, region)| {
            let value_flow = &value_flows[function];
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::Cast {
                        op: RegionCastOp::String,
                        ..
                    }
                ) || matches!(
                    kind,
                    RegionInstructionKind::NativeCall(call)
                        if stable_builtin_scalar_consumer(&call.target)
                            == Some(StableScalarConsumerBuiltin::StrVal)
                            && call.args.len() == 1
                            && direct_fixed_builtin_operand(call, 0).is_some()
                            && !optimizing_strval_uses_float_handler(
                                call,
                                value_flow,
                                &unit.constants,
                            )
                )
            })
        });
    let needs_callback_return_string = regions.values().any(|region| {
        region_contains(region, |kind| match kind {
            RegionInstructionKind::PregCallbackArray(_) => true,
            RegionInstructionKind::ArrayCallback(call) => {
                call.operation == RegionArrayCallbackOperation::PregReplace
            }
            _ => false,
        })
    });
    let needs_object_cast = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Cast {
                    op: RegionCastOp::Object,
                    ..
                }
            )
        })
    });
    let needs_object_class_name = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::FetchObjectClassName {
                    prepared_class: None,
                    ..
                }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call)
            if matches!(
                call.target,
                RegionCallTarget::Semantic {
                    operation: RegionSemanticOp::BoundClosureClass { .. }
                }
            )) || matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_scalar_consumer(&call.target)
                        == Some(StableScalarConsumerBuiltin::GetDebugType))
                || matches!(kind, RegionInstructionKind::NativeCall(call)
                    if stable_builtin_get_class(&call.target))
        })
    });
    let needs_acquire_callable = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::NativeCall(RegionNativeCall {
                        target: RegionCallTarget::Semantic {
                            operation: RegionSemanticOp::AcquireCallable { .. }
                        },
                        ..
                    })
                ) || matches!(
                    kind,
                    RegionInstructionKind::ArrayCallback(call)
                        if matches!(call.callback, RegionArrayCallbackTarget::Runtime(_))
                ) || matches!(
                    kind,
                    RegionInstructionKind::PregCallbackArray(call)
                        if call.entries.iter().any(|entry| matches!(
                            entry.callback,
                            RegionArrayCallbackTarget::Runtime(_)
                        ))
                )
            })
        });
    let needs_resolve_callable = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::NativeCall(RegionNativeCall {
                        target: RegionCallTarget::Semantic {
                            operation: RegionSemanticOp::ResolveCallable {
                                callable: php_ir::instruction::CallableKind::FunctionName { .. },
                                ..
                            }
                        },
                        ..
                    })
                )
            })
        });
    let needs_dynamic_instanceof = !baseline_helper_imports
        && regions.values().any(|region| {
            region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::NativeCall(RegionNativeCall {
                        target: RegionCallTarget::Semantic {
                            operation: RegionSemanticOp::DynamicInstanceOf { .. }
                        },
                        ..
                    })
                )
            })
        });
    let needs_echo = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::Echo { .. })
        })
    });
    let needs_local_fetch = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::LoadLocal { .. }
                    | RegionInstructionKind::FetchDim {
                        array: RegionOperand::Local(_),
                        ..
                    }
                    // Every baseline by-value array operand passes through
                    // the shared reference-payload guard. Even a register or
                    // constant can hold a compatibility reference after a
                    // continuation resume, so these instruction families
                    // need the cold local-fetch boundary as well.
                    | RegionInstructionKind::ArrayInsert { .. }
                    | RegionInstructionKind::ArraySpread { .. }
                    | RegionInstructionKind::AssignDim { .. }
                    | RegionInstructionKind::AppendDim { .. }
                    | RegionInstructionKind::UnsetDim { .. }
                    | RegionInstructionKind::BindReferenceDim { .. }
                    | RegionInstructionKind::IssetDim { .. }
                    | RegionInstructionKind::EmptyDim { .. }
                    | RegionInstructionKind::IssetLocal { .. }
                    | RegionInstructionKind::EmptyLocal { .. }
            )
        })
    });
    let needs_local_store = regions.values().any(|region| {
        region
            .exception_regions
            .iter()
            .any(|handler| handler.catch.is_some() && handler.exception_local.is_some())
            || region_contains(region, |kind| {
                matches!(
                    kind,
                    RegionInstructionKind::StoreLocal { .. }
                        | RegionInstructionKind::AssignLocalResult { .. }
                        | RegionInstructionKind::AssignDim { .. }
                        | RegionInstructionKind::AppendDim { .. }
                        | RegionInstructionKind::UnsetDim { .. }
                        | RegionInstructionKind::BindReferenceDim { .. }
                )
            })
    });
    let needs_value_release = true;
    // Local publication is part of the native frame ABI, not just explicit
    // PHP reference syntax.  Stores, unsets, foreach-by-reference and array
    // root updates can all publish a local through the same helper.  Keep the
    // helper available for every executable region so adding publication to a
    // lowering cannot accidentally make an otherwise supported function
    // uncompilable.
    let needs_reference_bind = true;
    let needs_argument_check = regions.values().any(|region| {
        region
            .params
            .iter()
            .any(|parameter| parameter.type_.is_some())
    }) || (needs_function_resolver
        && unit.functions.iter().any(|function| {
            function
                .params
                .iter()
                .any(|parameter| parameter.type_.is_some())
        }));
    let _has_explicit_reference_bind = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::BindReference { .. }
                    | RegionInstructionKind::BindReferenceDim { .. }
                    | RegionInstructionKind::BindReferenceIntoDim { .. }
                    | RegionInstructionKind::BindReferenceProperty { .. }
                    | RegionInstructionKind::BindReferenceFromProperty { .. }
                    | RegionInstructionKind::BindReferenceFromPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceDimFromProperty { .. }
                    | RegionInstructionKind::InitStaticLocal { .. }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call) if
                call.needs_local_reference_binding()
                    || call.direct_compiled_target().is_some_and(|target| {
                        regions.get(&target).is_some_and(|callee| {
                            callee.params.iter().any(|parameter| parameter.by_ref)
                        })
                    })
            )
        })
    });
    let needs_return_check = true;
    let needs_exception_new = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::NativeControl(RegionNativeControl::MakeException { .. })
            )
        })
    });
    let needs_array_new = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NewArray { .. })
                || matches!(kind, RegionInstructionKind::NativeCall(call)
                    if call.variadic || is_direct_linked_variadic_call(call))
        })
    });
    let needs_object_new = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NewObject { .. })
        })
    });
    let needs_property_fetch = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::FetchProperty { .. }
                    | RegionInstructionKind::FetchDynamicStaticProperty { .. }
                    | RegionInstructionKind::FetchObjectClassName { .. }
                    | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceDimFromProperty { .. }
            )
        })
    });
    let needs_property_assign = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::AssignProperty { .. }
                    | RegionInstructionKind::BindReferenceProperty { .. }
                    | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceDimFromProperty { .. }
            )
        })
    });
    let needs_dynamic_property_slot = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
            if matches!(
                &call.target,
                RegionCallTarget::Semantic {
                    operation: RegionSemanticOp::PropertyFetch {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyAssign {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyUnset {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyDimAssign {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyDimUnset {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    }
                }
            ))
        })
    });
    let needs_dynamic_property_test_slot = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
            if matches!(
                &call.target,
                RegionCallTarget::Semantic {
                    operation: RegionSemanticOp::PropertyIsset {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyEmpty {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyDimIsset {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    } | RegionSemanticOp::PropertyDimEmpty {
                        property: RegionPropertyName::Dynamic(_),
                        ..
                    }
                }
            ))
        })
    });
    let needs_object_clone = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::CloneObject { .. })
        })
    });
    let needs_plain_object_clone = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::CloneObject { plain: true, .. })
        })
    });
    let needs_prepared_closure_new = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::NativeDynamicCode(
                    RegionNativeDynamicCode::MakeClosure { .. }
                )
            )
        })
    });
    let needs_object_clone_with = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::CloneWith { .. })
        })
    });
    let needs_array_insert = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::ArrayInsert { .. }
                    | RegionInstructionKind::AssignDim { .. }
                    | RegionInstructionKind::AppendDim { .. }
                    | RegionInstructionKind::UnsetDim { .. }
                    | RegionInstructionKind::BindReferenceDim { .. }
                    | RegionInstructionKind::BindReferenceIntoDim { .. }
                    | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceDimFromProperty { .. }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call)
                if call.variadic || is_direct_linked_variadic_call(call))
        })
    });
    let needs_array_fetch = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::FetchDim { .. }
                    | RegionInstructionKind::AssignDim { .. }
                    | RegionInstructionKind::AppendDim { .. }
                    | RegionInstructionKind::IssetDim { .. }
                    | RegionInstructionKind::EmptyDim { .. }
                    | RegionInstructionKind::UnsetDim { .. }
                    | RegionInstructionKind::BindReferenceDim { .. }
                    | RegionInstructionKind::BindReferenceIntoDim { .. }
                    | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
                    | RegionInstructionKind::BindReferenceDimFromProperty { .. }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call)
                if stable_builtin_array_key_exists(&call.target))
        })
    });
    let needs_array_unset = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::UnsetDim { .. })
        })
    });
    let needs_array_spread = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::ArraySpread { .. })
        })
    });
    let needs_foreach_init = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::ForeachInit { .. }
                    | RegionInstructionKind::ForeachInitRef { .. }
            )
        })
    });
    let needs_foreach_next = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::ForeachNext { .. }
                    | RegionInstructionKind::ForeachNextRef { .. }
            )
        })
    });
    let needs_foreach_cleanup = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::ForeachCleanup { .. })
        })
    });
    let needs_constant_fetch = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::FetchConst { .. })
        })
    });
    let needs_truthy = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::Unary {
                    op: crate::region_ir::RegionUnaryOp::Not,
                    ..
                } | RegionInstructionKind::Cast {
                    op: crate::region_ir::RegionCastOp::Bool,
                    ..
                } | RegionInstructionKind::EmptyDim { .. }
                    | RegionInstructionKind::EmptyLocal { .. }
            )
        }) || region.blocks.iter().any(|block| {
            matches!(
                block.terminator,
                RegionTerminator::JumpIfFalse { .. }
                    | RegionTerminator::JumpIfTrue { .. }
                    | RegionTerminator::JumpIf { .. }
            )
        })
    });
    let needs_type_predicate = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::NativeCall(call)
                    if stable_builtin_type_predicate(&call.target).is_some()
                        && call.argument_operand_offset == 0
                        && call.args.len() == 1
                        && call.args[0].name.is_none()
                        && !call.args[0].unpack
                        && call.operands.len() == 1
                        && call.operands[0].is_some()
                        && !matches!(call.result, RegionCallResult::ReferenceLocal(_))
            )
        })
    });
    let needs_stable_length = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(
                kind,
                RegionInstructionKind::EmptyDim { .. } | RegionInstructionKind::EmptyLocal { .. }
            ) || matches!(kind, RegionInstructionKind::NativeCall(call) if stable_builtin_length(&call.target).is_some())
        })
    });
    let needs_string_predicate = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::NativeCall(call)
                if stable_builtin_string_predicate(&call.target).is_some())
        })
    });
    let needs_runtime_fatal = regions.values().any(|region| {
        region_contains(region, |kind| {
            matches!(kind, RegionInstructionKind::RuntimeFatal { .. })
        })
    });
    let needs_execution_poll = regions
        .values()
        .any(|region| !region.osr_entries().is_empty());
    // Shadow every generic-runtime requirement with the tier capability.  In
    // particular, the optimizing closure below never declares a helper and
    // therefore cannot smuggle one into code through an unused wrapper.
    let needs_call_trampoline = baseline_helper_imports && needs_call_trampoline;
    let needs_function_resolver = baseline_helper_imports && needs_function_resolver;
    let needs_semantic_dispatch = baseline_helper_imports && needs_semantic_dispatch;
    let needs_frame_arena = baseline_helper_imports && needs_frame_arena;
    let needs_dynamic_code = baseline_helper_imports && needs_dynamic_code;
    let needs_unary = baseline_helper_imports && needs_unary;
    if baseline_helper_imports {
        needs_exact_unary.fill(false);
    }
    if baseline_helper_imports {
        needs_exact_binary.fill(false);
    }
    if baseline_helper_imports {
        needs_exact_compare.fill(false);
    }
    let needs_compare = baseline_helper_imports && needs_compare;
    let needs_float_to_string = !baseline_helper_imports && needs_float_to_string;
    let needs_numeric_string = !baseline_helper_imports && needs_numeric_string;
    let needs_fmod_f64 = !baseline_helper_imports && needs_fmod_f64;
    let needs_round_f64 = !baseline_helper_imports && needs_round_f64;
    let needs_array_cast = !baseline_helper_imports && needs_array_cast;
    let needs_int_cast = !baseline_helper_imports && needs_int_cast;
    let needs_float_cast = !baseline_helper_imports && needs_float_cast;
    let needs_string_cast = !baseline_helper_imports && needs_string_cast;
    let needs_object_cast = !baseline_helper_imports && needs_object_cast;
    let needs_object_class_name = !baseline_helper_imports && needs_object_class_name;
    let needs_cast = baseline_helper_imports && needs_cast;
    let needs_direct_echo = !baseline_helper_imports && needs_echo;
    let needs_echo = baseline_helper_imports && needs_echo;
    let needs_local_fetch = baseline_helper_imports && needs_local_fetch;
    let needs_local_store = baseline_helper_imports && needs_local_store;
    let needs_value_release = baseline_helper_imports && needs_value_release;
    let needs_reference_bind = baseline_helper_imports && needs_reference_bind;
    let needs_argument_check = baseline_helper_imports && needs_argument_check;
    let needs_return_check = baseline_helper_imports && needs_return_check;
    let needs_prepared_exception_new = !baseline_helper_imports && needs_exception_new;
    let needs_exception_new = baseline_helper_imports && needs_exception_new;
    let needs_array_new = baseline_helper_imports && needs_array_new;
    let needs_prepared_object_new = !baseline_helper_imports && needs_object_new;
    let needs_prepared_closure_new = !baseline_helper_imports && needs_prepared_closure_new;
    let needs_object_new = baseline_helper_imports && needs_object_new;
    let needs_property_fetch = baseline_helper_imports && needs_property_fetch;
    let needs_property_assign = baseline_helper_imports && needs_property_assign;
    let needs_object_clone = baseline_helper_imports && needs_object_clone;
    let needs_plain_object_clone = !baseline_helper_imports && needs_plain_object_clone;
    let needs_dynamic_property_slot = !baseline_helper_imports && needs_dynamic_property_slot;
    let needs_dynamic_property_test_slot =
        !baseline_helper_imports && needs_dynamic_property_test_slot;
    let needs_object_clone_with = baseline_helper_imports && needs_object_clone_with;
    let needs_array_insert = baseline_helper_imports && needs_array_insert;
    let needs_array_fetch = baseline_helper_imports && needs_array_fetch;
    let needs_array_unset = baseline_helper_imports && needs_array_unset;
    let needs_array_spread = baseline_helper_imports && needs_array_spread;
    let needs_foreach_init = baseline_helper_imports && needs_foreach_init;
    let needs_foreach_next = baseline_helper_imports && needs_foreach_next;
    let needs_foreach_cleanup = baseline_helper_imports && needs_foreach_cleanup;
    let needs_constant_fetch = baseline_helper_imports && needs_constant_fetch;
    let needs_truthy = baseline_helper_imports && needs_truthy;
    let needs_type_predicate = baseline_helper_imports && needs_type_predicate;
    let needs_stable_length = baseline_helper_imports && needs_stable_length;
    let needs_string_predicate = baseline_helper_imports && needs_string_predicate;
    let needs_runtime_fatal = baseline_helper_imports && needs_runtime_fatal;
    let mut imports = vec![(
        "region-runtime-helper-abi".to_owned(),
        region.compile_metadata.helper_abi_hash as usize,
    )];
    if baseline_helper_imports && needs_call_trampoline {
        imports.push((
            baseline_call_symbol.clone(),
            runtime_helpers.baseline_call_dispatch,
        ));
    }
    if needs_baseline_builtin_dispatch {
        imports.push((
            native_builtin_dispatch_symbol.clone(),
            runtime_helpers.baseline_builtin_dispatch,
        ));
    }
    for builtin in StableSymbolQueryBuiltin::all() {
        if !needs_exact_symbol_query[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StableSymbolQueryBuiltin::Define => runtime_helpers.native_define,
            StableSymbolQueryBuiltin::Defined => runtime_helpers.native_defined,
            StableSymbolQueryBuiltin::Constant => runtime_helpers.native_constant,
            StableSymbolQueryBuiltin::FunctionExists => runtime_helpers.native_function_exists,
            StableSymbolQueryBuiltin::ClassExists => runtime_helpers.native_class_exists,
            StableSymbolQueryBuiltin::InterfaceExists => runtime_helpers.native_interface_exists,
            StableSymbolQueryBuiltin::TraitExists => runtime_helpers.native_trait_exists,
            StableSymbolQueryBuiltin::EnumExists => runtime_helpers.native_enum_exists,
            StableSymbolQueryBuiltin::MethodExists => runtime_helpers.native_method_exists,
            StableSymbolQueryBuiltin::PropertyExists => runtime_helpers.native_property_exists,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_NATIVE_SYMBOL_QUERY",
                format!(
                    "prepared symbol query requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StablePcreBuiltin::all() {
        if !needs_exact_pcre[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StablePcreBuiltin::Match => runtime_helpers.native_preg_match,
            StablePcreBuiltin::MatchAll => runtime_helpers.native_preg_match_all,
            StablePcreBuiltin::Replace => runtime_helpers.native_preg_replace,
            StablePcreBuiltin::Filter => runtime_helpers.native_preg_filter,
            StablePcreBuiltin::Split => runtime_helpers.native_preg_split,
            StablePcreBuiltin::Grep => runtime_helpers.native_preg_grep,
            StablePcreBuiltin::Quote => runtime_helpers.native_preg_quote,
            StablePcreBuiltin::LastError => runtime_helpers.native_preg_last_error,
            StablePcreBuiltin::LastErrorMessage => runtime_helpers.native_preg_last_error_msg,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_PCRE",
                format!(
                    "prepared PCRE builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_preg_callback {
        for (symbol, address) in [
            (
                "phrust_native_preg_callback_plan",
                runtime_helpers.native_preg_callback_plan,
            ),
            (
                "phrust_native_preg_callback_assemble",
                runtime_helpers.native_preg_callback_assemble,
            ),
        ] {
            if address == 0 {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_EXACT_PCRE_CALLBACK",
                    format!("prepared PCRE callback replacement requires {symbol}"),
                ));
            }
            imports.push((symbol.to_owned(), address));
        }
    }
    for builtin in StableJsonBuiltin::all() {
        if !needs_exact_json[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StableJsonBuiltin::Encode => runtime_helpers.native_json_encode,
            StableJsonBuiltin::Decode => runtime_helpers.native_json_decode,
            StableJsonBuiltin::Validate => runtime_helpers.native_json_validate,
            StableJsonBuiltin::LastError => runtime_helpers.native_json_last_error,
            StableJsonBuiltin::LastErrorMessage => runtime_helpers.native_json_last_error_msg,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_JSON",
                format!(
                    "prepared JSON builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableFormatBuiltin::all() {
        if !needs_exact_format[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StableFormatBuiltin::Sprintf => runtime_helpers.native_sprintf,
            StableFormatBuiltin::Printf => runtime_helpers.native_printf,
            StableFormatBuiltin::Vsprintf => runtime_helpers.native_vsprintf,
            StableFormatBuiltin::Vprintf => runtime_helpers.native_vprintf,
            StableFormatBuiltin::NumberFormat => runtime_helpers.native_number_format,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_FORMAT",
                format!(
                    "prepared formatting builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableHashBuiltin::all() {
        if !needs_exact_hash[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StableHashBuiltin::Md5 => runtime_helpers.native_md5,
            StableHashBuiltin::Sha1 => runtime_helpers.native_sha1,
            StableHashBuiltin::Crc32 => runtime_helpers.native_crc32,
            StableHashBuiltin::Hash => runtime_helpers.native_hash,
            StableHashBuiltin::HashHmac => runtime_helpers.native_hash_hmac,
            StableHashBuiltin::HashEquals => runtime_helpers.native_hash_equals,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_HASH",
                format!(
                    "prepared hash builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableByteCodecBuiltin::all() {
        if !needs_exact_byte_codec[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StableByteCodecBuiltin::Base64Encode => runtime_helpers.native_base64_encode,
            StableByteCodecBuiltin::Base64Decode => runtime_helpers.native_base64_decode,
            StableByteCodecBuiltin::Bin2Hex => runtime_helpers.native_bin2hex,
            StableByteCodecBuiltin::Hex2Bin => runtime_helpers.native_hex2bin,
            StableByteCodecBuiltin::QuotedPrintableDecode => {
                runtime_helpers.native_quoted_printable_decode
            }
            StableByteCodecBuiltin::UrlEncode => runtime_helpers.native_urlencode,
            StableByteCodecBuiltin::RawUrlEncode => runtime_helpers.native_rawurlencode,
            StableByteCodecBuiltin::UrlDecode => runtime_helpers.native_urldecode,
            StableByteCodecBuiltin::RawUrlDecode => runtime_helpers.native_rawurldecode,
            StableByteCodecBuiltin::UuEncode => runtime_helpers.native_convert_uuencode,
            StableByteCodecBuiltin::UuDecode => runtime_helpers.native_convert_uudecode,
            StableByteCodecBuiltin::AddCSlashes => runtime_helpers.native_addcslashes,
            StableByteCodecBuiltin::StripCSlashes => runtime_helpers.native_stripcslashes,
            StableByteCodecBuiltin::StripSlashes => runtime_helpers.native_stripslashes,
            StableByteCodecBuiltin::QuoteMeta => runtime_helpers.native_quotemeta,
            StableByteCodecBuiltin::Pack => runtime_helpers.native_pack,
            StableByteCodecBuiltin::Unpack => runtime_helpers.native_unpack,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_BYTE_CODEC",
                format!(
                    "prepared byte-codec builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableStringSearchCompareBuiltin::all() {
        if !needs_exact_string_search_compare[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_string_search_compare[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_STRING_SEARCH_COMPARE",
                format!(
                    "prepared string search/compare builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableStringRewriteBuiltin::all() {
        if !needs_exact_string_rewrite[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_string_rewrite[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_STRING_REWRITE",
                format!(
                    "prepared string rewrite builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableHtmlCodecBuiltin::all() {
        if !needs_exact_html_codec[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_html_codec[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_HTML_CODEC",
                format!(
                    "prepared HTML codec builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableUrlQueryBuiltin::all() {
        if !needs_exact_url_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_url_query[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_URL_QUERY",
                format!(
                    "prepared URL/query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableArrayAggregateBuiltin::all() {
        if !needs_exact_array_aggregate[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_array_aggregate[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_ARRAY_AGGREGATE",
                format!(
                    "prepared array aggregate requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableRecursiveArrayBuiltin::all() {
        if !needs_exact_recursive_array[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_recursive_array[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_RECURSIVE_ARRAY",
                format!(
                    "prepared recursive array operation requires fixed native handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableArraySortBuiltin::all() {
        if !needs_exact_array_sort[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_array_sort[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_ARRAY_PRESERVING_SORT",
                format!(
                    "prepared key-preserving sort requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_exact_array_multisort {
        let address = runtime_helpers.native_array_multisort;
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_ARRAY_MULTISORT",
                "prepared array_multisort requires its fixed native slice handler",
            ));
        }
        imports.push(("phrust_native_array_multisort".to_owned(), address));
    }
    for builtin in StableObjectIdentityBuiltin::all() {
        if !needs_exact_object_identity[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_object_identity[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_OBJECT_IDENTITY",
                format!(
                    "prepared object-identity builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableCallableQueryBuiltin::all() {
        if !needs_exact_callable_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_is_callable;
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CALLABLE_QUERY",
                format!(
                    "prepared callable-query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableCallbackHandlerBuiltin::all() {
        if !needs_exact_callback_handler[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_callback_handler[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CALLBACK_HANDLER",
                format!(
                    "prepared callback-handler builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableAutoloadCallbackBuiltin::all() {
        if !needs_exact_autoload_callback[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_autoload_callback[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_AUTOLOAD_CALLBACK",
                format!(
                    "prepared autoload-callback builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_exact_shutdown_callback {
        let address = runtime_helpers.native_register_shutdown_function;
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_SHUTDOWN_CALLBACK",
                "prepared register_shutdown_function requires its exact native slice handler",
            ));
        }
        imports.push((
            "phrust_native_register_shutdown_function".to_owned(),
            address,
        ));
    }
    for builtin in StableSerializationBuiltin::all() {
        if !needs_exact_serialization[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_serialization[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_SERIALIZATION",
                format!(
                    "prepared serialization builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableTokenizerBuiltin::all() {
        if !needs_exact_tokenizer[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_tokenizer[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_TOKENIZER",
                format!(
                    "prepared tokenizer builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableMbstringBuiltin::all() {
        if !needs_exact_mbstring[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_mbstring[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_MBSTRING",
                format!(
                    "prepared mbstring builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableBcmathBuiltin::all() {
        if !needs_exact_bcmath[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_bcmath[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_BCMATH",
                format!(
                    "prepared bcmath builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableFilterBuiltin::all() {
        if !needs_exact_filter[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_filter[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_FILTER",
                format!(
                    "prepared filter builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableSessionBuiltin::all() {
        if !needs_exact_session[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_session[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_SESSION",
                format!(
                    "prepared session builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableObjectVarsBuiltin::all() {
        if !needs_exact_object_vars[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_object_vars[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_OBJECT_VARS",
                format!(
                    "prepared object-vars builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableClassMetadataBuiltin::all() {
        if !needs_exact_class_metadata[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_class_metadata[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CLASS_METADATA",
                format!(
                    "prepared class-metadata builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableClassLineageBuiltin::all() {
        if !needs_exact_class_lineage[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_class_lineage[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CLASS_LINEAGE",
                format!(
                    "prepared class-lineage builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableExtensionQueryBuiltin::all() {
        if !needs_exact_extension_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_extension_query[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_EXTENSION_QUERY",
                format!(
                    "prepared extension-query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableMemoryQueryBuiltin::all() {
        if !needs_exact_memory_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_memory_query[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_MEMORY_QUERY",
                format!(
                    "prepared memory-query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableGcBuiltin::all() {
        if !needs_exact_gc[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_gc[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_GC",
                format!(
                    "prepared GC builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableResourceQueryBuiltin::all() {
        if !needs_exact_resource_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_resource_query[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_RESOURCE_QUERY",
                format!(
                    "prepared resource-query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableErrorStateBuiltin::all() {
        if !needs_exact_error_state[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_error_state[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_ERROR_STATE",
                format!(
                    "prepared error-state builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_exact_settype {
        if runtime_helpers.native_settype == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_SETTYPE",
                "prepared settype builtin requires exact handler phrust_native_settype",
            ));
        }
        imports.push((
            "phrust_native_settype".to_owned(),
            runtime_helpers.native_settype,
        ));
    }
    for builtin in StableConfigurationBuiltin::all() {
        if !needs_exact_configuration[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_configuration[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CONFIGURATION",
                format!(
                    "prepared configuration builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableHttpResponseBuiltin::all() {
        if !needs_exact_http_response[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_http_response[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_HTTP_RESPONSE",
                format!(
                    "prepared HTTP-response builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableCookieBuiltin::all() {
        if !needs_exact_cookie[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_cookie[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_COOKIE",
                format!(
                    "prepared cookie builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableClockBuiltin::all() {
        if !needs_exact_clock[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_clock[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CLOCK",
                format!(
                    "prepared clock builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableDateBuiltin::all() {
        if !needs_exact_date[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_date[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_DATE",
                format!(
                    "prepared date builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableRandomBuiltin::all() {
        if !needs_exact_random[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_random[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_RANDOM",
                format!(
                    "prepared random builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableRequestQueryBuiltin::all() {
        if !needs_exact_request_query[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_request_query[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_REQUEST_QUERY",
                format!(
                    "prepared request-query builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableDeclarationInventoryBuiltin::all() {
        if !needs_exact_declaration_inventory[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_declaration_inventory[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_DECLARATION_INVENTORY",
                format!(
                    "prepared declaration-inventory builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_exact_constant_inventory {
        if runtime_helpers.native_constant_inventory == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_CONSTANT_INVENTORY",
                "prepared constant-inventory builtin requires exact handler phrust_native_get_defined_constants",
            ));
        }
        imports.push((
            "phrust_native_get_defined_constants".to_owned(),
            runtime_helpers.native_constant_inventory,
        ));
    }
    if needs_exact_compact {
        if runtime_helpers.native_compact == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_COMPACT",
                "prepared compact builtin requires exact handler phrust_native_compact",
            ));
        }
        imports.push((
            "phrust_native_compact".to_owned(),
            runtime_helpers.native_compact,
        ));
    }
    for builtin in StableFrameIntrospectionBuiltin::all() {
        if !needs_exact_frame_introspection[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_frame_introspection[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_FRAME_INTROSPECTION",
                format!(
                    "prepared frame-introspection builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableBaseConversionBuiltin::all() {
        if !needs_exact_base_conversion[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_base_conversion[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_BASE_CONVERSION",
                format!(
                    "prepared base-conversion builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if needs_exact_intval_base {
        if runtime_helpers.native_intval_base == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_INTVAL_BASE",
                "prepared two-argument intval requires exact handler phrust_native_intval_base",
            ));
        }
        imports.push((
            "phrust_native_intval_base".to_owned(),
            runtime_helpers.native_intval_base,
        ));
    }
    for builtin in StableNetworkAddressBuiltin::all() {
        if !needs_exact_network_address[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_network_address[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_NETWORK_ADDRESS",
                format!(
                    "prepared network-address builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableCompressionCodecBuiltin::all() {
        if !needs_exact_compression_codec[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_compression_codec[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_COMPRESSION_CODEC",
                format!(
                    "prepared compression-codec builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StablePathBuiltin::all() {
        if !needs_exact_path[builtin.index()] {
            continue;
        }
        let address = match builtin {
            StablePathBuiltin::Basename => runtime_helpers.native_basename,
            StablePathBuiltin::Dirname => runtime_helpers.native_dirname,
            StablePathBuiltin::Realpath => runtime_helpers.native_realpath,
            StablePathBuiltin::FileExists => runtime_helpers.native_file_exists,
            StablePathBuiltin::IsFile => runtime_helpers.native_is_file,
            StablePathBuiltin::IsDir => runtime_helpers.native_is_dir,
            StablePathBuiltin::IsReadable => runtime_helpers.native_is_readable,
            StablePathBuiltin::IsWritable => runtime_helpers.native_is_writable,
            StablePathBuiltin::IsLink => runtime_helpers.native_is_link,
            StablePathBuiltin::FilePerms => runtime_helpers.native_fileperms,
            StablePathBuiltin::FileOwner => runtime_helpers.native_fileowner,
            StablePathBuiltin::FileGroup => runtime_helpers.native_filegroup,
            StablePathBuiltin::FileType => runtime_helpers.native_filetype,
            StablePathBuiltin::DiskFreeSpace => runtime_helpers.native_disk_free_space,
            StablePathBuiltin::DiskTotalSpace => runtime_helpers.native_disk_total_space,
            StablePathBuiltin::Pathinfo => runtime_helpers.native_pathinfo,
            StablePathBuiltin::Stat => runtime_helpers.native_stat,
            StablePathBuiltin::Lstat => runtime_helpers.native_lstat,
            StablePathBuiltin::File => runtime_helpers.native_file,
            StablePathBuiltin::Glob => runtime_helpers.native_glob,
            StablePathBuiltin::OpenDir => runtime_helpers.native_opendir,
            StablePathBuiltin::ReadDir => runtime_helpers.native_readdir,
            StablePathBuiltin::RewindDir => runtime_helpers.native_rewinddir,
            StablePathBuiltin::CloseDir => runtime_helpers.native_closedir,
            StablePathBuiltin::ScanDir => runtime_helpers.native_scandir,
            StablePathBuiltin::StreamGetMetaData => runtime_helpers.native_stream_get_meta_data,
            StablePathBuiltin::StreamGetWrappers => runtime_helpers.native_stream_get_wrappers,
            StablePathBuiltin::StreamIsLocal => runtime_helpers.native_stream_is_local,
            StablePathBuiltin::StreamResolveIncludePath => {
                runtime_helpers.native_stream_resolve_include_path
            }
            StablePathBuiltin::StreamContextCreate => runtime_helpers.native_stream_context_create,
            StablePathBuiltin::StreamContextGetDefault => {
                runtime_helpers.native_stream_context_get_default
            }
            StablePathBuiltin::StreamContextGetOptions => {
                runtime_helpers.native_stream_context_get_options
            }
            StablePathBuiltin::StreamContextSetDefault => {
                runtime_helpers.native_stream_context_set_default
            }
            StablePathBuiltin::StreamContextSetOption => {
                runtime_helpers.native_stream_context_set_option
            }
            StablePathBuiltin::StreamContextSetOptions => {
                runtime_helpers.native_stream_context_set_options
            }
            StablePathBuiltin::StreamFilterAppend => runtime_helpers.native_stream_filter_append,
            StablePathBuiltin::StreamFilterPrepend => runtime_helpers.native_stream_filter_prepend,
            StablePathBuiltin::StreamFilterRemove => runtime_helpers.native_stream_filter_remove,
            StablePathBuiltin::StreamIsAtty => runtime_helpers.native_stream_isatty,
            StablePathBuiltin::StreamSetTimeout => runtime_helpers.native_stream_set_timeout,
            StablePathBuiltin::Chmod => runtime_helpers.native_chmod,
            StablePathBuiltin::Symlink => runtime_helpers.native_symlink,
            StablePathBuiltin::Readfile => runtime_helpers.native_readfile,
            StablePathBuiltin::IsUploadedFile => runtime_helpers.native_is_uploaded_file,
            StablePathBuiltin::Tempnam => runtime_helpers.native_tempnam,
            StablePathBuiltin::Tmpfile => runtime_helpers.native_tmpfile,
            StablePathBuiltin::Filesize => runtime_helpers.native_filesize,
            StablePathBuiltin::Filemtime => runtime_helpers.native_filemtime,
            StablePathBuiltin::FileGetContents => runtime_helpers.native_file_get_contents,
            StablePathBuiltin::FilePutContents => runtime_helpers.native_file_put_contents,
            StablePathBuiltin::Rename => runtime_helpers.native_rename,
            StablePathBuiltin::Unlink => runtime_helpers.native_unlink,
            StablePathBuiltin::Mkdir => runtime_helpers.native_mkdir,
            StablePathBuiltin::Rmdir => runtime_helpers.native_rmdir,
            StablePathBuiltin::Touch => runtime_helpers.native_touch,
            StablePathBuiltin::Fopen => runtime_helpers.native_fopen,
            StablePathBuiltin::Fwrite => runtime_helpers.native_fwrite,
            StablePathBuiltin::Fclose => runtime_helpers.native_fclose,
            StablePathBuiltin::Fread => runtime_helpers.native_fread,
            StablePathBuiltin::Fgets => runtime_helpers.native_fgets,
            StablePathBuiltin::Fgetc => runtime_helpers.native_fgetc,
            StablePathBuiltin::Feof => runtime_helpers.native_feof,
            StablePathBuiltin::Fflush => runtime_helpers.native_fflush,
            StablePathBuiltin::Fseek => runtime_helpers.native_fseek,
            StablePathBuiltin::Ftell => runtime_helpers.native_ftell,
            StablePathBuiltin::Ftruncate => runtime_helpers.native_ftruncate,
            StablePathBuiltin::Rewind => runtime_helpers.native_rewind,
            StablePathBuiltin::StreamGetContents => runtime_helpers.native_stream_get_contents,
            StablePathBuiltin::StreamCopyToStream => runtime_helpers.native_stream_copy_to_stream,
        };
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_PATH",
                format!(
                    "prepared path builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    for builtin in StableOutputBufferBuiltin::all() {
        if !needs_exact_output_buffer[builtin.index()] {
            continue;
        }
        let address = runtime_helpers.native_output_buffer[builtin.index()];
        if address == 0 {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_EXACT_OUTPUT_BUFFER",
                format!(
                    "prepared output-buffer builtin requires exact handler {}",
                    builtin.symbol()
                ),
            ));
        }
        imports.push((builtin.symbol().to_owned(), address));
    }
    if baseline_helper_imports && needs_semantic_dispatch {
        imports.push((
            baseline_semantic_dispatch_symbol.clone(),
            runtime_helpers.baseline_semantic_dispatch,
        ));
    }
    if baseline_helper_imports && needs_function_resolver {
        imports.push((
            native_function_resolve_symbol.clone(),
            runtime_helpers.native_function_resolve,
        ));
    }
    if baseline_helper_imports && needs_frame_arena {
        imports.push((
            "phrust_native_frame_alloc".to_owned(),
            runtime_helpers.native_frame_alloc,
        ));
        imports.push((
            "phrust_native_frame_release".to_owned(),
            runtime_helpers.native_frame_release,
        ));
    }
    if baseline_helper_imports && needs_dynamic_code {
        imports.push((
            native_dynamic_code_symbol.clone(),
            runtime_helpers.native_dynamic_code,
        ));
    }
    for (needed, configured, fallback, symbol) in [
        (
            needs_unary,
            runtime_helpers.baseline_unary,
            test_native_unary_fallback as *const () as usize,
            "phrust_baseline_native_unary",
        ),
        (
            needs_baseline_binary,
            runtime_helpers.baseline_binary,
            test_baseline_binary_fallback as *const () as usize,
            "phrust_baseline_native_binary",
        ),
        (
            needs_compare,
            runtime_helpers.baseline_compare,
            test_native_compare_fallback as *const () as usize,
            "phrust_baseline_native_compare",
        ),
        (
            needs_cast,
            runtime_helpers.baseline_cast,
            test_native_cast_fallback as *const () as usize,
            "phrust_baseline_native_cast",
        ),
        (
            needs_echo,
            runtime_helpers.native_echo,
            test_native_echo_fallback as *const () as usize,
            "phrust_native_echo",
        ),
        (
            needs_local_fetch,
            runtime_helpers.native_local_fetch,
            test_native_local_fetch_fallback as *const () as usize,
            "phrust_native_local_fetch",
        ),
        (
            needs_local_store,
            runtime_helpers.native_local_store,
            test_native_local_store_fallback as *const () as usize,
            "phrust_native_local_store",
        ),
        (
            needs_value_release,
            runtime_helpers.native_value_release,
            test_native_value_release_fallback as *const () as usize,
            "phrust_native_value_release",
        ),
        (
            needs_reference_bind,
            runtime_helpers.native_reference_bind,
            test_native_reference_bind_fallback as *const () as usize,
            "phrust_native_reference_bind",
        ),
        (
            needs_argument_check,
            runtime_helpers.native_argument_check,
            test_native_argument_check_fallback as *const () as usize,
            "phrust_native_argument_check",
        ),
        (
            needs_return_check,
            runtime_helpers.native_return_check,
            test_native_return_check_fallback as *const () as usize,
            "phrust_native_return_check",
        ),
        (
            needs_exception_new,
            runtime_helpers.native_exception_new,
            test_native_exception_new_fallback as *const () as usize,
            "phrust_native_exception_new",
        ),
        (
            needs_array_new,
            runtime_helpers.native_array_new,
            test_native_array_new_fallback as *const () as usize,
            "phrust_native_array_new",
        ),
        (
            needs_object_new,
            runtime_helpers.native_object_new,
            test_native_object_new_fallback as *const () as usize,
            "phrust_native_object_new",
        ),
        (
            needs_property_fetch,
            runtime_helpers.native_property_fetch,
            test_native_property_fetch_fallback as *const () as usize,
            "phrust_native_property_fetch",
        ),
        (
            needs_property_assign,
            runtime_helpers.native_property_assign,
            test_native_property_assign_fallback as *const () as usize,
            "phrust_native_property_assign",
        ),
        (
            needs_object_clone,
            runtime_helpers.native_object_clone,
            test_native_object_clone_fallback as *const () as usize,
            "phrust_native_object_clone",
        ),
        (
            needs_object_clone_with,
            runtime_helpers.native_object_clone_with,
            test_native_object_clone_with_fallback as *const () as usize,
            "phrust_native_object_clone_with",
        ),
        (
            needs_array_insert,
            runtime_helpers.native_array_insert,
            test_native_array_insert_fallback as *const () as usize,
            "phrust_native_array_insert",
        ),
        (
            needs_array_insert,
            runtime_helpers.native_array_insert_local,
            test_native_array_insert_fallback as *const () as usize,
            "phrust_native_array_insert_local",
        ),
        (
            needs_array_fetch,
            runtime_helpers.native_array_fetch,
            test_native_array_fetch_fallback as *const () as usize,
            "phrust_native_array_fetch",
        ),
        (
            needs_array_unset,
            runtime_helpers.native_array_unset,
            test_native_array_unset_fallback as *const () as usize,
            "phrust_native_array_unset",
        ),
        (
            needs_array_spread,
            runtime_helpers.native_array_spread,
            test_native_array_spread_fallback as *const () as usize,
            "phrust_native_array_spread",
        ),
        (
            needs_foreach_init,
            runtime_helpers.native_foreach_init,
            test_native_foreach_init_fallback as *const () as usize,
            "phrust_native_foreach_init",
        ),
        (
            needs_foreach_next,
            runtime_helpers.native_foreach_next,
            test_native_foreach_next_fallback as *const () as usize,
            "phrust_native_foreach_next",
        ),
        (
            needs_foreach_cleanup,
            runtime_helpers.native_foreach_cleanup,
            test_native_foreach_cleanup_fallback as *const () as usize,
            "phrust_native_foreach_cleanup",
        ),
        (
            needs_constant_fetch,
            runtime_helpers.native_constant_fetch,
            test_native_constant_fetch_fallback as *const () as usize,
            "phrust_native_constant_fetch",
        ),
        (
            needs_truthy,
            runtime_helpers.native_truthy,
            test_native_truthy_fallback as *const () as usize,
            "phrust_native_truthy",
        ),
        (
            needs_type_predicate,
            runtime_helpers.native_type_predicate,
            test_native_type_predicate_fallback as *const () as usize,
            "phrust_native_type_predicate",
        ),
        (
            needs_stable_length,
            runtime_helpers.native_stable_length,
            test_native_stable_length_fallback as *const () as usize,
            "phrust_native_stable_length",
        ),
        (
            needs_string_predicate,
            runtime_helpers.native_string_predicate,
            test_native_string_predicate_fallback as *const () as usize,
            "phrust_native_string_predicate",
        ),
        (
            needs_runtime_fatal,
            runtime_helpers.native_runtime_fatal,
            test_native_runtime_fatal_fallback as *const () as usize,
            "phrust_native_runtime_fatal",
        ),
        (
            needs_execution_poll,
            runtime_helpers.native_execution_poll,
            test_native_execution_poll_fallback as *const () as usize,
            "phrust_native_execution_poll",
        ),
    ] {
        if needed && (baseline_helper_imports || symbol == "phrust_native_execution_poll") {
            let address = if configured == 0 {
                fallback
            } else {
                configured
            };
            imports.push((symbol.to_owned(), address));
        }
    }
    for operation in NATIVE_EXACT_BINARY_OPERATIONS {
        let index = native_exact_binary_index(operation);
        if !needs_exact_binary[index] {
            continue;
        }
        let configured = runtime_helpers.native_binary[index];
        imports.push((
            native_exact_binary_symbol(operation).to_owned(),
            if configured == 0 {
                test_native_exact_binary_fallback as *const () as usize
            } else {
                configured
            },
        ));
    }
    for operation in NATIVE_EXACT_UNARY_OPERATIONS {
        let index = native_exact_unary_index(operation);
        if !needs_exact_unary[index] {
            continue;
        }
        let configured = runtime_helpers.native_exact_unary[index];
        imports.push((
            native_exact_unary_symbol(operation).to_owned(),
            if configured == 0 {
                test_native_exact_unary_fallback as *const () as usize
            } else {
                configured
            },
        ));
    }
    for operation in NATIVE_EXACT_COMPARE_OPERATIONS {
        let index = native_exact_compare_index(operation);
        if !needs_exact_compare[index] {
            continue;
        }
        let configured = runtime_helpers.native_exact_compare[index];
        imports.push((
            native_exact_compare_symbol(operation).to_owned(),
            if configured == 0 {
                test_native_exact_compare_fallback as *const () as usize
            } else {
                configured
            },
        ));
    }
    if needs_direct_echo {
        imports.push((
            "phrust_native_echo_bytes".to_owned(),
            if runtime_helpers.native_echo_bytes == 0 {
                test_native_echo_bytes_fallback as *const () as usize
            } else {
                runtime_helpers.native_echo_bytes
            },
        ));
    }
    if needs_float_to_string {
        imports.push((
            "phrust_native_float_to_string".to_owned(),
            if runtime_helpers.native_float_to_string == 0 {
                test_native_float_to_string_fallback as *const () as usize
            } else {
                runtime_helpers.native_float_to_string
            },
        ));
    }
    if needs_numeric_string {
        imports.push((
            "phrust_native_numeric_string".to_owned(),
            if runtime_helpers.native_numeric_string == 0 {
                test_native_numeric_string_fallback as *const () as usize
            } else {
                runtime_helpers.native_numeric_string
            },
        ));
    }
    if needs_fmod_f64 {
        imports.push((
            "phrust_native_fmod_f64".to_owned(),
            if runtime_helpers.native_fmod_f64 == 0 {
                test_native_fmod_f64_fallback as *const () as usize
            } else {
                runtime_helpers.native_fmod_f64
            },
        ));
    }
    if needs_round_f64 {
        imports.push((
            "phrust_native_round_f64".to_owned(),
            if runtime_helpers.native_round_f64 == 0 {
                test_native_round_f64_fallback as *const () as usize
            } else {
                runtime_helpers.native_round_f64
            },
        ));
    }
    for builtin in StablePureMathBuiltin::all() {
        if !needs_exact_pure_math[builtin.index()] {
            continue;
        }
        let configured = runtime_helpers.native_pure_math[builtin.index()];
        imports.push((
            builtin.symbol().to_owned(),
            if configured == 0 {
                test_native_pure_math_fallback(builtin)
            } else {
                configured
            },
        ));
    }
    if needs_array_cast {
        imports.push((
            "phrust_native_array_cast".to_owned(),
            if runtime_helpers.native_array_cast == 0 {
                test_native_array_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_array_cast
            },
        ));
    }
    if needs_int_cast {
        imports.push((
            "phrust_native_int_cast".to_owned(),
            if runtime_helpers.native_int_cast == 0 {
                test_native_int_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_int_cast
            },
        ));
    }
    if needs_float_cast {
        imports.push((
            "phrust_native_float_cast".to_owned(),
            if runtime_helpers.native_float_cast == 0 {
                test_native_float_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_float_cast
            },
        ));
    }
    if needs_string_cast {
        imports.push((
            "phrust_native_string_cast".to_owned(),
            if runtime_helpers.native_string_cast == 0 {
                test_native_string_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_string_cast
            },
        ));
    }
    if needs_callback_return_string {
        imports.push((
            "phrust_native_callback_return_string".to_owned(),
            if runtime_helpers.native_callback_return_string == 0 {
                test_native_string_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_callback_return_string
            },
        ));
    }
    if needs_object_class_name {
        imports.push((
            "phrust_native_object_class_name".to_owned(),
            if runtime_helpers.native_object_class_name == 0 {
                test_native_object_class_name_fallback as *const () as usize
            } else {
                runtime_helpers.native_object_class_name
            },
        ));
    }
    if needs_acquire_callable {
        imports.push((
            "phrust_native_acquire_callable".to_owned(),
            if runtime_helpers.native_acquire_callable == 0 {
                test_native_object_class_name_fallback as *const () as usize
            } else {
                runtime_helpers.native_acquire_callable
            },
        ));
    }
    if needs_resolve_callable {
        imports.push((
            "phrust_native_resolve_callable".to_owned(),
            if runtime_helpers.native_resolve_callable == 0 {
                test_native_object_class_name_fallback as *const () as usize
            } else {
                runtime_helpers.native_resolve_callable
            },
        ));
    }
    if needs_dynamic_instanceof {
        imports.push((
            "phrust_native_dynamic_instanceof".to_owned(),
            if runtime_helpers.native_dynamic_instanceof == 0 {
                test_native_object_class_name_fallback as *const () as usize
            } else {
                runtime_helpers.native_dynamic_instanceof
            },
        ));
    }
    if needs_object_cast {
        imports.push((
            "phrust_native_object_cast".to_owned(),
            if runtime_helpers.native_object_cast == 0 {
                test_native_object_cast_fallback as *const () as usize
            } else {
                runtime_helpers.native_object_cast
            },
        ));
    }
    if needs_prepared_object_new {
        imports.push((
            "phrust_native_prepared_object_new".to_owned(),
            if runtime_helpers.native_prepared_object_new == 0 {
                test_native_prepared_object_new_fallback as *const () as usize
            } else {
                runtime_helpers.native_prepared_object_new
            },
        ));
    }
    if needs_prepared_exception_new {
        imports.push((
            "phrust_native_prepared_exception_new".to_owned(),
            if runtime_helpers.native_prepared_exception_new == 0 {
                test_native_prepared_exception_new_fallback as *const () as usize
            } else {
                runtime_helpers.native_prepared_exception_new
            },
        ));
    }
    if needs_prepared_closure_new {
        imports.push((
            "phrust_native_prepared_closure_new".to_owned(),
            if runtime_helpers.native_prepared_closure_new == 0 {
                test_native_prepared_closure_new_fallback as *const () as usize
            } else {
                runtime_helpers.native_prepared_closure_new
            },
        ));
    }
    if needs_plain_object_clone {
        imports.push((
            "phrust_native_plain_object_clone".to_owned(),
            if runtime_helpers.native_plain_object_clone == 0 {
                test_native_plain_object_clone_fallback as *const () as usize
            } else {
                runtime_helpers.native_plain_object_clone
            },
        ));
    }
    if needs_dynamic_property_slot {
        imports.push((
            "phrust_native_dynamic_property_slot".to_owned(),
            if runtime_helpers.native_dynamic_property_slot == 0 {
                test_native_dynamic_property_fallback as *const () as usize
            } else {
                runtime_helpers.native_dynamic_property_slot
            },
        ));
    }
    if needs_dynamic_property_test_slot {
        imports.push((
            "phrust_native_dynamic_property_test_slot".to_owned(),
            if runtime_helpers.native_dynamic_property_test_slot == 0 {
                test_native_dynamic_property_fallback as *const () as usize
            } else {
                runtime_helpers.native_dynamic_property_test_slot
            },
        ));
    }
    #[cfg(test)]
    {
        let aliases = imports
            .iter()
            .skip(1)
            .map(|(symbol, address)| (native_helper_import_symbol(symbol, *address), *address))
            .collect::<Vec<_>>();
        imports.extend(aliases);
    }
    let import_refs = imports
        .iter()
        .map(|(name, address)| (name.as_str(), *address))
        .collect::<Vec<_>>();
    let function_key = native_function_key(
        request
            .deployment_identity
            .clone()
            .unwrap_or_else(|| crate::stable_ir_fingerprint(unit)),
        function.raw(),
        unit.functions[function.index()].params.len(),
        region.local_count,
        request.opt_level >= 2,
        request.invalidation_generation,
    );
    let compiled_clif_blocks = std::cell::Cell::new(None);
    let compiled_maximum_pre_regalloc = std::cell::Cell::new(None);
    let compiled_maximum_temporary_cache_entries = std::cell::Cell::new(None);
    let compiled_pre_regalloc_replans = std::cell::Cell::new(0_usize);
    let compiled = compile_managed_native(
        request,
        function,
        function_key,
        if compilation_mode
            == crate::cranelift_lowering::baseline_streaming::NativeCompilationMode::StreamingBaseline
        {
            crate::code_manager::NativeCompileAdmission::request_critical(
                plan.admission_cost_tokens(),
            )
        } else {
            crate::code_manager::NativeCompileAdmission::background_optimizing(
                plan.admission_cost_tokens(),
            )
        },
        compilation_mode.specialization(),
        &import_refs,
        |module, codegen_context, builder_context, name| {
            let mut active_plan = selected_plan.borrow().clone();
            let mut active_fragment_layout = selected_fragment_layout.borrow().clone();
            let helper_address = |symbol: &str| {
                imports
                    .iter()
                    .find_map(|(name, address)| (name == symbol).then_some(*address))
                    .expect("required native helper address must be imported")
            };
            let native_call_helper = if needs_call_trampoline {
                let pointer_type = module.target_config().pointer_type();
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                Some(declare_native_helper(
                    module,
                    &baseline_call_symbol,
                    &signature,
                    helper_address(&baseline_call_symbol),
                )?)
            } else {
                None
            };
            let native_dynamic_code_helper = if needs_dynamic_code {
                let pointer_type = module.target_config().pointer_type();
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                Some(declare_native_helper(
                    module,
                    &native_dynamic_code_symbol,
                    &signature,
                    helper_address(&native_dynamic_code_symbol),
                )?)
            } else {
                None
            };
            let mut native_operations = BaselineNativeOperations::default();
            let pointer_type = module.target_config().pointer_type();
            let mut exact_symbol_query = [None; StableSymbolQueryBuiltin::COUNT];
            for builtin in StableSymbolQueryBuiltin::all() {
                if !needs_exact_symbol_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_symbol_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_pcre = [None; StablePcreBuiltin::COUNT];
            for builtin in StablePcreBuiltin::all() {
                if !needs_exact_pcre[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StablePcreBuiltin::LastError | StablePcreBuiltin::LastErrorMessage => 0,
                    StablePcreBuiltin::Quote => 2,
                    StablePcreBuiltin::Grep => 3,
                    StablePcreBuiltin::Split => 4,
                    StablePcreBuiltin::Match
                    | StablePcreBuiltin::MatchAll
                    | StablePcreBuiltin::Replace
                    | StablePcreBuiltin::Filter => 5,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_pcre[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let preg_callback_plan = if needs_preg_callback {
                let mut signature = module.make_signature();
                for _ in 0..5 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_preg_callback_plan",
                    &signature,
                    helper_address("phrust_native_preg_callback_plan"),
                )?)
            } else {
                None
            };
            let preg_callback_assemble = if needs_preg_callback {
                let mut signature = module.make_signature();
                for _ in 0..3 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_preg_callback_assemble",
                    &signature,
                    helper_address("phrust_native_preg_callback_assemble"),
                )?)
            } else {
                None
            };
            let mut exact_json = [None; StableJsonBuiltin::COUNT];
            for builtin in StableJsonBuiltin::all() {
                if !needs_exact_json[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableJsonBuiltin::LastError | StableJsonBuiltin::LastErrorMessage => 0,
                    StableJsonBuiltin::Encode | StableJsonBuiltin::Validate => 3,
                    StableJsonBuiltin::Decode => 4,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_json[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_format = [None; StableFormatBuiltin::COUNT];
            for builtin in StableFormatBuiltin::all() {
                if !needs_exact_format[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                match builtin {
                    StableFormatBuiltin::Sprintf | StableFormatBuiltin::Printf => {
                        signature.params.push(AbiParam::new(types::I32));
                        signature
                            .params
                            .push(AbiParam::new(module.target_config().pointer_type()));
                    }
                    StableFormatBuiltin::Vsprintf | StableFormatBuiltin::Vprintf => {
                        signature.params.push(AbiParam::new(types::I64));
                        signature.params.push(AbiParam::new(types::I64));
                    }
                    StableFormatBuiltin::NumberFormat => {
                        for _ in 0..4 {
                            signature.params.push(AbiParam::new(types::I64));
                        }
                    }
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_format[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_hash = [None; StableHashBuiltin::COUNT];
            for builtin in StableHashBuiltin::all() {
                if !needs_exact_hash[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableHashBuiltin::Crc32 => 1,
                    StableHashBuiltin::Md5
                    | StableHashBuiltin::Sha1
                    | StableHashBuiltin::HashEquals => 2,
                    StableHashBuiltin::Hash | StableHashBuiltin::HashHmac => 4,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_hash[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_byte_codec = [None; StableByteCodecBuiltin::COUNT];
            for builtin in StableByteCodecBuiltin::all() {
                if !needs_exact_byte_codec[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                match builtin {
                    StableByteCodecBuiltin::Pack => {
                        signature.params.push(AbiParam::new(types::I32));
                        signature
                            .params
                            .push(AbiParam::new(module.target_config().pointer_type()));
                    }
                    StableByteCodecBuiltin::Unpack => {
                        for _ in 0..3 {
                            signature.params.push(AbiParam::new(types::I64));
                        }
                    }
                    StableByteCodecBuiltin::Base64Decode
                    | StableByteCodecBuiltin::AddCSlashes => {
                        for _ in 0..2 {
                            signature.params.push(AbiParam::new(types::I64));
                        }
                    }
                    StableByteCodecBuiltin::Base64Encode
                    | StableByteCodecBuiltin::Bin2Hex
                    | StableByteCodecBuiltin::Hex2Bin
                    | StableByteCodecBuiltin::QuotedPrintableDecode
                    | StableByteCodecBuiltin::UrlEncode
                    | StableByteCodecBuiltin::RawUrlEncode
                    | StableByteCodecBuiltin::UrlDecode
                    | StableByteCodecBuiltin::RawUrlDecode
                    | StableByteCodecBuiltin::UuEncode
                    | StableByteCodecBuiltin::UuDecode
                    | StableByteCodecBuiltin::StripCSlashes
                    | StableByteCodecBuiltin::StripSlashes
                    | StableByteCodecBuiltin::QuoteMeta => {
                        signature.params.push(AbiParam::new(types::I64));
                    }
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_byte_codec[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_string_search_compare =
                [None; StableStringSearchCompareBuiltin::COUNT];
            for builtin in StableStringSearchCompareBuiltin::all() {
                if !needs_exact_string_search_compare[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableStringSearchCompareBuiltin::StrPBrk
                    | StableStringSearchCompareBuiltin::StrNatCmp
                    | StableStringSearchCompareBuiltin::StrNatCaseCmp => 2,
                    StableStringSearchCompareBuiltin::StrStr
                    | StableStringSearchCompareBuiltin::StrIStr
                    | StableStringSearchCompareBuiltin::StrRChr => 3,
                    StableStringSearchCompareBuiltin::SubstrCompare => 5,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_string_search_compare[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_string_rewrite = [None; StableStringRewriteBuiltin::COUNT];
            for builtin in StableStringRewriteBuiltin::all() {
                if !needs_exact_string_rewrite[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableStringRewriteBuiltin::UcWords
                    | StableStringRewriteBuiltin::StripTags
                    | StableStringRewriteBuiltin::StrSplit => 2,
                    StableStringRewriteBuiltin::StrTr
                    | StableStringRewriteBuiltin::VersionCompare => 3,
                    StableStringRewriteBuiltin::StrPad
                    | StableStringRewriteBuiltin::SubstrReplace => 4,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_string_rewrite[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_html_codec = [None; StableHtmlCodecBuiltin::COUNT];
            for builtin in StableHtmlCodecBuiltin::all() {
                if !needs_exact_html_codec[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableHtmlCodecBuiltin::SpecialChars | StableHtmlCodecBuiltin::Entities => 4,
                    StableHtmlCodecBuiltin::EntityDecode => 3,
                    StableHtmlCodecBuiltin::SpecialCharsDecode => 2,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_html_codec[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_url_query = [None; StableUrlQueryBuiltin::COUNT];
            for builtin in StableUrlQueryBuiltin::all() {
                if !needs_exact_url_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableUrlQueryBuiltin::ParseUrl | StableUrlQueryBuiltin::ParseStr => 2,
                    StableUrlQueryBuiltin::HttpBuildQuery => 4,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_url_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_array_aggregate = [None; StableArrayAggregateBuiltin::COUNT];
            for builtin in StableArrayAggregateBuiltin::all() {
                if !needs_exact_array_aggregate[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableArrayAggregateBuiltin::Sum => 1,
                    StableArrayAggregateBuiltin::Count
                    | StableArrayAggregateBuiltin::SizeOf => 2,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_array_aggregate[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_recursive_array = [None; StableRecursiveArrayBuiltin::COUNT];
            for builtin in StableRecursiveArrayBuiltin::all() {
                if !needs_exact_recursive_array[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_recursive_array[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_array_sort = [None; StableArraySortBuiltin::COUNT];
            for builtin in StableArraySortBuiltin::all() {
                if !needs_exact_array_sort[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_array_sort[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let exact_array_multisort = if needs_exact_array_multisort {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(module.target_config().pointer_type()));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_array_multisort",
                    &signature,
                    helper_address("phrust_native_array_multisort"),
                )?)
            } else {
                None
            };
            let mut exact_frame_introspection =
                [None; StableFrameIntrospectionBuiltin::COUNT];
            let mut exact_object_identity = [None; StableObjectIdentityBuiltin::COUNT];
            for builtin in StableObjectIdentityBuiltin::all() {
                if !needs_exact_object_identity[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_object_identity[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_callable_query = [None; StableCallableQueryBuiltin::COUNT];
            for builtin in StableCallableQueryBuiltin::all() {
                if !needs_exact_callable_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                for _ in 0..3 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_callable_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_callback_handler = [None; StableCallbackHandlerBuiltin::COUNT];
            for builtin in StableCallbackHandlerBuiltin::all() {
                if !needs_exact_callback_handler[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                for _ in 0..2 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_callback_handler[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_autoload_callback = [None; StableAutoloadCallbackBuiltin::COUNT];
            for builtin in StableAutoloadCallbackBuiltin::all() {
                if !needs_exact_autoload_callback[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                for _ in 0..3 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_autoload_callback[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let exact_shutdown_callback = if needs_exact_shutdown_callback {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I32));
                signature
                    .params
                    .push(AbiParam::new(module.target_config().pointer_type()));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_register_shutdown_function",
                    &signature,
                    helper_address("phrust_native_register_shutdown_function"),
                )?)
            } else {
                None
            };
            let mut exact_serialization = [None; StableSerializationBuiltin::COUNT];
            for builtin in StableSerializationBuiltin::all() {
                if !needs_exact_serialization[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_serialization[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_tokenizer = [None; StableTokenizerBuiltin::COUNT];
            for builtin in StableTokenizerBuiltin::all() {
                if !needs_exact_tokenizer[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableTokenizerBuiltin::GetAll => 2,
                    StableTokenizerBuiltin::Name => 1,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_tokenizer[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_mbstring = [None; StableMbstringBuiltin::COUNT];
            for builtin in StableMbstringBuiltin::all() {
                if !needs_exact_mbstring[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableMbstringBuiltin::ListEncodings => 0,
                    StableMbstringBuiltin::InternalEncoding
                    | StableMbstringBuiltin::EncodingAliases
                    | StableMbstringBuiltin::SubstituteCharacter => 1,
                    StableMbstringBuiltin::CheckEncoding
                    | StableMbstringBuiltin::Strlen
                    | StableMbstringBuiltin::Strtolower
                    | StableMbstringBuiltin::Strtoupper
                    | StableMbstringBuiltin::Strwidth
                    | StableMbstringBuiltin::Ucfirst
                    | StableMbstringBuiltin::Lcfirst
                    | StableMbstringBuiltin::Ord
                    | StableMbstringBuiltin::Chr
                    | StableMbstringBuiltin::ParseStr => 2,
                    StableMbstringBuiltin::DetectEncoding
                    | StableMbstringBuiltin::ConvertEncoding
                    | StableMbstringBuiltin::SubstrCount
                    | StableMbstringBuiltin::ConvertCase => 3,
                    StableMbstringBuiltin::Stripos
                    | StableMbstringBuiltin::Strpos
                    | StableMbstringBuiltin::Strripos
                    | StableMbstringBuiltin::Strrpos
                    | StableMbstringBuiltin::Substr
                    | StableMbstringBuiltin::Strcut => 4,
                    StableMbstringBuiltin::Strimwidth => 5,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_mbstring[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_bcmath = [None; StableBcmathBuiltin::COUNT];
            for builtin in StableBcmathBuiltin::all() {
                if !needs_exact_bcmath[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableBcmathBuiltin::Scale => 1,
                    StableBcmathBuiltin::Sqrt => 2,
                    StableBcmathBuiltin::Add
                    | StableBcmathBuiltin::Comp
                    | StableBcmathBuiltin::Div
                    | StableBcmathBuiltin::Mod
                    | StableBcmathBuiltin::Mul
                    | StableBcmathBuiltin::Pow
                    | StableBcmathBuiltin::Sub => 3,
                    StableBcmathBuiltin::PowMod => 4,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_bcmath[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_filter = [None; StableFilterBuiltin::COUNT];
            for builtin in StableFilterBuiltin::all() {
                if !needs_exact_filter[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableFilterBuiltin::Input => 4,
                    StableFilterBuiltin::HasVar => 2,
                    StableFilterBuiltin::InputArray
                    | StableFilterBuiltin::VarArray
                    | StableFilterBuiltin::Var => 3,
                    StableFilterBuiltin::List => 0,
                    StableFilterBuiltin::Id => 1,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_filter[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_session = [None; StableSessionBuiltin::COUNT];
            for builtin in StableSessionBuiltin::all() {
                if !needs_exact_session[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableSessionBuiltin::CacheExpire
                    | StableSessionBuiltin::CacheLimiter
                    | StableSessionBuiltin::Decode
                    | StableSessionBuiltin::CreateId
                    | StableSessionBuiltin::Id
                    | StableSessionBuiltin::ModuleName
                    | StableSessionBuiltin::Name
                    | StableSessionBuiltin::RegenerateId
                    | StableSessionBuiltin::SavePath
                    | StableSessionBuiltin::Start => 1,
                    StableSessionBuiltin::SetCookieParams => 5,
                    StableSessionBuiltin::SetSaveHandler => 9,
                    StableSessionBuiltin::Abort
                    | StableSessionBuiltin::Commit
                    | StableSessionBuiltin::Destroy
                    | StableSessionBuiltin::Gc
                    | StableSessionBuiltin::Encode
                    | StableSessionBuiltin::GetCookieParams
                    | StableSessionBuiltin::RegisterShutdown
                    | StableSessionBuiltin::Reset
                    | StableSessionBuiltin::Status
                    | StableSessionBuiltin::Unset
                    | StableSessionBuiltin::WriteClose => 0,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_session[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_object_vars = [None; StableObjectVarsBuiltin::COUNT];
            for builtin in StableObjectVarsBuiltin::all() {
                if !needs_exact_object_vars[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_object_vars[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_class_metadata = [None; StableClassMetadataBuiltin::COUNT];
            for builtin in StableClassMetadataBuiltin::all() {
                if !needs_exact_class_metadata[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_class_metadata[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_class_lineage = [None; StableClassLineageBuiltin::COUNT];
            for builtin in StableClassLineageBuiltin::all() {
                if !needs_exact_class_lineage[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableClassLineageBuiltin::ParentClass => 1,
                    StableClassLineageBuiltin::Implements => 2,
                    StableClassLineageBuiltin::IsSubclassOf | StableClassLineageBuiltin::IsA => 3,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_class_lineage[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_extension_query = [None; StableExtensionQueryBuiltin::COUNT];
            for builtin in StableExtensionQueryBuiltin::all() {
                if !needs_exact_extension_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_extension_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_memory_query = [None; StableMemoryQueryBuiltin::COUNT];
            for builtin in StableMemoryQueryBuiltin::all() {
                if !needs_exact_memory_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_memory_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_gc = [None; StableGcBuiltin::COUNT];
            for builtin in StableGcBuiltin::all() {
                if !needs_exact_gc[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_gc[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_resource_query = [None; StableResourceQueryBuiltin::COUNT];
            for builtin in StableResourceQueryBuiltin::all() {
                if !needs_exact_resource_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_resource_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_error_state = [None; StableErrorStateBuiltin::COUNT];
            for builtin in StableErrorStateBuiltin::all() {
                if !needs_exact_error_state[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_error_state[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let exact_settype = if needs_exact_settype {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_settype",
                    &signature,
                    helper_address("phrust_native_settype"),
                )?)
            } else {
                None
            };
            let mut exact_configuration = [None; StableConfigurationBuiltin::COUNT];
            for builtin in StableConfigurationBuiltin::all() {
                if !needs_exact_configuration[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableConfigurationBuiltin::IncludePath
                    | StableConfigurationBuiltin::TimezoneGet => 0,
                    StableConfigurationBuiltin::IniGet
                    | StableConfigurationBuiltin::CfgVar
                    | StableConfigurationBuiltin::SetIncludePath
                    | StableConfigurationBuiltin::TimezoneSet => 1,
                    StableConfigurationBuiltin::IniGetAll
                    | StableConfigurationBuiltin::IniSet => 2,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_configuration[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_http_response = [None; StableHttpResponseBuiltin::COUNT];
            for builtin in StableHttpResponseBuiltin::all() {
                if !needs_exact_http_response[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableHttpResponseBuiltin::HeadersList
                    | StableHttpResponseBuiltin::HeadersSent => 0,
                    StableHttpResponseBuiltin::HeaderRemove
                    | StableHttpResponseBuiltin::ResponseCode => 1,
                    StableHttpResponseBuiltin::Header => 3,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_http_response[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_cookie = [None; StableCookieBuiltin::COUNT];
            for builtin in StableCookieBuiltin::all() {
                if !needs_exact_cookie[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                for _ in 0..7 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_cookie[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_clock = [None; StableClockBuiltin::COUNT];
            for builtin in StableClockBuiltin::all() {
                if !needs_exact_clock[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableClockBuiltin::Time => 0,
                    StableClockBuiltin::Microtime | StableClockBuiltin::Hrtime => 1,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_clock[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_date = [None; StableDateBuiltin::COUNT];
            for builtin in StableDateBuiltin::all() {
                if !needs_exact_date[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableDateBuiltin::TimezoneIdentifiers => 0,
                    StableDateBuiltin::Date
                    | StableDateBuiltin::Gmdate
                    | StableDateBuiltin::Strtotime => 2,
                    StableDateBuiltin::Checkdate => 3,
                    StableDateBuiltin::Mktime | StableDateBuiltin::Gmmktime => 6,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_date[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_random = [None; StableRandomBuiltin::COUNT];
            for builtin in StableRandomBuiltin::all() {
                if !needs_exact_random[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableRandomBuiltin::GetRandMax | StableRandomBuiltin::MtGetRandMax => 0,
                    StableRandomBuiltin::RandomBytes | StableRandomBuiltin::Shuffle => 1,
                    StableRandomBuiltin::RandomInt
                    | StableRandomBuiltin::Rand
                    | StableRandomBuiltin::MtRand
                    | StableRandomBuiltin::ArrayRand => 2,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_random[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_request_query = [None; StableRequestQueryBuiltin::COUNT];
            for builtin in StableRequestQueryBuiltin::all() {
                if !needs_exact_request_query[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableRequestQueryBuiltin::Environment
                    | StableRequestQueryBuiltin::Uname
                    | StableRequestQueryBuiltin::ChangeDirectory
                    | StableRequestQueryBuiltin::Umask => 1,
                    StableRequestQueryBuiltin::ClearStatCache => 2,
                    StableRequestQueryBuiltin::TempDir
                    | StableRequestQueryBuiltin::CurrentDirectory
                    | StableRequestQueryBuiltin::SapiName
                    | StableRequestQueryBuiltin::CurrentUser
                    | StableRequestQueryBuiltin::IncludedFiles => 0,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_request_query[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_declaration_inventory =
                [None; StableDeclarationInventoryBuiltin::COUNT];
            for builtin in StableDeclarationInventoryBuiltin::all() {
                if !needs_exact_declaration_inventory[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_declaration_inventory[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let exact_constant_inventory = if needs_exact_constant_inventory {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_get_defined_constants",
                    &signature,
                    helper_address("phrust_native_get_defined_constants"),
                )?)
            } else {
                None
            };
            let exact_compact = if needs_exact_compact {
                let mut signature = module.make_signature();
                for _ in 0..5 {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_compact",
                    &signature,
                    helper_address("phrust_native_compact"),
                )?)
            } else {
                None
            };
            for builtin in StableFrameIntrospectionBuiltin::all() {
                if !needs_exact_frame_introspection[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableFrameIntrospectionBuiltin::NumArgs
                    | StableFrameIntrospectionBuiltin::GetArgs => 0,
                    StableFrameIntrospectionBuiltin::GetArg => 1,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_frame_introspection[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_base_conversion = [None; StableBaseConversionBuiltin::COUNT];
            for builtin in StableBaseConversionBuiltin::all() {
                if !needs_exact_base_conversion[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableBaseConversionBuiltin::BaseConvert => 3,
                    StableBaseConversionBuiltin::BinDec
                    | StableBaseConversionBuiltin::DecBin
                    | StableBaseConversionBuiltin::DecHex
                    | StableBaseConversionBuiltin::DecOct
                    | StableBaseConversionBuiltin::HexDec
                    | StableBaseConversionBuiltin::OctDec => 1,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_base_conversion[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let exact_intval_base = if needs_exact_intval_base {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_intval_base",
                    &signature,
                    helper_address("phrust_native_intval_base"),
                )?)
            } else {
                None
            };
            let mut exact_network_address = [None; StableNetworkAddressBuiltin::COUNT];
            for builtin in StableNetworkAddressBuiltin::all() {
                if !needs_exact_network_address[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_network_address[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_compression_codec = [None; StableCompressionCodecBuiltin::COUNT];
            for builtin in StableCompressionCodecBuiltin::all() {
                if !needs_exact_compression_codec[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StableCompressionCodecBuiltin::GzDecode
                    | StableCompressionCodecBuiltin::GzUncompress
                    | StableCompressionCodecBuiltin::GzInflate
                    | StableCompressionCodecBuiltin::ZlibDecode => 2,
                    StableCompressionCodecBuiltin::GzEncode
                    | StableCompressionCodecBuiltin::GzCompress
                    | StableCompressionCodecBuiltin::GzDeflate
                    | StableCompressionCodecBuiltin::ZlibEncode => 3,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_compression_codec[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_path = [None; StablePathBuiltin::COUNT];
            for builtin in StablePathBuiltin::all() {
                if !needs_exact_path[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                let arity = match builtin {
                    StablePathBuiltin::Realpath
                    | StablePathBuiltin::FileExists
                    | StablePathBuiltin::IsFile
                    | StablePathBuiltin::IsDir
                    | StablePathBuiltin::IsReadable
                    | StablePathBuiltin::IsWritable
                    | StablePathBuiltin::IsLink
                    | StablePathBuiltin::FilePerms
                    | StablePathBuiltin::FileOwner
                    | StablePathBuiltin::FileGroup
                    | StablePathBuiltin::FileType
                    | StablePathBuiltin::DiskFreeSpace
                    | StablePathBuiltin::DiskTotalSpace
                    | StablePathBuiltin::Stat
                    | StablePathBuiltin::Lstat
                    | StablePathBuiltin::Filesize
                    | StablePathBuiltin::Filemtime
                    | StablePathBuiltin::Unlink
                    | StablePathBuiltin::Mkdir
                    | StablePathBuiltin::Rmdir
                    | StablePathBuiltin::Touch
                    | StablePathBuiltin::Fclose
                    | StablePathBuiltin::Fgetc
                    | StablePathBuiltin::Feof
                    | StablePathBuiltin::Fflush
                    | StablePathBuiltin::Ftell
                    | StablePathBuiltin::Rewind
                    | StablePathBuiltin::OpenDir
                    | StablePathBuiltin::ReadDir
                    | StablePathBuiltin::RewindDir
                    | StablePathBuiltin::CloseDir
                    | StablePathBuiltin::StreamGetMetaData
                    | StablePathBuiltin::StreamIsLocal
                    | StablePathBuiltin::StreamResolveIncludePath
                    | StablePathBuiltin::StreamContextCreate
                    | StablePathBuiltin::StreamContextGetDefault
                    | StablePathBuiltin::StreamContextGetOptions
                    | StablePathBuiltin::StreamContextSetDefault
                    | StablePathBuiltin::StreamFilterRemove
                    | StablePathBuiltin::StreamIsAtty
                    | StablePathBuiltin::Readfile
                    | StablePathBuiltin::IsUploadedFile => 1,
                    StablePathBuiltin::StreamGetWrappers | StablePathBuiltin::Tmpfile => 0,
                    StablePathBuiltin::Basename
                    | StablePathBuiltin::Dirname
                    | StablePathBuiltin::Pathinfo
                    | StablePathBuiltin::Glob
                    | StablePathBuiltin::Rename
                    | StablePathBuiltin::Fopen
                    | StablePathBuiltin::Fread
                    | StablePathBuiltin::Fgets
                    | StablePathBuiltin::Ftruncate
                    | StablePathBuiltin::ScanDir
                    | StablePathBuiltin::StreamContextSetOptions
                    | StablePathBuiltin::Chmod
                    | StablePathBuiltin::Symlink
                    | StablePathBuiltin::Tempnam => 2,
                    StablePathBuiltin::Fwrite
                    | StablePathBuiltin::Fseek
                    | StablePathBuiltin::File
                    | StablePathBuiltin::StreamGetContents
                    | StablePathBuiltin::StreamSetTimeout => 3,
                    StablePathBuiltin::StreamCopyToStream
                    | StablePathBuiltin::FilePutContents
                    | StablePathBuiltin::StreamContextSetOption
                    | StablePathBuiltin::StreamFilterAppend
                    | StablePathBuiltin::StreamFilterPrepend => 4,
                    StablePathBuiltin::FileGetContents => 5,
                };
                for _ in 0..arity {
                    signature.params.push(AbiParam::new(types::I64));
                }
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_path[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_output_buffer = [None; StableOutputBufferBuiltin::COUNT];
            for builtin in StableOutputBufferBuiltin::all() {
                if !needs_exact_output_buffer[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                exact_output_buffer[builtin.index()] = Some(declare_native_helper(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let mut exact_binary = [None; NATIVE_EXACT_BINARY_COUNT];
            for operation in NATIVE_EXACT_BINARY_OPERATIONS {
                let index = native_exact_binary_index(operation);
                exact_binary[index] = declare_native_control_handler(
                    module,
                    needs_exact_binary[index],
                    native_exact_binary_symbol(operation),
                    2,
                    || helper_address(native_exact_binary_symbol(operation)),
                )?;
            }
            let mut exact_unary = [None; NATIVE_EXACT_UNARY_COUNT];
            for operation in NATIVE_EXACT_UNARY_OPERATIONS {
                let index = native_exact_unary_index(operation);
                exact_unary[index] = declare_native_control_handler(
                    module,
                    needs_exact_unary[index],
                    native_exact_unary_symbol(operation),
                    1,
                    || helper_address(native_exact_unary_symbol(operation)),
                )?;
            }
            let mut exact_compare = [None; NATIVE_EXACT_COMPARE_COUNT];
            for operation in NATIVE_EXACT_COMPARE_OPERATIONS {
                let index = native_exact_compare_index(operation);
                exact_compare[index] = declare_native_control_handler(
                    module,
                    needs_exact_compare[index],
                    native_exact_compare_symbol(operation),
                    2,
                    || helper_address(native_exact_compare_symbol(operation)),
                )?;
            }
            let echo_bytes = if needs_direct_echo {
                let mut bytes_signature = module.make_signature();
                bytes_signature.params.push(AbiParam::new(pointer_type));
                bytes_signature.params.push(AbiParam::new(types::I64));
                let bytes = declare_native_helper(
                    module,
                    "phrust_native_echo_bytes",
                    &bytes_signature,
                    helper_address("phrust_native_echo_bytes"),
                )?;
                Some(bytes)
            } else {
                None
            };
            let float_to_string = if needs_float_to_string {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::F64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_float_to_string",
                    &signature,
                    helper_address("phrust_native_float_to_string"),
                )?)
            } else {
                None
            };
            let numeric_string = if needs_numeric_string {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_pure_handler(
                    module,
                    "phrust_native_numeric_string",
                    &signature,
                    helper_address("phrust_native_numeric_string"),
                )?)
            } else {
                None
            };
            let fmod_f64 = if needs_fmod_f64 {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::F64));
                signature.params.push(AbiParam::new(types::F64));
                signature.returns.push(AbiParam::new(types::F64));
                Some(declare_native_pure_handler(
                    module,
                    "phrust_native_fmod_f64",
                    &signature,
                    helper_address("phrust_native_fmod_f64"),
                )?)
            } else {
                None
            };
            let round_f64 = if needs_round_f64 {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::F64));
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::F64));
                Some(declare_native_pure_handler(
                    module,
                    "phrust_native_round_f64",
                    &signature,
                    helper_address("phrust_native_round_f64"),
                )?)
            } else {
                None
            };
            let mut pure_math = [None; StablePureMathBuiltin::COUNT];
            for builtin in StablePureMathBuiltin::all() {
                if !needs_exact_pure_math[builtin.index()] {
                    continue;
                }
                let mut signature = module.make_signature();
                for _ in 0..if builtin.accepts_arity(2) { 2 } else { 1 } {
                    signature.params.push(AbiParam::new(types::F64));
                }
                signature.returns.push(AbiParam::new(types::F64));
                pure_math[builtin.index()] = Some(declare_native_pure_handler(
                    module,
                    builtin.symbol(),
                    &signature,
                    helper_address(builtin.symbol()),
                )?);
            }
            let array_cast = declare_native_control_handler(
                module,
                needs_array_cast,
                "phrust_native_array_cast",
                1,
                || helper_address("phrust_native_array_cast"),
            )?;
            let int_cast = declare_native_control_handler(
                module,
                needs_int_cast,
                "phrust_native_int_cast",
                1,
                || helper_address("phrust_native_int_cast"),
            )?;
            let float_cast = declare_native_control_handler(
                module,
                needs_float_cast,
                "phrust_native_float_cast",
                1,
                || helper_address("phrust_native_float_cast"),
            )?;
            let string_cast = declare_native_control_handler(
                module,
                needs_string_cast,
                "phrust_native_string_cast",
                1,
                || helper_address("phrust_native_string_cast"),
            )?;
            let callback_return_string = declare_native_control_handler(
                module,
                needs_callback_return_string,
                "phrust_native_callback_return_string",
                1,
                || helper_address("phrust_native_callback_return_string"),
            )?;
            let object_cast = declare_native_control_handler(
                module,
                needs_object_cast,
                "phrust_native_object_cast",
                1,
                || helper_address("phrust_native_object_cast"),
            )?;
            let object_class_name = if needs_object_class_name {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_object_class_name",
                    &signature,
                    helper_address("phrust_native_object_class_name"),
                )?)
            } else {
                None
            };
            let acquire_callable = declare_native_control_handler(
                module,
                needs_acquire_callable,
                "phrust_native_acquire_callable",
                1,
                || helper_address("phrust_native_acquire_callable"),
            )?;
            let resolve_callable = declare_native_control_handler(
                module,
                needs_resolve_callable,
                "phrust_native_resolve_callable",
                2,
                || helper_address("phrust_native_resolve_callable"),
            )?;
            let dynamic_instanceof = declare_native_control_handler(
                module,
                needs_dynamic_instanceof,
                "phrust_native_dynamic_instanceof",
                2,
                || helper_address("phrust_native_dynamic_instanceof"),
            )?;
            let prepared_object_new = if needs_prepared_object_new {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_prepared_object_new",
                    &signature,
                    helper_address("phrust_native_prepared_object_new"),
                )?)
            } else {
                None
            };
            let prepared_exception_new = if needs_prepared_exception_new {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_prepared_exception_new",
                    &signature,
                    helper_address("phrust_native_prepared_exception_new"),
                )?)
            } else {
                None
            };
            let prepared_closure_new = if needs_prepared_closure_new {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_prepared_closure_new",
                    &signature,
                    helper_address("phrust_native_prepared_closure_new"),
                )?)
            } else {
                None
            };
            let plain_object_clone = if needs_plain_object_clone {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I64));
                Some(declare_native_helper(
                    module,
                    "phrust_native_plain_object_clone",
                    &signature,
                    helper_address("phrust_native_plain_object_clone"),
                )?)
            } else {
                None
            };
            let dynamic_property_slot = declare_native_control_handler(
                module,
                needs_dynamic_property_slot,
                "phrust_native_dynamic_property_slot",
                2,
                || helper_address("phrust_native_dynamic_property_slot"),
            )?;
            let dynamic_property_test_slot = declare_native_control_handler(
                module,
                needs_dynamic_property_test_slot,
                "phrust_native_dynamic_property_test_slot",
                2,
                || helper_address("phrust_native_dynamic_property_test_slot"),
            )?;
            if needs_baseline_builtin_dispatch {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.builtin_dispatch = Some(declare_native_helper(
                    module,
                    &native_builtin_dispatch_symbol,
                    &signature,
                    helper_address(&native_builtin_dispatch_symbol),
                )?);
            }
            if needs_semantic_dispatch {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.semantic_dispatch = Some(declare_native_helper(
                    module,
                    &baseline_semantic_dispatch_symbol,
                    &signature,
                    helper_address(&baseline_semantic_dispatch_symbol),
                )?);
            }
            if needs_function_resolver {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.function_resolve = Some(declare_native_helper(
                    module,
                    &native_function_resolve_symbol,
                    &signature,
                    helper_address(&native_function_resolve_symbol),
                )?);
            }
            if needs_frame_arena {
                let mut alloc_signature = module.make_signature();
                alloc_signature.params.push(AbiParam::new(types::I64));
                alloc_signature.params.push(AbiParam::new(types::I64));
                alloc_signature.params.push(AbiParam::new(types::I64));
                alloc_signature.returns.push(AbiParam::new(pointer_type));
                native_operations.frame_alloc = Some(declare_native_helper(
                    module,
                    "phrust_native_frame_alloc",
                    &alloc_signature,
                    helper_address("phrust_native_frame_alloc"),
                )?);
                let mut release_signature = module.make_signature();
                release_signature.params.push(AbiParam::new(types::I64));
                release_signature.params.push(AbiParam::new(pointer_type));
                release_signature.returns.push(AbiParam::new(types::I32));
                native_operations.frame_release = Some(declare_native_helper(
                    module,
                    "phrust_native_frame_release",
                    &release_signature,
                    helper_address("phrust_native_frame_release"),
                )?);
            }
            if needs_unary {
                native_operations.unary = Some(declare_baseline_value_operation(
                    module,
                    "phrust_baseline_native_unary",
                    1,
                    helper_address("phrust_baseline_native_unary"),
                )?);
            }
            if needs_baseline_binary {
                native_operations.baseline_binary = Some(declare_baseline_value_operation(
                    module,
                    "phrust_baseline_native_binary",
                    4,
                    helper_address("phrust_baseline_native_binary"),
                )?);
            }
            if needs_compare {
                native_operations.compare = Some(declare_baseline_value_operation(
                    module,
                    "phrust_baseline_native_compare",
                    2,
                    helper_address("phrust_baseline_native_compare"),
                )?);
            }
            if needs_cast {
                native_operations.cast = Some(declare_baseline_value_operation(
                    module,
                    "phrust_baseline_native_cast",
                    1,
                    helper_address("phrust_baseline_native_cast"),
                )?);
            }
            if needs_echo {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.echo = Some(declare_native_helper(
                    module,
                    "phrust_native_echo",
                    &signature,
                    helper_address("phrust_native_echo"),
                )?);
            }
            if needs_local_fetch {
                native_operations.local_fetch = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_local_fetch",
                    5,
                    helper_address("phrust_native_local_fetch"),
                )?);
            }
            if needs_local_store {
                native_operations.local_store = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_local_store",
                    4,
                    helper_address("phrust_native_local_store"),
                )?);
            }
            if needs_value_release {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.value_release = Some(declare_native_helper(
                    module,
                    "phrust_native_value_release",
                    &signature,
                    helper_address("phrust_native_value_release"),
                )?);
            }
            if needs_reference_bind {
                native_operations.reference_bind = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_reference_bind",
                    3,
                    helper_address("phrust_native_reference_bind"),
                )?);
            }
            if needs_argument_check {
                native_operations.argument_check = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_argument_check",
                    5,
                    helper_address("phrust_native_argument_check"),
                )?);
            }
            if needs_return_check {
                native_operations.return_check = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_return_check",
                    2,
                    helper_address("phrust_native_return_check"),
                )?);
            }
            if needs_exception_new {
                native_operations.exception_new = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_exception_new",
                    3,
                    helper_address("phrust_native_exception_new"),
                )?);
            }
            if needs_array_new {
                native_operations.array_new = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_new",
                    0,
                    helper_address("phrust_native_array_new"),
                )?);
            }
            if needs_object_new {
                native_operations.object_new = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_object_new",
                    0,
                    helper_address("phrust_native_object_new"),
                )?);
            }
            if needs_property_fetch {
                native_operations.property_fetch = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_property_fetch",
                    3,
                    helper_address("phrust_native_property_fetch"),
                )?);
            }
            if needs_property_assign {
                native_operations.property_assign = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_property_assign",
                    4,
                    helper_address("phrust_native_property_assign"),
                )?);
            }
            if needs_object_clone {
                native_operations.object_clone = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_object_clone",
                    1,
                    helper_address("phrust_native_object_clone"),
                )?);
            }
            if needs_object_clone_with {
                native_operations.object_clone_with = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_object_clone_with",
                    2,
                    helper_address("phrust_native_object_clone_with"),
                )?);
            }
            if needs_array_insert {
                native_operations.array_insert = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_insert",
                    3,
                    helper_address("phrust_native_array_insert"),
                )?);
                native_operations.array_insert_local = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_insert_local",
                    3,
                    helper_address("phrust_native_array_insert_local"),
                )?);
            }
            if needs_array_fetch {
                native_operations.array_fetch = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_fetch",
                    2,
                    helper_address("phrust_native_array_fetch"),
                )?);
            }
            if needs_array_unset {
                native_operations.array_unset = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_unset",
                    2,
                    helper_address("phrust_native_array_unset"),
                )?);
            }
            if needs_array_spread {
                native_operations.array_spread = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_array_spread",
                    2,
                    helper_address("phrust_native_array_spread"),
                )?);
            }
            if needs_foreach_init {
                native_operations.foreach_init = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_foreach_init",
                    3,
                    helper_address("phrust_native_foreach_init"),
                )?);
            }
            if needs_foreach_next {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.foreach_next = Some(declare_native_helper(
                    module,
                    "phrust_native_foreach_next",
                    &signature,
                    helper_address("phrust_native_foreach_next"),
                )?);
            }
            if needs_foreach_cleanup {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.foreach_cleanup = Some(declare_native_helper(
                    module,
                    "phrust_native_foreach_cleanup",
                    &signature,
                    helper_address("phrust_native_foreach_cleanup"),
                )?);
            }
            if needs_constant_fetch {
                native_operations.constant_fetch = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_constant_fetch",
                    2,
                    helper_address("phrust_native_constant_fetch"),
                )?);
            }
            if needs_truthy {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                signature.params.push(AbiParam::new(pointer_type));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.truthy = Some(declare_native_helper(
                    module,
                    "phrust_native_truthy",
                    &signature,
                    helper_address("phrust_native_truthy"),
                )?);
            }
            if needs_type_predicate {
                native_operations.type_predicate = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_type_predicate",
                    1,
                    helper_address("phrust_native_type_predicate"),
                )?);
            }
            if needs_stable_length {
                native_operations.stable_length = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_stable_length",
                    3,
                    helper_address("phrust_native_stable_length"),
                )?);
            }
            if needs_string_predicate {
                native_operations.string_predicate = Some(declare_baseline_value_operation(
                    module,
                    "phrust_native_string_predicate",
                    2,
                    helper_address("phrust_native_string_predicate"),
                )?);
            }
            if needs_runtime_fatal {
                let mut signature = module.make_signature();
                signature.params.push(AbiParam::new(types::I32));
                signature.params.push(AbiParam::new(types::I32));
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.runtime_fatal = Some(declare_native_helper(
                    module,
                    "phrust_native_runtime_fatal",
                    &signature,
                    helper_address("phrust_native_runtime_fatal"),
                )?);
            }
            if needs_execution_poll {
                let mut signature = module.make_signature();
                signature.returns.push(AbiParam::new(types::I32));
                native_operations.execution_poll = Some(declare_native_helper(
                    module,
                    "phrust_native_execution_poll",
                    &signature,
                    helper_address("phrust_native_execution_poll"),
                )?);
            }
            let mut functions = BTreeMap::new();
            for candidate in regions.values() {
                let symbol = if candidate.function == function {
                    name.to_owned()
                } else {
                    format!("{name}.callee.{}", candidate.function.raw())
                };
                let signature = region_graph_signature(module, candidate)?;
                let func_id = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare executable region {symbol}: {error}"),
                        )
                    })?;
                functions.insert(candidate.function, func_id);
            }
            let synthetic_base = u32::try_from(unit.functions.len()).map_err(|_| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                    "source unit function count does not fit the fragment symbol space",
                )
            })?;
            let mut next_synthetic = synthetic_base;
            let tier_operations = if baseline_helper_imports {
                let value_release_commit_symbol = FunctionId::new(next_synthetic);
                next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                        "native baseline value-release symbol id overflowed",
                    )
                })?;
                let symbol = format!("{name}.native.value_release_commit");
                let signature = direct_value_release_signature(module);
                let value_release_commit = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare {symbol}: {error}"),
                        )
                    })?;
                functions.insert(value_release_commit_symbol, value_release_commit);
                NativeTierOperations::Baseline {
                    call: native_call_helper,
                    dynamic_code: native_dynamic_code_helper,
                    operations: native_operations,
                    value_release_commit,
                    value_release_commit_symbol,
                }
            } else {
                let array_ensure_unique_symbol = FunctionId::new(next_synthetic);
                next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                        "native optimizing-operation symbol id overflowed",
                    )
                })?;
                let symbol = format!("{name}.native.array_ensure_unique");
                let signature = direct_array_ensure_unique_signature(module);
                let array_ensure_unique = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare {symbol}: {error}"),
                        )
                    })?;
                functions.insert(array_ensure_unique_symbol, array_ensure_unique);
                let array_child_entry_symbol = FunctionId::new(next_synthetic);
                next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                        "native optimizing-operation symbol id overflowed",
                    )
                })?;
                let symbol = format!("{name}.native.array_child_entry");
                let signature = direct_array_child_entry_signature(module);
                let array_child_entry = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare {symbol}: {error}"),
                        )
                    })?;
                functions.insert(array_child_entry_symbol, array_child_entry);
                let value_release_validate_symbol = FunctionId::new(next_synthetic);
                next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                        "native value-release validator symbol id overflowed",
                    )
                })?;
                let symbol = format!("{name}.native.value_release_validate");
                let signature = direct_value_release_signature(module);
                let value_release_validate = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare {symbol}: {error}"),
                        )
                    })?;
                functions.insert(value_release_validate_symbol, value_release_validate);
                let value_release_commit_symbol = FunctionId::new(next_synthetic);
                next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                        "native value-release commit symbol id overflowed",
                    )
                })?;
                let symbol = format!("{name}.native.value_release_commit");
                let signature = direct_value_release_signature(module);
                let value_release_commit = module
                    .declare_function(&symbol, Linkage::Local, &signature)
                    .map_err(|error| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_DECLARE",
                            format!("failed to declare {symbol}: {error}"),
                        )
                    })?;
                functions.insert(value_release_commit_symbol, value_release_commit);
                NativeTierOperations::Optimizing {
                    operations: NativeOptimizingOperations {
                        execution_poll: native_operations.execution_poll,
                        exact_binary,
                        exact_unary,
                        exact_compare,
                        echo_bytes,
                        float_to_string,
                        numeric_string,
                        fmod_f64,
                        round_f64,
                        pure_math,
                        array_cast,
                        int_cast,
                        float_cast,
                        string_cast,
                        callback_return_string,
                        object_cast,
                        object_class_name,
                        acquire_callable,
                        resolve_callable,
                        dynamic_instanceof,
                        prepared_object_new,
                        prepared_exception_new,
                        prepared_closure_new,
                        plain_object_clone,
                        dynamic_property_slot,
                        dynamic_property_test_slot,
                        exact_symbol_query,
                        exact_pcre,
                        preg_callback_plan,
                        preg_callback_assemble,
                        exact_json,
                        exact_format,
                        exact_hash,
                        exact_byte_codec,
                        exact_string_search_compare,
                        exact_string_rewrite,
                        exact_html_codec,
                        exact_url_query,
                        exact_array_aggregate,
                        exact_recursive_array,
                        exact_array_sort,
                        exact_array_multisort,
                        exact_object_identity,
                        exact_callable_query,
                        exact_callback_handler,
                        exact_autoload_callback,
                        exact_shutdown_callback,
                        exact_serialization,
                        exact_tokenizer,
                        exact_mbstring,
                        exact_bcmath,
                        exact_filter,
                        exact_session,
                        exact_object_vars,
                        exact_class_metadata,
                        exact_class_lineage,
                        exact_extension_query,
                        exact_memory_query,
                        exact_gc,
                        exact_resource_query,
                        exact_error_state,
                        exact_settype,
                        exact_configuration,
                        exact_http_response,
                        exact_cookie,
                        exact_clock,
                        exact_date,
                        exact_random,
                        exact_request_query,
                        exact_declaration_inventory,
                        exact_constant_inventory,
                        exact_compact,
                        exact_frame_introspection,
                        exact_base_conversion,
                        exact_intval_base,
                        exact_network_address,
                        exact_compression_codec,
                        exact_path,
                        exact_output_buffer,
                        array_ensure_unique,
                        array_ensure_unique_symbol,
                        array_child_entry,
                        array_child_entry_symbol,
                        value_release_validate,
                        value_release_validate_symbol,
                        value_release_commit,
                        value_release_commit_symbol,
                    },
                }
            };
            let (mut fragment_functions, mut fragment_symbols) =
                declare_fragment_functions(
                    module,
                    name,
                    region,
                    active_fragment_layout.as_ref(),
                    0,
                    &mut next_synthetic,
                    &mut functions,
                )?;
            let inline_constants = collect_bounded_inline_values(unit, &regions);
            let tail_forwards = regions
                .values()
                .flat_map(|candidate| {
                    candidate.blocks.iter().filter_map(|block| {
                        let (continuation, target) =
                            bounded_tail_forward_target(candidate, block, &regions)?;
                        (!trampoline_functions.contains(&target))
                            .then_some(((candidate.function, continuation), target))
                    })
                })
                .collect::<BTreeMap<_, _>>();

            let mut code_bytes = 0_u64;
            let mut clif_blocks = 0_usize;
            let mut maximum_pre_regalloc = PreRegallocMetrics::default();
            let mut maximum_temporary_cache_entries = 0_usize;
            let mut native_pc_ranges = Vec::new();
            let mut relocatable_bytes = Vec::new();
            let mut relocatable_functions = Vec::new();
            let mut relocatable_relocations = Vec::new();
            let mut emitted_production_lowering = Vec::new();
            let mut function_code_metrics = BTreeMap::new();
            // Keep parameter metadata for every function in the source unit,
            // including callees deliberately omitted from a bounded local
            // call graph. The typed trampoline still needs the declared
            // by-reference contract for those functions; otherwise ordinary
            // lvalue arguments (such as `$this->property`) are conservatively
            // rebound as references before dispatch.
            let mut function_params = unit
                .functions
                .iter()
                .enumerate()
                .filter_map(|(index, function)| {
                    let function_id = u32::try_from(index).ok().map(FunctionId::new)?;
                    let native_arity =
                        crate::region_ir::native_function_parameter_locals(unit, function_id)?
                            .len();
                    Some((
                        function_id,
                        NativeFunctionMetadata {
                            name: function.name.clone(),
                            params: function.params.clone(),
                            requires_trampoline: ir_function_requires_trampoline(function),
                            native_arity,
                            reference_only_trampoline: (function
                                .params
                                .iter()
                                .any(|parameter| parameter.by_ref)
                                || function.returns_by_ref)
                                && !ir_function_requires_non_reference_trampoline(function),
                            returns_by_reference: function.returns_by_ref,
                            has_exception_handlers: ir_function_has_exception_handler(function),
                        },
                    ))
                })
                .collect::<BTreeMap<_, _>>();
            function_params.extend(regions.iter().map(|(function, region)| {
                let ir_function = &unit.functions[function.index()];
                (
                    *function,
                    NativeFunctionMetadata {
                        name: ir_function.name.clone(),
                        params: region.params.clone(),
                        requires_trampoline: trampoline_functions.contains(function),
                        native_arity: region.arity(),
                        reference_only_trampoline: (ir_function
                            .params
                            .iter()
                            .any(|parameter| parameter.by_ref)
                            || ir_function.returns_by_ref)
                            && !ir_function_requires_non_reference_trampoline(ir_function),
                        returns_by_reference: ir_function.returns_by_ref,
                        has_exception_handlers: !region.exception_regions.is_empty(),
                    },
                )
            }));
            let mut preflighted_whole = None;
            let mut preflighted_fragments = BTreeMap::<u32, DefinedRegionFunction>::new();
            // A planner-admitted whole optimizing function still needs exact
            // CLIF preflight. Direct calls, ownership, and guards can expand
            // one Region instruction into enough backend state to exceed the
            // whole-function ceiling even when the source estimate is
            // bounded. Keep the ordinary whole-function representation when
            // its exact form fits; otherwise enter the same deterministic
            // fragment refinement used below.
            if active_fragment_layout.is_none() {
                let register_liveness = NativeRegisterLiveness::analyze(region);
                let compiler =
                    crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
                        region.compile_metadata.tier,
                    );
                let preflight = compiler.compile_fragment(&mut |mode| {
                    define_region_graph_function(
                        module,
                        codegen_context,
                        builder_context,
                        region,
                        &unit.constants,
                        &value_flows[&region.function],
                        functions[&region.function],
                        &functions,
                        &inline_constants,
                        &tail_forwards,
                        &function_params,
                        &request.external_function_signatures,
                        tier_operations,
                        &register_liveness,
                        None,
                        runtime_unit_identity,
                        mode,
                        false,
                        true,
                    )
                });
                match preflight {
                    Ok(defined)
                        if defined
                            .pre_regalloc
                            .exceeds_replan_margin(region.compile_metadata.tier) =>
                    {
                        active_plan = NativeCompilePlan::for_bounded_fragments(region);
                        active_fragment_layout =
                            Some(NativeFunctionFragmentLayout::for_plan(region, &active_plan)?);
                        compiled_pre_regalloc_replans
                            .set(compiled_pre_regalloc_replans.get().saturating_add(1));
                        (fragment_functions, fragment_symbols) = declare_fragment_functions(
                            module,
                            name,
                            region,
                            active_fragment_layout.as_ref(),
                            0,
                            &mut next_synthetic,
                            &mut functions,
                        )?;
                    }
                    Ok(defined) => preflighted_whole = Some(defined),
                    Err(error) if error.code == "JIT_CRANELIFT_PRE_REGALLOC_BUDGET" => {
                        active_plan = NativeCompilePlan::for_bounded_fragments(region);
                        active_fragment_layout =
                            Some(NativeFunctionFragmentLayout::for_plan(region, &active_plan)?);
                        compiled_pre_regalloc_replans
                            .set(compiled_pre_regalloc_replans.get().saturating_add(1));
                        (fragment_functions, fragment_symbols) = declare_fragment_functions(
                            module,
                            name,
                            region,
                            active_fragment_layout.as_ref(),
                            0,
                            &mut next_synthetic,
                            &mut functions,
                        )?;
                    }
                    Err(error) => return Err(error),
                }
            }
            // Fragmented optimizing functions and streaming baseline
            // functions use exact preflight for every fragment. The cheap
            // planner estimate intentionally cannot account for the full
            // live-state fanout of direct guards; without this pass, one
            // underestimated fragment rejects the complete artifact only
            // after all preceding fragments have already been compiled.
            if active_fragment_layout.is_some() {
                for replan_attempt in 0..=MAX_PRE_REGALLOC_REPLAN_ATTEMPTS {
                    let mut offending_fragments = Vec::new();
                    let mut round_preflighted = BTreeMap::new();
                    if let Some(layout) = active_fragment_layout.as_ref() {
                        for fragment in &layout.fragments {
                            let compiler = crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
                                region.compile_metadata.tier,
                            );
                            let preflight = compiler.compile_fragment(&mut |mode| {
                                let func_id = if layout.fragments.len() == 1 {
                                    functions[&region.function]
                                } else {
                                    fragment_functions[&fragment.id]
                                };
                                define_region_graph_function(
                                    module,
                                    codegen_context,
                                    builder_context,
                                    region,
                                    &unit.constants,
                                    &value_flows[&region.function],
                                    func_id,
                                    &functions,
                                    &inline_constants,
                                    &tail_forwards,
                                    &function_params,
                                    &request.external_function_signatures,
                                    tier_operations,
                                    &layout.register_liveness,
                                    Some(NativeFragmentDefinition {
                                        layout,
                                        fragment,
                                        functions: &fragment_functions,
                                    }),
                                    runtime_unit_identity,
                                    mode,
                                    layout.fragments.len() == 1,
                                    true,
                                )
                            });
                            match preflight {
                                Ok(defined)
                                    if defined
                                        .pre_regalloc
                                        .exceeds_replan_margin(region.compile_metadata.tier) =>
                                {
                                    offending_fragments.push((
                                        fragment.id,
                                        defined
                                            .pre_regalloc
                                            .minimum_fragment_count(region.compile_metadata.tier),
                                    ));
                                }
                                Ok(defined) => {
                                    round_preflighted.insert(fragment.id, defined);
                                }
                                Err(error) if error.code == "JIT_CRANELIFT_PRE_REGALLOC_BUDGET" => {
                                    // A hard-limit rejection does not expose
                                    // trustworthy metrics. Bisect it and let
                                    // the next exact preflight size both
                                    // children before any regalloc work.
                                    offending_fragments.push((fragment.id, 2));
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    }
                    if offending_fragments.is_empty() {
                        preflighted_fragments = round_preflighted;
                        break;
                    }
                    if replan_attempt == MAX_PRE_REGALLOC_REPLAN_ATTEMPTS {
                        return Err(CraneliftLoweringError::new(
                            "JIT_CRANELIFT_PRE_REGALLOC_REPLAN_LIMIT",
                            format!(
                                "fragments {offending_fragments:?} still exceed the exact pre-regalloc safety margin after {MAX_PRE_REGALLOC_REPLAN_ATTEMPTS} deterministic replan rounds"
                            ),
                        ));
                    }
                    // Refine every exact offender in the same deterministic
                    // round. Splitting only the first offender made the global
                    // attempt limit depend on how many independently large
                    // fragments a function happened to contain. Descending IDs
                    // keep lower fragment IDs stable while each split
                    // re-enumerates the plan.
                    offending_fragments.sort_unstable_by_key(|(fragment_id, _)| *fragment_id);
                    offending_fragments.dedup_by_key(|(fragment_id, _)| *fragment_id);
                    for (fragment_id, pieces) in offending_fragments.into_iter().rev() {
                        let block_shape = active_plan
                            .fragments
                            .iter()
                            .find(|fragment| fragment.id == fragment_id)
                            .map(|fragment| {
                                fragment
                                    .blocks
                                    .iter()
                                    .map(|block| {
                                        let region_block = &region.blocks[block.index()];
                                        let instructions = region_block
                                            .instructions
                                            .iter()
                                            .map(|instruction| {
                                                let manifest =
                                                    crate::region_ir::baseline_instruction_lowering(
                                                        &instruction.source_kind,
                                                    );
                                                format!(
                                                    "{}(uses={},live={})",
                                                    manifest.variant,
                                                    instruction.register_uses().len(),
                                                    instruction.live_locals.len(),
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join("+");
                                        format!(
                                            "{}(source={}):instructions={}:{}:entry-live={}:terminator={}:terminator-live={}:terminator-registers={}",
                                            block.raw(),
                                            region_block.source_block.raw(),
                                            region_block.instructions.len(),
                                            instructions,
                                            region_block.entry_live_locals.len(),
                                            crate::region_ir::baseline_terminator_lowering(
                                                &region_block.source_terminator,
                                            )
                                            .variant,
                                            region_block.terminator_live_locals.len(),
                                            region_block
                                                .terminator_live_registers
                                                .as_ref()
                                                .map_or(0, Vec::len),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",")
                            })
                            .unwrap_or_default();
                        active_plan = active_plan.refine_fragment_into(region, fragment_id, pieces).ok_or_else(|| {
                            CraneliftLoweringError::new(
                                "JIT_CRANELIFT_PRE_REGALLOC_UNSPLITTABLE",
                                format!(
                                    "function {} fragment {fragment_id} exceeds the exact pre-regalloc safety margin and contains no safe Region-block cut (block:instruction-count={block_shape})",
                                    region.function_name,
                                ),
                            )
                        })?;
                    }
                    compiled_pre_regalloc_replans
                        .set(compiled_pre_regalloc_replans.get().saturating_add(1));
                    active_fragment_layout =
                        Some(NativeFunctionFragmentLayout::for_plan(region, &active_plan)?);
                    (fragment_functions, fragment_symbols) = declare_fragment_functions(
                        module,
                        name,
                        region,
                        active_fragment_layout.as_ref(),
                        replan_attempt + 1,
                        &mut next_synthetic,
                        &mut functions,
                    )?;
                }
            }
            {
                let referenced_internal_functions = std::cell::RefCell::new(BTreeSet::new());
                let mut append_defined = |symbol: FunctionId,
                                      arity: u8,
                                      local_count: u32,
                                      mut defined: DefinedRegionFunction|
             -> Result<(u64, u32), CraneliftLoweringError> {
                let alignment = usize::try_from(defined.alignment).map_err(|_| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_CACHE_ALIGNMENT",
                        "native function alignment does not fit usize",
                    )
                })?;
                let padding = if alignment == 0 {
                    0
                } else {
                    (alignment - relocatable_bytes.len() % alignment) % alignment
                };
                relocatable_bytes.resize(relocatable_bytes.len().saturating_add(padding), 0);
                let code_offset = relocatable_bytes.len() as u64;
                let candidate_bytes = defined.code.len() as u64;
                clif_blocks = clif_blocks.saturating_add(defined.clif_blocks);
                maximum_pre_regalloc.max_assign(defined.pre_regalloc);
                maximum_temporary_cache_entries = maximum_temporary_cache_entries
                    .max(defined.maximum_temporary_cache_entries);
                relocatable_bytes.extend_from_slice(&defined.code);
                for relocation in &mut defined.relocations {
                    if let crate::JitRelocatableTarget::InternalFunction(function) =
                        &relocation.target
                    {
                        referenced_internal_functions
                            .borrow_mut()
                            .insert(*function);
                    }
                    relocation.offset = relocation.offset.saturating_add(code_offset);
                }
                relocatable_relocations.append(&mut defined.relocations);
                emitted_production_lowering.append(&mut defined.production_lowering);
                relocatable_functions.push(crate::JitRelocatableFunction {
                    function: symbol,
                    code_offset,
                    code_len: candidate_bytes,
                    arity,
                    local_count,
                });
                code_bytes = code_bytes.saturating_add(candidate_bytes);
                native_pc_ranges.append(&mut defined.native_pc_ranges);
                Ok((candidate_bytes, defined.native_stack_bytes))
            };
            // A compile group may contain many bounded native fragments. Reuse
            // Cranelift's allocation-heavy translation scratch sequentially;
            // `clear_context` preserves its backing allocations after every
            // fragment while regalloc still sees only one fragment at a time.
            for candidate in regions.values() {
                if let Some(layout) = &active_fragment_layout {
                    let mut function_bytes = 0_u64;
                    let mut maximum_stack = 0_u32;
                    if layout.fragments.len() == 1 {
                        let fragment = &layout.fragments[0];
                        let defined = if let Some(preflighted) =
                            preflighted_fragments.remove(&fragment.id)
                        {
                            compile_preflighted_region_function(
                                module,
                                codegen_context,
                                functions[&candidate.function],
                                candidate,
                                &functions,
                                preflighted,
                            )?
                        } else {
                            let compiler =
                                crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
                                    candidate.compile_metadata.tier,
                                );
                            compiler.compile_fragment(&mut |compilation_mode| {
                                define_region_graph_function(
                                    module,
                                    codegen_context,
                                    builder_context,
                                    candidate,
                                    &unit.constants,
                                    &value_flows[&candidate.function],
                                    functions[&candidate.function],
                                    &functions,
                                    &inline_constants,
                                    &tail_forwards,
                                    &function_params,
                                    &request.external_function_signatures,
                                    tier_operations,
                                    &layout.register_liveness,
                                    Some(NativeFragmentDefinition {
                                        layout,
                                        fragment,
                                        functions: &fragment_functions,
                                    }),
                                    runtime_unit_identity,
                                    compilation_mode,
                                    true,
                                    false,
                                )
                            })?
                        };
                        let metrics = append_defined(
                            candidate.function,
                            region_arity(candidate)?,
                            candidate.local_count,
                            defined,
                        )?;
                        function_code_metrics.insert(candidate.function, metrics);
                        continue;
                    }
                    for fragment in &layout.fragments {
                        let defined = if let Some(preflighted) =
                            preflighted_fragments.remove(&fragment.id)
                        {
                            compile_preflighted_region_function(
                                module,
                                codegen_context,
                                fragment_functions[&fragment.id],
                                candidate,
                                &functions,
                                preflighted,
                            )?
                        } else {
                            let compiler =
                                crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
                                    candidate.compile_metadata.tier,
                                );
                            compiler.compile_fragment(&mut |compilation_mode| {
                                define_region_graph_function(
                                    module,
                                    codegen_context,
                                    builder_context,
                                    candidate,
                                    &unit.constants,
                                    &value_flows[&candidate.function],
                                    fragment_functions[&fragment.id],
                                    &functions,
                                    &inline_constants,
                                    &tail_forwards,
                                    &function_params,
                                    &request.external_function_signatures,
                                    tier_operations,
                                    &layout.register_liveness,
                                    Some(NativeFragmentDefinition {
                                        layout,
                                        fragment,
                                        functions: &fragment_functions,
                                    }),
                                    runtime_unit_identity,
                                    compilation_mode,
                                    false,
                                    false,
                                )
                            })?
                        };
                        let (bytes, stack) = append_defined(
                            fragment_symbols[&fragment.id],
                            0,
                            candidate.local_count,
                            defined,
                        )?;
                        function_bytes = function_bytes.saturating_add(bytes);
                        maximum_stack = maximum_stack.max(stack);
                    }
                    let wrapper = define_region_fragment_wrapper(
                        module,
                        codegen_context,
                        builder_context,
                        candidate,
                        functions[&candidate.function],
                        &fragment_functions,
                        layout,
                        &functions,
                        &value_flows[&candidate.function],
                        tier_operations,
                    )?;
                    let (bytes, stack) = append_defined(
                        candidate.function,
                        region_arity(candidate)?,
                        candidate.local_count,
                        wrapper,
                    )?;
                    function_bytes = function_bytes.saturating_add(bytes);
                    maximum_stack = maximum_stack.max(stack);
                    function_code_metrics
                        .insert(candidate.function, (function_bytes, maximum_stack));
                } else {
                    let register_liveness = NativeRegisterLiveness::analyze(candidate);
                    let defined = if let Some(preflighted) = preflighted_whole.take() {
                        compile_preflighted_region_function(
                            module,
                            codegen_context,
                            functions[&candidate.function],
                            candidate,
                            &functions,
                            preflighted,
                        )
                    } else {
                        let compiler =
                            crate::cranelift_lowering::baseline_streaming::compiler_for_tier(
                                candidate.compile_metadata.tier,
                            );
                        compiler.compile_fragment(&mut |compilation_mode| {
                            define_region_graph_function(
                                module,
                                codegen_context,
                                builder_context,
                                candidate,
                                &unit.constants,
                                &value_flows[&candidate.function],
                                functions[&candidate.function],
                                &functions,
                                &inline_constants,
                                &tail_forwards,
                                &function_params,
                                &request.external_function_signatures,
                                tier_operations,
                                &register_liveness,
                                None,
                                runtime_unit_identity,
                                compilation_mode,
                                false,
                                false,
                            )
                        })
                    }?;
                    let metrics = append_defined(
                        candidate.function,
                        region_arity(candidate)?,
                        candidate.local_count,
                        defined,
                    )?;
                    function_code_metrics.insert(candidate.function, metrics);
                }
            }
            let referenced = referenced_internal_functions.borrow().clone();
            match tier_operations {
                NativeTierOperations::Optimizing { operations } => {
                // Optimizing support functions are part of an artifact only
                // when its emitted CLIF actually relocates to them. The old
                // unconditional bundle compiled and published all five
                // bodies even for pure scalar functions. Keep the exact
                // native dependency closure for the emitted direct paths.
                let needs_ensure =
                    referenced.contains(&operations.array_ensure_unique_symbol);
                let needs_child =
                    referenced.contains(&operations.array_child_entry_symbol);
                let needs_validate =
                    referenced.contains(&operations.value_release_validate_symbol);
                let needs_commit =
                    referenced.contains(&operations.value_release_commit_symbol);

                if needs_ensure {
                    let defined = define_direct_array_ensure_unique_function(
                        module,
                        codegen_context,
                        builder_context,
                        operations.array_ensure_unique,
                    )?;
                    let _ = append_defined(
                        operations.array_ensure_unique_symbol,
                        0,
                        0,
                        defined,
                    )?;
                }
                if needs_child {
                    let defined = define_direct_array_child_entry_function(
                        module,
                        codegen_context,
                        builder_context,
                        operations.array_child_entry,
                    )?;
                    let _ = append_defined(
                        operations.array_child_entry_symbol,
                        0,
                        0,
                        defined,
                    )?;
                }
                if needs_validate {
                    let defined = define_direct_value_release_validate_function(
                        module,
                        codegen_context,
                        builder_context,
                        operations.value_release_validate,
                        operations.value_release_validate_symbol,
                    )?;
                    let _ = append_defined(
                        operations.value_release_validate_symbol,
                        0,
                        0,
                        defined,
                    )?;
                }
                if needs_commit {
                    let defined = define_direct_value_release_commit_function(
                        module,
                        codegen_context,
                        builder_context,
                        operations.value_release_commit,
                        operations.value_release_commit_symbol,
                    )?;
                    let _ = append_defined(
                        operations.value_release_commit_symbol,
                        0,
                        0,
                        defined,
                    )?;
                }
                },
                NativeTierOperations::Baseline {
                    value_release_commit,
                    value_release_commit_symbol,
                    ..
                } if referenced.contains(&value_release_commit_symbol) => {
                    let defined = define_direct_value_release_commit_function(
                        module,
                        codegen_context,
                        builder_context,
                        value_release_commit,
                        value_release_commit_symbol,
                    )?;
                    let _ = append_defined(value_release_commit_symbol, 0, 0, defined)?;
                },
                NativeTierOperations::Baseline { .. } => {},
            }
            }
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                module
                    .finalize_definitions()
                    .map_err(|error| error.to_string())
            }))
            .map_err(|payload| {
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| {
                        payload
                            .downcast_ref::<&str>()
                            .map(|value| (*value).to_owned())
                    })
                    .unwrap_or_else(|| "Cranelift finalization panicked".to_owned());
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_FINALIZE",
                    format!("failed to finalize executable region call graph: {message}"),
                )
            })?
            .map_err(|error| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_FINALIZE",
                    format!("failed to finalize executable region call graph: {error}"),
                )
            })?;
            let function_entries = regions
                .values()
                .map(|candidate| {
                    let (function_code_bytes, native_stack_bytes) =
                        function_code_metrics[&candidate.function];
                    Ok(crate::JitNativeFunctionEntryMetadata {
                        function: candidate.function,
                        address: module.get_finalized_function(functions[&candidate.function])
                            as usize,
                        arity: region_arity(candidate)?,
                        code_bytes: function_code_bytes,
                        native_stack_bytes,
                        local_count: candidate.local_count,
                        direct_call_sites: candidate
                            .blocks
                            .iter()
                            .flat_map(|block| &block.instructions)
                            .filter(|instruction| {
                                matches!(&instruction.kind, RegionInstructionKind::NativeCall(call)
                                if call.direct_compiled_target().is_some_and(|target| {
                                    (regions.contains_key(&target) || needs_function_resolver)
                                        && function_params.get(&target).is_some_and(
                                            |metadata| {
                                                metadata.native_arity == call.operands.len()
                                                    && (!metadata.requires_trampoline
                                                        || (metadata.reference_only_trampoline
                                                            && metadata.params.iter().enumerate().all(
                                                                |(index, parameter)| {
                                                                        !parameter.by_ref
                                                                        || call.args.get(index).is_some_and(
                                                                            |argument| {
                                                                                argument.by_ref_local.is_some()
                                                                            },
                                                                        )
                                                                },
                                                            )))
                                            },
                                        )
                                        && !matches!(
                                            call.result,
                                            RegionCallResult::ReferenceLocal(_)
                                        )
                                        && call.args.iter().all(|argument| {
                                            argument.name.is_none() && !argument.unpack
                                        })
                                        && !(call.operands.is_empty()
                                            && inline_constants.contains_key(&target))
                                }))
                            })
                            .count() as u64,
                        direct_method_call_sites: candidate
                            .blocks
                            .iter()
                            .flat_map(|block| &block.instructions)
                            .filter(|instruction| {
                                matches!(&instruction.kind, RegionInstructionKind::NativeCall(call)
                                if call.argument_operand_offset == 1
                                    && call.direct_compiled_target().is_some_and(|target| {
                                        (regions.contains_key(&target) || needs_function_resolver)
                                            && function_params
                                                .get(&target)
                                                .is_some_and(|metadata| {
                                                    !metadata.requires_trampoline
                                                })
                                            && !matches!(
                                                call.result,
                                                RegionCallResult::ReferenceLocal(_)
                                            )
                                            && call.args.iter().all(|argument| {
                                                argument.name.is_none() && !argument.unpack
                                            })
                                    }))
                            })
                            .count() as u64,
                        inlined_call_sites: candidate
                            .blocks
                            .iter()
                            .flat_map(|block| &block.instructions)
                            .filter(|instruction| {
                                matches!(&instruction.kind, RegionInstructionKind::NativeCall(call)
                                if call.direct_compiled_target().is_some_and(|target| {
                                    inline_constants
                                        .get(&target)
                                        .copied()
                                        .and_then(|value| bounded_inline_call_operand(call, value))
                                        .is_some()
                                }))
                            })
                            .count() as u64,
                        inline_bytes_added: candidate
                            .blocks
                            .iter()
                            .flat_map(|block| &block.instructions)
                            .filter(|instruction| {
                                matches!(&instruction.kind, RegionInstructionKind::NativeCall(call)
                                if call.direct_compiled_target().is_some_and(|target| {
                                    inline_constants
                                        .get(&target)
                                        .copied()
                                        .and_then(|value| bounded_inline_call_operand(call, value))
                                        .is_some()
                                }))
                            })
                            .count() as u64
                            * 8,
                        tail_call_sites: tail_forwards
                            .keys()
                            .filter(|(function, _)| *function == candidate.function)
                            .count() as u64,
                        inline_rejected_by_reason: inline_rejection_counts(candidate, &regions),
                    })
                })
                .collect::<Result<Vec<_>, CraneliftLoweringError>>()?;
            let root = functions[&function];
            let address = module.get_finalized_function(root) as usize;
            let region_state_metadata = region_graph_metadata(
                function,
                region.local_count,
                regions.values(),
                native_pc_ranges,
                function_entries,
                active_fragment_layout
                    .as_ref()
                    .map(|layout| &layout.register_liveness),
                &value_flows,
                emitted_production_lowering,
            );
            let mut handle = JitFunctionHandle::i64_status_out_native(
                u64::from(function.raw()) + 1,
                request.region_id.clone(),
                CraneliftCompilerIdentity,
                address,
                arity,
                code_bytes,
                0,
                fast_path_hits,
                region_state_metadata,
            );
            if compilation_mode
                == crate::cranelift_lowering::baseline_streaming::NativeCompilationMode::SsaOptimizing
            {
                let forbidden = relocatable_relocations.iter().find_map(|relocation| {
                    match &relocation.target {
                        crate::JitRelocatableTarget::Helper(symbol)
                            if symbol.starts_with("phrust_baseline_") =>
                        {
                            Some(symbol.as_str())
                        }
                        crate::JitRelocatableTarget::Helper(symbol)
                            if matches!(
                                symbol.as_str(),
                                "phrust_native_define"
                                    | "phrust_native_defined"
                                    | "phrust_native_constant"
                                    | "phrust_native_echo_bytes"
                                    | "phrust_native_float_to_string"
                                    | "phrust_native_numeric_string"
                                    | "phrust_native_fmod_f64"
                                    | "phrust_native_round_f64"
                                    | "phrust_native_array_cast"
                                    | "phrust_native_int_cast"
                                    | "phrust_native_float_cast"
                                    | "phrust_native_string_cast"
                                    | "phrust_native_callback_return_string"
                                    | "phrust_native_add"
                                    | "phrust_native_subtract"
                                    | "phrust_native_multiply"
                                    | "phrust_native_divide"
                                    | "phrust_native_modulo"
                                    | "phrust_native_concat"
                                    | "phrust_native_power"
                                    | "phrust_native_bit_and"
                                    | "phrust_native_bit_or"
                                    | "phrust_native_bit_xor"
                                    | "phrust_native_shift_left"
                                    | "phrust_native_shift_right"
                                    | "phrust_native_unary_plus"
                                    | "phrust_native_unary_minus"
                                    | "phrust_native_bit_not"
                                    | "phrust_native_equal"
                                    | "phrust_native_not_equal"
                                    | "phrust_native_identical"
                                    | "phrust_native_not_identical"
                                    | "phrust_native_less"
                                    | "phrust_native_less_equal"
                                    | "phrust_native_greater"
                                    | "phrust_native_greater_equal"
                                    | "phrust_native_spaceship"
                                    | "phrust_native_object_cast"
                                    | "phrust_native_object_class_name"
                                    | "phrust_native_acquire_callable"
                                    | "phrust_native_is_callable"
                                    | "phrust_native_resolve_callable"
                                    | "phrust_native_dynamic_instanceof"
                                    | "phrust_native_prepared_object_new"
                                    | "phrust_native_prepared_exception_new"
                                    | "phrust_native_prepared_closure_new"
                                    | "phrust_native_plain_object_clone"
                                    | "phrust_native_dynamic_property_slot"
                                    | "phrust_native_dynamic_property_test_slot"
                                    | "phrust_native_function_exists"
                                    | "phrust_native_class_exists"
                                    | "phrust_native_interface_exists"
                                    | "phrust_native_trait_exists"
                                    | "phrust_native_enum_exists"
                                    | "phrust_native_method_exists"
                                    | "phrust_native_property_exists"
                                    | "phrust_native_execution_poll"
                            )
                                || symbol.starts_with("phrust_native_preg_")
                                || symbol.starts_with("phrust_native_json_")
                                || StablePureMathBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableBaseConversionBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || symbol == "phrust_native_intval_base"
                                || StableNetworkAddressBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableCompressionCodecBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableStringSearchCompareBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableStringRewriteBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableHtmlCodecBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableUrlQueryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableArrayAggregateBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableRecursiveArrayBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableArraySortBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || symbol == "phrust_native_array_multisort"
                                || StableObjectIdentityBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableCallbackHandlerBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableAutoloadCallbackBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || symbol == "phrust_native_register_shutdown_function"
                                || StableSerializationBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableTokenizerBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableMbstringBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableBcmathBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableFilterBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableSessionBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableObjectVarsBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableClassMetadataBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableClassLineageBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableExtensionQueryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableMemoryQueryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableGcBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableResourceQueryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableErrorStateBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || symbol == "phrust_native_settype"
                                || StableConfigurationBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableHttpResponseBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableCookieBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableClockBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableDateBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableRandomBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableRequestQueryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || StableDeclarationInventoryBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || symbol == "phrust_native_get_defined_constants"
                                || symbol == "phrust_native_compact"
                                || StableFrameIntrospectionBuiltin::all()
                                    .iter()
                                    .any(|builtin| builtin.symbol() == symbol)
                                || matches!(
                                    symbol.as_str(),
                                    "phrust_native_sprintf"
                                        | "phrust_native_printf"
                                        | "phrust_native_vsprintf"
                                        | "phrust_native_vprintf"
                                        | "phrust_native_number_format"
                                        | "phrust_native_md5"
                                        | "phrust_native_sha1"
                                        | "phrust_native_crc32"
                                        | "phrust_native_hash"
                                        | "phrust_native_hash_hmac"
                                        | "phrust_native_hash_equals"
                                        | "phrust_native_base64_encode"
                                        | "phrust_native_base64_decode"
                                        | "phrust_native_bin2hex"
                                        | "phrust_native_hex2bin"
                                        | "phrust_native_quoted_printable_decode"
                                        | "phrust_native_urlencode"
                                        | "phrust_native_rawurlencode"
                                        | "phrust_native_urldecode"
                                        | "phrust_native_rawurldecode"
                                        | "phrust_native_convert_uuencode"
                                        | "phrust_native_convert_uudecode"
                                        | "phrust_native_addcslashes"
                                        | "phrust_native_stripcslashes"
                                        | "phrust_native_stripslashes"
                                        | "phrust_native_quotemeta"
                                        | "phrust_native_pack"
                                        | "phrust_native_unpack"
                                        | "phrust_native_basename"
                                        | "phrust_native_dirname"
                                        | "phrust_native_realpath"
                                        | "phrust_native_file_exists"
                                        | "phrust_native_is_file"
                                        | "phrust_native_is_dir"
                                        | "phrust_native_is_readable"
                                        | "phrust_native_is_writable"
                                        | "phrust_native_is_link"
                                        | "phrust_native_fileperms"
                                        | "phrust_native_fileowner"
                                        | "phrust_native_filegroup"
                                        | "phrust_native_filetype"
                                        | "phrust_native_disk_free_space"
                                        | "phrust_native_disk_total_space"
                                        | "phrust_native_pathinfo"
                                        | "phrust_native_stat"
                                        | "phrust_native_lstat"
                                        | "phrust_native_file"
                                        | "phrust_native_glob"
                                        | "phrust_native_opendir"
                                        | "phrust_native_readdir"
                                        | "phrust_native_rewinddir"
                                        | "phrust_native_closedir"
                                        | "phrust_native_scandir"
                                        | "phrust_native_stream_get_meta_data"
                                        | "phrust_native_stream_get_wrappers"
                                        | "phrust_native_stream_is_local"
                                        | "phrust_native_stream_resolve_include_path"
                                        | "phrust_native_stream_context_create"
                                        | "phrust_native_stream_context_get_default"
                                        | "phrust_native_stream_context_get_options"
                                        | "phrust_native_stream_context_set_default"
                                        | "phrust_native_stream_context_set_option"
                                        | "phrust_native_stream_context_set_options"
                                        | "phrust_native_stream_filter_append"
                                        | "phrust_native_stream_filter_prepend"
                                        | "phrust_native_stream_filter_remove"
                                        | "phrust_native_stream_isatty"
                                        | "phrust_native_stream_set_timeout"
                                        | "phrust_native_chmod"
                                        | "phrust_native_symlink"
                                        | "phrust_native_readfile"
                                        | "phrust_native_is_uploaded_file"
                                        | "phrust_native_tempnam"
                                        | "phrust_native_tmpfile"
                                        | "phrust_native_filesize"
                                        | "phrust_native_filemtime"
                                        | "phrust_native_file_get_contents"
                                        | "phrust_native_file_put_contents"
                                        | "phrust_native_rename"
                                        | "phrust_native_unlink"
                                        | "phrust_native_mkdir"
                                        | "phrust_native_rmdir"
                                        | "phrust_native_touch"
                                        | "phrust_native_fopen"
                                        | "phrust_native_fwrite"
                                        | "phrust_native_fclose"
                                        | "phrust_native_fread"
                                        | "phrust_native_fgets"
                                        | "phrust_native_fgetc"
                                        | "phrust_native_feof"
                                        | "phrust_native_fflush"
                                        | "phrust_native_fseek"
                                        | "phrust_native_ftell"
                                        | "phrust_native_ftruncate"
                                        | "phrust_native_rewind"
                                        | "phrust_native_stream_get_contents"
                                        | "phrust_native_stream_copy_to_stream"
                                        | "phrust_native_ob_start"
                                        | "phrust_native_ob_get_clean"
                                        | "phrust_native_ob_get_contents"
                                        | "phrust_native_ob_get_flush"
                                        | "phrust_native_ob_get_length"
                                        | "phrust_native_ob_get_level"
                                        | "phrust_native_ob_end_flush"
                                        | "phrust_native_ob_end_clean"
                                ) =>
                        {
                            None
                        }
                        crate::JitRelocatableTarget::Helper(symbol) => Some(symbol.as_str()),
                        crate::JitRelocatableTarget::InternalFunction(_) => None,
                    }
                });
                if let Some(symbol) = forbidden {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_OPTIMIZER_HELPER_IMPORT",
                        format!(
                            "optimizing artifact attempted to publish forbidden runtime import {symbol}"
                        ),
                    ));
                }
            }
            handle.bind_relocatable_code(crate::JitRelocatableCode {
                root: function,
                code: relocatable_bytes,
                functions: relocatable_functions,
                relocations: relocatable_relocations,
            });
            compiled_clif_blocks.set(Some(clif_blocks));
            compiled_maximum_pre_regalloc.set(Some(maximum_pre_regalloc));
            compiled_maximum_temporary_cache_entries
                .set(Some(maximum_temporary_cache_entries));
            *selected_plan.borrow_mut() = active_plan;
            *selected_fragment_layout.borrow_mut() = active_fragment_layout;
            Ok((handle, code_bytes))
        },
    )?;
    let plan = selected_plan.into_inner();
    let fragment_layout = selected_fragment_layout.into_inner();
    let fragment_frame_metrics = fragment_layout.as_ref().map_or((0, 0, 0), |layout| {
        (
            layout.frame.value_slots,
            layout.frame.shared_register_slots,
            layout.frame.scratch_register_slots,
        )
    });
    let mut handle = compiled.handle;
    handle.bind_ssa_metrics(ssa_metrics.0, ssa_metrics.1, ssa_metrics.2);
    Ok(NativeScalarRegionCompileResult {
        handle,
        code_bytes: compiled.code_bytes,
        clif_blocks: compiled_clif_blocks.get(),
        maximum_pre_regalloc: compiled_maximum_pre_regalloc.get(),
        maximum_temporary_cache_entries: compiled_maximum_temporary_cache_entries.get(),
        fragment_frame_slots: fragment_frame_metrics.0,
        fragment_shared_register_slots: fragment_frame_metrics.1,
        fragment_scratch_register_slots: fragment_frame_metrics.2,
        pre_regalloc_replans: compiled_pre_regalloc_replans.get(),
        fast_path_hits,
        has_control_flow,
        compilation_mode,
        plan,
    })
}

pub(super) fn select_native_region_tier(
    candidate: &mut RegionGraph,
    _plan: &NativeCompilePlan,
    _constants: &[IrConstant],
) {
    let source_register_state = (candidate.compile_metadata.tier == NativeCompilerTier::Optimizing)
        .then(|| {
            let mut source = candidate.clone();
            for block in &mut source.blocks {
                block.terminator_live_registers = None;
                for instruction in &mut block.instructions {
                    instruction.transition_live_registers = None;
                }
            }
            native_register_state_points(&source)
        });
    // Baseline and optimizing code must share real CFG boundaries around a
    // baseline-only island.  Otherwise baseline has no edge on which it can
    // re-enter the published optimizing continuation and one unsupported
    // operation silently downgrades the rest of the PHP block.
    *candidate = prepare_optimizing_baseline_islands(candidate.clone());
    if candidate.compile_metadata.tier == NativeCompilerTier::Optimizing {
        pin_native_transition_registers(
            candidate,
            source_register_state
                .as_ref()
                .expect("optimizing tier owns source register state"),
        );
        let _ = crate::region_ir::opt::optimize_executable_region(candidate);
        // Optimization may remove instructions, but it must not collapse a
        // direct/unsupported family boundary into one lowering block.
        *candidate = prepare_optimizing_baseline_islands(candidate.clone());
        pin_native_transition_registers(
            candidate,
            source_register_state
                .as_ref()
                .expect("optimizing tier owns source register state"),
        );
    }
}

fn validate_region_native_coverage(region: &RegionGraph) -> Result<(), CraneliftLoweringError> {
    if region.local_count as usize > crate::JIT_DEOPT_MAX_SLOTS {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_MISSING_DEOPT_SLOT_LOWERING",
            format!(
                "function {} has {} locals; native state ABI supports {}",
                region.function_name,
                region.local_count,
                crate::JIT_DEOPT_MAX_SLOTS
            ),
        ));
    }
    for block in &region.blocks {
        for instruction in &block.instructions {
            if let RegionInstructionKind::CompileTimeFatal { diagnostic_id } = &instruction.kind {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_IR_COMPILE_FATAL",
                    format!(
                        "function={} diagnostic={} span={}:{}-{}",
                        region.function_name,
                        diagnostic_id,
                        instruction.span.file.raw(),
                        instruction.span.start,
                        instruction.span.end
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn region_arity(region: &RegionGraph) -> Result<u8, CraneliftLoweringError> {
    region.arity().try_into().map_err(|_| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_REGION_ARITY",
            "executable Region IR arity does not fit the native ABI",
        )
    })
}

fn region_instruction_result_register(kind: &RegionInstructionKind) -> Option<RegId> {
    match kind {
        RegionInstructionKind::Move { dst, .. }
        | RegionInstructionKind::LoadLocal { dst, .. }
        | RegionInstructionKind::AssignLocalResult { dst, .. }
        | RegionInstructionKind::Binary { dst, .. }
        | RegionInstructionKind::Unary { dst, .. }
        | RegionInstructionKind::Compare { dst, .. }
        | RegionInstructionKind::Cast { dst, .. }
        | RegionInstructionKind::NewArray { dst }
        | RegionInstructionKind::NewObject { dst, .. }
        | RegionInstructionKind::FetchProperty { dst, .. }
        | RegionInstructionKind::FetchDynamicStaticProperty { dst, .. }
        | RegionInstructionKind::FetchObjectClassName { dst, .. }
        | RegionInstructionKind::AssignProperty { dst, .. }
        | RegionInstructionKind::CloneObject { dst, .. }
        | RegionInstructionKind::CloneWith { dst, .. }
        | RegionInstructionKind::FetchDim { dst, .. }
        | RegionInstructionKind::FetchConst { dst }
        | RegionInstructionKind::AssignDim { dst, .. }
        | RegionInstructionKind::AppendDim { dst, .. }
        | RegionInstructionKind::IssetDim { dst, .. }
        | RegionInstructionKind::EmptyDim { dst, .. }
        | RegionInstructionKind::IssetLocal { dst, .. }
        | RegionInstructionKind::EmptyLocal { dst, .. }
        | RegionInstructionKind::ForeachInit { iterator: dst, .. }
        | RegionInstructionKind::ForeachInitRef { iterator: dst, .. }
        | RegionInstructionKind::ForeachNext { has_value: dst, .. }
        | RegionInstructionKind::ForeachNextRef { has_value: dst, .. } => Some(*dst),
        RegionInstructionKind::ArrayCallback(call) => Some(call.result),
        RegionInstructionKind::PregCallbackArray(call) => Some(call.result),
        RegionInstructionKind::RuntimeFatal { dst: Some(dst), .. } => Some(*dst),
        RegionInstructionKind::NativeCall(RegionNativeCall {
            result: RegionCallResult::Register(dst),
            ..
        }) => Some(*dst),
        RegionInstructionKind::NativeControl(RegionNativeControl::MakeException {
            dst, ..
        }) => Some(*dst),
        RegionInstructionKind::NativeSuspend(
            RegionNativeSuspend::GeneratorYield { dst, .. }
            | RegionNativeSuspend::GeneratorDelegate { dst, .. }
            | RegionNativeSuspend::FiberSuspend { dst, .. },
        ) => Some(*dst),
        RegionInstructionKind::NativeDynamicCode(
            RegionNativeDynamicCode::Include { dst, .. }
            | RegionNativeDynamicCode::Eval { dst, .. }
            | RegionNativeDynamicCode::MakeClosure { dst, .. },
        ) => Some(*dst),
        RegionInstructionKind::Nop
        | RegionInstructionKind::StoreLocal { .. }
        | RegionInstructionKind::BindReference { .. }
        | RegionInstructionKind::BindReferenceDim { .. }
        | RegionInstructionKind::BindReferenceIntoDim { .. }
        | RegionInstructionKind::BindReferenceProperty { .. }
        | RegionInstructionKind::BindReferenceFromProperty { .. }
        | RegionInstructionKind::BindReferenceFromPropertyDim { .. }
        | RegionInstructionKind::BindReferenceIntoPropertyDim { .. }
        | RegionInstructionKind::BindReferenceDimFromProperty { .. }
        | RegionInstructionKind::InitStaticLocal { .. }
        | RegionInstructionKind::Discard { .. }
        | RegionInstructionKind::Echo { .. }
        | RegionInstructionKind::ArrayInsert { .. }
        | RegionInstructionKind::ArraySpread { .. }
        | RegionInstructionKind::UnsetDim { .. }
        | RegionInstructionKind::UnsetLocal { .. }
        | RegionInstructionKind::ForeachCleanup { .. }
        | RegionInstructionKind::NativeCall(RegionNativeCall {
            result: RegionCallResult::ReferenceLocal(_) | RegionCallResult::Discard,
            ..
        })
        | RegionInstructionKind::NativeControl(_)
        | RegionInstructionKind::NativeDynamicCode(_)
        | RegionInstructionKind::RuntimeFatal { dst: None, .. }
        | RegionInstructionKind::CompileTimeFatal { .. } => None,
    }
}

fn region_instruction_defined_registers(kind: &RegionInstructionKind) -> Vec<RegId> {
    let mut registers = region_instruction_result_register(kind)
        .into_iter()
        .collect::<Vec<_>>();
    match kind {
        RegionInstructionKind::ArrayInsert { array, .. }
        | RegionInstructionKind::ArraySpread { array, .. } => registers.push(*array),
        RegionInstructionKind::ForeachNext { key, value, .. } => {
            registers.extend(*key);
            registers.push(*value);
        }
        RegionInstructionKind::ForeachNextRef { key, .. } => registers.extend(*key),
        _ => {}
    }
    registers.sort_unstable();
    registers.dedup();
    registers
}

fn region_register_types(region: &RegionGraph) -> BTreeMap<RegId, ir::Type> {
    region
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .flat_map(|instruction| {
            region_instruction_defined_registers(&instruction.kind)
                .into_iter()
                .map(|register| (register, types::I64))
        })
        .collect()
}

/// Deliberately tiny first inlining tier. It handles only a stable zero-arity
/// function whose complete body returns one scalar constant. This preserves a
/// hard code-growth bound and cannot recursively inline a call graph.
fn bounded_inline_return(region: &RegionGraph) -> Option<BoundedInlineValue> {
    if region.return_type.is_some()
        || region.flags.is_method
        || region.flags.is_closure
        || region.flags.is_generator
        || region.blocks.len() != 1
    {
        return None;
    }
    let block = &region.blocks[0];
    let RegionTerminator::Return {
        value,
        finally: None,
    } = block.terminator
    else {
        return None;
    };
    match block.instructions.as_slice() {
        [] if region.params.is_empty()
            && matches!(value, RegionOperand::I64(_) | RegionOperand::Constant(_)) =>
        {
            Some(BoundedInlineValue::Constant(value))
        }
        [
            RegionInstruction {
                kind: RegionInstructionKind::Move { dst, src },
                ..
            },
        ] if value == RegionOperand::Register(*dst)
            && matches!(src, RegionOperand::I64(_) | RegionOperand::Constant(_)) =>
        {
            Some(BoundedInlineValue::Constant(*src))
        }
        [
            RegionInstruction {
                kind:
                    RegionInstructionKind::LoadLocal {
                        dst,
                        local,
                        quiet: false,
                    },
                ..
            },
        ] if value == RegionOperand::Register(*dst)
            && region.params.iter().all(|parameter| {
                parameter.required
                    && parameter.default.is_none()
                    && parameter.type_.is_none()
                    && !parameter.by_ref
                    && !parameter.variadic
            }) =>
        {
            region
                .parameter_locals
                .iter()
                .position(|parameter| parameter == local)
                .map(|index| BoundedInlineValue::Argument {
                    index,
                    arity: region.params.len(),
                })
        }
        _ => None,
    }
}

fn collect_bounded_inline_values(
    unit: &IrUnit,
    roots: &BTreeMap<FunctionId, RegionGraph>,
) -> BTreeMap<FunctionId, BoundedInlineValue> {
    if !roots
        .values()
        .any(|region| region.compile_metadata.tier == NativeCompilerTier::Optimizing)
    {
        return BTreeMap::new();
    }
    roots
        .values()
        .flat_map(RegionGraph::direct_callees)
        .filter(|callee| !roots.contains_key(callee))
        .filter(|callee| {
            unit.functions
                .get(callee.index())
                .is_some_and(|function| !ir_function_requires_trampoline(function))
        })
        .filter_map(|callee| {
            crate::region_ir::build_baseline_region(unit, callee)
                .ok()
                .and_then(|region| bounded_inline_return(&region))
                .map(|value| (callee, value))
        })
        .collect()
}

fn bounded_inline_rejection(region: &RegionGraph) -> &'static str {
    if !region.params.is_empty() {
        "arguments"
    } else if region.flags.is_method || region.flags.is_closure {
        "receiver-or-closure-environment"
    } else if region.flags.is_generator {
        "suspension"
    } else if region.return_type.is_some() {
        "return-type-check"
    } else if region.blocks.len() != 1 {
        "control-flow-complexity"
    } else {
        "not-bounded-wrapper"
    }
}

fn inline_rejection_counts(
    caller: &RegionGraph,
    regions: &BTreeMap<FunctionId, RegionGraph>,
) -> BTreeMap<String, u64> {
    let mut reasons = BTreeMap::new();
    for call in caller
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            RegionInstructionKind::NativeCall(call) => Some(call),
            _ => None,
        })
    {
        let Some(target) = call.direct_compiled_target() else {
            continue;
        };
        let Some(callee) = regions.get(&target) else {
            continue;
        };
        if bounded_inline_return(callee)
            .and_then(|value| bounded_inline_call_operand(call, value))
            .is_some()
        {
            continue;
        }
        let reason = if call.operands.is_empty() {
            bounded_inline_rejection(callee)
        } else {
            "arguments-or-receiver"
        };
        let count = reasons.entry(reason.to_owned()).or_insert(0_u64);
        *count = count.saturating_add(1);
    }
    reasons
}

/// Selects the deliberately small tail-call subset whose callee can consume
/// the caller's packed argument buffer directly. This avoids allocating a
/// second arena frame and transfers the caller's argument ownership exactly
/// once. More general tail calls need an owned-frame transfer protocol.
fn bounded_tail_forward_target(
    region: &RegionGraph,
    block: &crate::region_ir::RegionBlock,
    regions: &BTreeMap<FunctionId, RegionGraph>,
) -> Option<(u32, FunctionId)> {
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (region, block, regions);
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    {
        let RegionTerminator::Return {
            value: RegionOperand::Register(returned),
            finally: None,
        } = &block.terminator
        else {
            return None;
        };
        let (last, prefix) = block.instructions.split_last()?;
        let RegionInstructionKind::NativeCall(call) = &last.kind else {
            return None;
        };
        let RegionCallResult::Register(destination) = call.result else {
            return None;
        };
        let target = call.direct_compiled_target()?;
        let callee = regions.get(&target)?;
        if destination != *returned
            || target == region.function
            || call.argument_operand_offset != 0
            || call.variadic
            || call.returns_by_reference
            || region.returns_by_ref
            || callee.returns_by_ref
            || region.params != callee.params
            || region.return_type != callee.return_type
            || !region.exception_regions.is_empty()
            || !callee.exception_regions.is_empty()
            || region.flags.is_generator
            || region.flags.is_closure
            || region.flags.is_method
            || callee.flags.is_generator
            || callee.flags.is_closure
            || callee.flags.is_method
            || prefix.len() != region.parameter_locals.len()
            || call.operands.len() != region.parameter_locals.len()
            || !callee
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .all(|instruction| {
                    matches!(
                        instruction.kind,
                        RegionInstructionKind::Nop
                            | RegionInstructionKind::Move { .. }
                            | RegionInstructionKind::LoadLocal { .. }
                    )
                })
        {
            return None;
        }
        for (((instruction, local), operand), parameter) in prefix
            .iter()
            .zip(&region.parameter_locals)
            .zip(&call.operands)
            .zip(&call.args)
        {
            let RegionInstructionKind::LoadLocal {
                dst,
                local: loaded,
                quiet: false,
            } = &instruction.kind
            else {
                return None;
            };
            if *loaded != *local
                || *operand != Some(RegionOperand::Register(*dst))
                || parameter.name.is_some()
                || parameter.unpack
                || parameter.by_ref_local.is_some()
                || parameter.by_ref_dim.is_some()
                || parameter.by_ref_property.is_some()
                || parameter.by_ref_property_dim.is_some()
            {
                return None;
            }
        }
        Some((last.continuation_id, target))
    }
}

fn region_graph_signature(
    module: &JITModule,
    region: &RegionGraph,
) -> Result<Signature, CraneliftLoweringError> {
    region_arity(region)?;
    Ok(native_php_entry_signature(module))
}

fn direct_array_ensure_unique_signature(module: &JITModule) -> Signature {
    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I8));
    signature.returns.push(AbiParam::new(types::I32));
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn direct_array_child_entry_signature(module: &JITModule) -> Signature {
    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(pointer_type));
    signature
}

fn direct_value_release_signature(module: &JITModule) -> Signature {
    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I8));
    signature
}

fn region_fragment_signature(
    module: &JITModule,
    region: &RegionGraph,
) -> Result<Signature, CraneliftLoweringError> {
    region_arity(region)?;
    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    #[cfg(target_arch = "x86_64")]
    {
        signature.call_conv = CallConv::Tail;
    }
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    Ok(signature)
}

type DeclaredFragmentFunctions = (BTreeMap<u32, FuncId>, BTreeMap<u32, FunctionId>);

#[allow(clippy::too_many_arguments)]
fn declare_fragment_functions(
    module: &mut JITModule,
    root_symbol: &str,
    region: &RegionGraph,
    layout: Option<&NativeFunctionFragmentLayout>,
    replan_attempt: usize,
    next_synthetic: &mut u32,
    functions: &mut BTreeMap<FunctionId, FuncId>,
) -> Result<DeclaredFragmentFunctions, CraneliftLoweringError> {
    let mut fragment_functions = BTreeMap::new();
    let mut fragment_symbols = BTreeMap::new();
    let Some(layout) = layout else {
        return Ok((fragment_functions, fragment_symbols));
    };
    if layout.fragments.len() == 1 {
        fragment_functions.insert(layout.fragments[0].id, functions[&region.function]);
        return Ok((fragment_functions, fragment_symbols));
    }
    for fragment in &layout.fragments {
        let synthetic = FunctionId::new(*next_synthetic);
        *next_synthetic = next_synthetic.checked_add(1).ok_or_else(|| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_FRAGMENT_SYMBOL_LIMIT",
                "native fragment symbol id overflowed",
            )
        })?;
        let symbol = if replan_attempt == 0 {
            format!("{root_symbol}.fragment.{}", fragment.id)
        } else {
            format!(
                "{root_symbol}.replan.{replan_attempt}.fragment.{}",
                fragment.id
            )
        };
        let signature = region_fragment_signature(module, region)?;
        let func_id = module
            .declare_function(&symbol, Linkage::Local, &signature)
            .map_err(|error| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_DECLARE_FRAGMENT",
                    format!("failed to declare native fragment {symbol}: {error}"),
                )
            })?;
        fragment_functions.insert(fragment.id, func_id);
        fragment_symbols.insert(fragment.id, synthetic);
        functions.insert(synthetic, func_id);
    }
    Ok((fragment_functions, fragment_symbols))
}

pub(super) struct DefinedRegionFunction {
    lowered_function: Option<ir::Function>,
    code: Vec<u8>,
    clif_blocks: usize,
    alignment: u64,
    relocations: Vec<crate::JitRelocatableRelocation>,
    native_pc_ranges: Vec<crate::JitNativePcRange>,
    native_stack_bytes: u32,
    pre_regalloc: PreRegallocMetrics,
    maximum_temporary_cache_entries: usize,
    production_lowering: Vec<crate::JitProductionLoweringMetadata>,
}

const MAX_NATIVE_SPILL_FRAME_BYTES: u32 = 1024 * 1024;
const MAX_FRAGMENT_CLIF_BLOCKS: usize = 768;
const MAX_OPTIMIZING_CLIF_BLOCKS: usize = 4_096;
const MAX_FRAGMENT_CLIF_VALUES: usize = 16_384;
const MAX_OPTIMIZING_CLIF_VALUES: usize = 65_536;
const MAX_FRAGMENT_CLIF_INSTRUCTIONS: usize = 32_768;
const MAX_OPTIMIZING_CLIF_INSTRUCTIONS: usize = 65_536;
const MAX_FRAGMENT_BLOCK_PARAMETERS: usize = 4_096;
const MAX_OPTIMIZING_BLOCK_PARAMETERS: usize = 16_384;
// Exact CLIF must retain 30% headroom below the absolute backend ceiling.
// This is intentionally stricter than merely avoiding a hard rejection: it
// keeps the admitted regalloc graph away from the nonlinear edge while the
// planner's cheaper estimate remains calibrated independently.
const PRE_REGALLOC_REPLAN_MARGIN_PERCENT: usize = 70;
// The planner admits at most 64 Region blocks per fragment. Six bisection
// rounds are therefore sufficient to reduce every splittable offender to one
// Region block (ceil(log2(64))). A remaining offender is structurally
// unsplittable and is rejected before regalloc; this is a proof-derived bound,
// not a wall-time retry budget.
const MAX_PRE_REGALLOC_REPLAN_ATTEMPTS: usize = 6;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PreRegallocMetrics {
    pub(super) blocks: usize,
    pub(super) values: usize,
    pub(super) instructions: usize,
    pub(super) block_parameters: usize,
    pub(super) loads: usize,
    pub(super) stores: usize,
    pub(super) loads_per_source_instruction_milli: usize,
    pub(super) stores_per_source_instruction_milli: usize,
}

impl PreRegallocMetrics {
    fn limits(tier: NativeCompilerTier) -> (usize, usize, usize, usize) {
        if tier == NativeCompilerTier::Optimizing {
            (
                MAX_OPTIMIZING_CLIF_BLOCKS,
                MAX_OPTIMIZING_CLIF_VALUES,
                MAX_OPTIMIZING_CLIF_INSTRUCTIONS,
                MAX_OPTIMIZING_BLOCK_PARAMETERS,
            )
        } else {
            (
                MAX_FRAGMENT_CLIF_BLOCKS,
                MAX_FRAGMENT_CLIF_VALUES,
                MAX_FRAGMENT_CLIF_INSTRUCTIONS,
                MAX_FRAGMENT_BLOCK_PARAMETERS,
            )
        }
    }

    fn exceeds_replan_margin(self, tier: NativeCompilerTier) -> bool {
        let (blocks, values, instructions, parameters) = Self::limits(tier);
        self.blocks.saturating_mul(100) > blocks.saturating_mul(PRE_REGALLOC_REPLAN_MARGIN_PERCENT)
            || self.values.saturating_mul(100)
                > values.saturating_mul(PRE_REGALLOC_REPLAN_MARGIN_PERCENT)
            || self.instructions.saturating_mul(100)
                > instructions.saturating_mul(PRE_REGALLOC_REPLAN_MARGIN_PERCENT)
            || self.block_parameters.saturating_mul(100)
                > parameters.saturating_mul(PRE_REGALLOC_REPLAN_MARGIN_PERCENT)
    }

    /// Minimum number of approximately balanced fragments required by the
    /// largest exact CLIF dimension. This is a planning hint only: every
    /// resulting fragment is exact-preflighted again before regalloc.
    fn minimum_fragment_count(self, tier: NativeCompilerTier) -> usize {
        let percent = PRE_REGALLOC_REPLAN_MARGIN_PERCENT;
        let (blocks, values, instructions, parameters) = Self::limits(tier);
        let block_limit = blocks.saturating_mul(percent) / 100;
        let value_limit = values.saturating_mul(percent) / 100;
        let instruction_limit = instructions.saturating_mul(percent) / 100;
        let parameter_limit = parameters.saturating_mul(percent) / 100;
        [
            self.blocks.div_ceil(block_limit.max(1)),
            self.values.div_ceil(value_limit.max(1)),
            self.instructions.div_ceil(instruction_limit.max(1)),
            self.block_parameters.div_ceil(parameter_limit.max(1)),
        ]
        .into_iter()
        .max()
        .unwrap_or(2)
        .max(2)
    }

    fn max_assign(&mut self, other: Self) {
        self.blocks = self.blocks.max(other.blocks);
        self.values = self.values.max(other.values);
        self.instructions = self.instructions.max(other.instructions);
        self.block_parameters = self.block_parameters.max(other.block_parameters);
        self.loads = self.loads.max(other.loads);
        self.stores = self.stores.max(other.stores);
        self.loads_per_source_instruction_milli = self
            .loads_per_source_instruction_milli
            .max(other.loads_per_source_instruction_milli);
        self.stores_per_source_instruction_milli = self
            .stores_per_source_instruction_milli
            .max(other.stores_per_source_instruction_milli);
    }
}

pub(super) fn validate_pre_regalloc_structure(
    function: &ir::Function,
    region: &RegionGraph,
    fragment: Option<u32>,
) -> Result<PreRegallocMetrics, CraneliftLoweringError> {
    let blocks = function.layout.blocks().count();
    let values = function.dfg.num_values();
    let instructions = function
        .layout
        .blocks()
        .map(|block| function.layout.block_insts(block).count())
        .sum::<usize>();
    let block_parameters = function
        .layout
        .blocks()
        .map(|block| function.dfg.block_params(block).len())
        .sum::<usize>();
    let mut loads = 0_usize;
    let mut stores = 0_usize;
    for block in function.layout.blocks() {
        for instruction in function.layout.block_insts(block) {
            match function.dfg.insts[instruction].opcode() {
                ir::Opcode::Load | ir::Opcode::StackLoad => loads = loads.saturating_add(1),
                ir::Opcode::Store | ir::Opcode::StackStore => stores = stores.saturating_add(1),
                _ => {}
            }
        }
    }
    let (maximum_blocks, maximum_values, maximum_instructions, maximum_block_parameters) =
        if region.compile_metadata.tier == NativeCompilerTier::Optimizing {
            (
                MAX_OPTIMIZING_CLIF_BLOCKS,
                MAX_OPTIMIZING_CLIF_VALUES,
                MAX_OPTIMIZING_CLIF_INSTRUCTIONS,
                MAX_OPTIMIZING_BLOCK_PARAMETERS,
            )
        } else {
            (
                MAX_FRAGMENT_CLIF_BLOCKS,
                MAX_FRAGMENT_CLIF_VALUES,
                MAX_FRAGMENT_CLIF_INSTRUCTIONS,
                MAX_FRAGMENT_BLOCK_PARAMETERS,
            )
        };
    if blocks > maximum_blocks
        || values > maximum_values
        || instructions > maximum_instructions
        || block_parameters > maximum_block_parameters
    {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_PRE_REGALLOC_BUDGET",
            format!(
                "function {} fragment={} exceeds the pre-regalloc ceiling: clif_blocks={blocks}/{maximum_blocks} clif_values={values}/{maximum_values} clif_instructions={instructions}/{maximum_instructions} block_parameters={block_parameters}/{maximum_block_parameters}",
                region.function_name,
                fragment.map_or_else(|| "whole".to_owned(), |id| id.to_string()),
            ),
        ));
    }
    Ok(PreRegallocMetrics {
        blocks,
        values,
        instructions,
        block_parameters,
        loads,
        stores,
        loads_per_source_instruction_milli: 0,
        stores_per_source_instruction_milli: 0,
    })
}

fn compile_preflighted_region_function(
    module: &mut JITModule,
    ctx: &mut cranelift_codegen::Context,
    func_id: FuncId,
    region: &RegionGraph,
    functions: &BTreeMap<FunctionId, FuncId>,
    mut defined: DefinedRegionFunction,
) -> Result<DefinedRegionFunction, CraneliftLoweringError> {
    ctx.func = defined.lowered_function.take().ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_MISSING_PREFLIGHT_CLIF",
            "exact preflight did not retain its verified CLIF function",
        )
    })?;
    module.define_function(func_id, ctx).map_err(|error| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_DEFINE",
            format!("failed to define preflighted native function: {error}"),
        )
    })?;
    let compiled = ctx.compiled_code().ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_CACHE_CODE",
            "Cranelift returned no compiled machine-code buffer",
        )
    })?;
    let native_stack_bytes = compiled
        .buffer
        .frame_layout()
        .map_or(0, |layout| layout.frame_to_fp_offset);
    if native_stack_bytes > MAX_NATIVE_SPILL_FRAME_BYTES {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_NATIVE_STACK_LIMIT",
            format!(
                "function {} requires {native_stack_bytes} native stack bytes; limit is {MAX_NATIVE_SPILL_FRAME_BYTES}",
                region.function_name
            ),
        ));
    }
    defined.code = compiled.code_buffer().to_vec();
    defined.alignment = u64::from(compiled.buffer.alignment)
        .max(module.isa().function_alignment().minimum as u64)
        .max(module.isa().symbol_alignment());
    defined.relocations = compiled
        .buffer
        .relocs()
        .iter()
        .map(|relocation| {
            capture_relocation(
                module,
                ModuleReloc::from_mach_reloc(relocation, &ctx.func, func_id),
                functions,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    defined.native_pc_ranges = ctx
        .compiled_code()
        .into_iter()
        .flat_map(|compiled| compiled.buffer.get_srclocs_sorted())
        .filter_map(|range| {
            let source = range.loc.bits();
            (source != 0 && source != u32::MAX).then_some(crate::JitNativePcRange {
                function: region.function,
                start: range.start,
                end: range.end,
                continuation_id: source - 1,
            })
        })
        .collect();
    defined.native_stack_bytes = native_stack_bytes;
    module.clear_context(ctx);
    Ok(defined)
}

#[allow(clippy::too_many_arguments)]
fn define_region_fragment_wrapper(
    module: &mut JITModule,
    ctx: &mut cranelift_codegen::Context,
    builder_context: &mut FunctionBuilderContext,
    region: &RegionGraph,
    func_id: FuncId,
    fragment_functions: &BTreeMap<u32, FuncId>,
    layout: &NativeFunctionFragmentLayout,
    relocation_functions: &BTreeMap<FunctionId, FuncId>,
    value_flow: &ExecutableValueFlow,
    tier_operations: NativeTierOperations,
) -> Result<DefinedRegionFunction, CraneliftLoweringError> {
    let pointer_type = module.target_config().pointer_type();
    ctx.func.signature = region_graph_signature(module, region)?;
    ctx.func.name = UserFuncName::user(0, func_id.as_u32());
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, builder_context);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        let params = builder.block_params(entry).to_vec();
        let runtime = params[0];
        let arguments = params[1];
        let result_out = params[2];
        let deopt_out = params[3];
        let resume_id = params[4];
        let resume_state = params[5];
        if region.compile_metadata.tier == NativeCompilerTier::Baseline {
            lower_baseline_function_entry(&mut builder, deopt_out, region.function)?;
        }
        let (arguments, resume_id) = if region.compile_metadata.tier == NativeCompilerTier::Baseline
        {
            let NativeTierOperations::Baseline { operations, .. } = tier_operations else {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_NATIVE_ENTRY_BINDING",
                    "baseline fragment wrapper has no baseline operation plane",
                ));
            };
            lower_baseline_bind_packed_arguments(
                module,
                &mut builder,
                operations
                    .argument_check
                    .map(|helper| helper.with_runtime(runtime)),
                &region.params,
                region
                    .parameter_locals
                    .len()
                    .saturating_sub(region.params.len()),
                arguments,
                result_out,
                deopt_out,
                resume_id,
                region.function,
            )?
        } else {
            (arguments, resume_id)
        };
        let frame_layout = &layout.frame;
        let frame_bytes = frame_layout.frame_bytes()?;
        let frame_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            frame_bytes,
            3,
        ));
        let frame = builder.ins().stack_addr(pointer_type, frame_slot, 0);
        let uninitialized = builder.ins().iconst(
            types::I64,
            crate::jit_encode_constant(crate::JIT_VALUE_UNINITIALIZED),
        );
        for local in frame_layout.local_slots.keys().copied() {
            let initial = if matches!(
                value_flow.local_storage(local),
                crate::region_ir::LocalStorageClass::RequestGlobal
                    | crate::region_ir::LocalStorageClass::Superglobal
            ) {
                lower_trusted_request_local_reference(
                    &mut builder,
                    deopt_out,
                    region.function,
                    local,
                )
            } else {
                uninitialized
            };
            builder.ins().store(
                MemFlagsData::new(),
                initial,
                frame,
                frame_layout.local_offset(local)?,
            );
        }
        for (index, local) in region.parameter_locals.iter().enumerate() {
            let value = builder.ins().load(
                types::I64,
                MemFlagsData::new(),
                arguments,
                i32::try_from(index.saturating_mul(8)).map_err(|_| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_ARITY",
                        "fragment wrapper argument offset does not fit the native ABI",
                    )
                })?,
            );
            if value_flow.owns_parameter_at_entry(*local) {
                lower_optimizing_retain(&mut builder, value, deopt_out);
            }
            builder.ins().store(
                MemFlagsData::new(),
                value,
                frame,
                frame_layout.local_offset(*local)?,
            );
        }
        let continue_status = builder
            .ins()
            .iconst(types::I32, i64::from(crate::JitCallStatus::CONTINUE.0));
        let empty = builder.ins().iconst(types::I64, 0);
        builder.ins().store(
            MemFlagsData::new(),
            continue_status,
            frame,
            frame_layout.pending_status_offset(),
        );
        builder.ins().store(
            MemFlagsData::new(),
            empty,
            frame,
            frame_layout.pending_value_offset(),
        );
        for (value, offset) in [
            (arguments, frame_layout.arguments_offset()),
            (result_out, frame_layout.result_out_offset()),
            (deopt_out, frame_layout.deopt_out_offset()),
            (resume_state, frame_layout.resume_state_offset()),
        ] {
            builder
                .ins()
                .store(MemFlagsData::new(), value, frame, offset);
        }
        builder.ins().store(
            MemFlagsData::new(),
            resume_id,
            frame,
            frame_layout.resume_id_offset(),
        );

        let call_blocks = layout
            .fragments
            .iter()
            .map(|fragment| (fragment.id, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let root_fragment = layout.block_owner[&BlockId::new(0)];
        if layout.fragments.len() == 1 {
            builder.ins().jump(call_blocks[&root_fragment], &[]);
        } else {
            // Cranelift lowers a sparse `Switch` to control-flow blocks for
            // every resume id. Large PHP functions have hundreds of precise
            // transition ids, so that representation made this tiny wrapper
            // larger than a bounded fragment. Match all ids owned by a
            // fragment in one straight-line predicate instead. Intermediate
            // compare values die immediately and the wrapper CFG now scales
            // with the number of fragments, not the number of safepoints.
            for fragment in &layout.fragments {
                if fragment.id == root_fragment {
                    continue;
                }
                let mut matches_fragment = None;
                for encoded_resume in
                    layout
                        .resume_owner
                        .iter()
                        .filter_map(|(encoded_resume, owner)| {
                            (*owner == fragment.id).then_some(*encoded_resume)
                        })
                {
                    let matches_resume =
                        builder
                            .ins()
                            .icmp_imm(IntCC::Equal, resume_id, i64::from(encoded_resume));
                    matches_fragment = Some(match matches_fragment {
                        Some(previous) => builder.ins().bor(previous, matches_resume),
                        None => matches_resume,
                    });
                }
                if let Some(matches_fragment) = matches_fragment {
                    let next_fragment = builder.create_block();
                    builder.ins().brif(
                        matches_fragment,
                        call_blocks[&fragment.id],
                        &[],
                        next_fragment,
                        &[],
                    );
                    builder.switch_to_block(next_fragment);
                }
            }
            builder.ins().jump(call_blocks[&root_fragment], &[]);
        }

        for fragment in &layout.fragments {
            builder.switch_to_block(call_blocks[&fragment.id]);
            let callee =
                module.declare_func_in_func(fragment_functions[&fragment.id], builder.func);
            let entry_block = fragment
                .normal_entries
                .iter()
                .next()
                .copied()
                .unwrap_or(BlockId::new(0));
            let entry_id = builder
                .ins()
                .iconst(types::I32, i64::from(entry_block.raw()));
            builder.ins().store(
                MemFlagsData::new(),
                entry_id,
                frame,
                frame_layout.entry_id_offset(),
            );
            let call = builder.ins().call(callee, &[runtime, frame]);
            let status = builder.inst_results(call)[0];
            builder.ins().return_(&[status]);
        }
        builder.seal_all_blocks();
        builder.finalize();
    }
    let pre_regalloc = validate_pre_regalloc_structure(&ctx.func, region, None)?;
    let verifier_flags = settings::Flags::new(settings::builder());
    verify_function(&ctx.func, &verifier_flags).map_err(|error| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_FRAGMENT_WRAPPER",
            format!("Cranelift verifier rejected fragment wrapper: {error}"),
        )
    })?;
    let clif_blocks = ctx.func.layout.blocks().count();
    module.define_function(func_id, ctx).map_err(|error| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_FRAGMENT_WRAPPER",
            format!("failed to define native fragment wrapper: {error}"),
        )
    })?;
    let compiled = ctx.compiled_code().ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_FRAGMENT_WRAPPER",
            "Cranelift returned no fragment-wrapper machine code",
        )
    })?;
    let native_stack_bytes = compiled
        .buffer
        .frame_layout()
        .map_or(0, |frame| frame.frame_to_fp_offset);
    if native_stack_bytes > MAX_NATIVE_SPILL_FRAME_BYTES {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_FRAGMENT_WRAPPER_STACK_LIMIT",
            format!(
                "fragment wrapper requires {native_stack_bytes} native stack bytes; limit is {MAX_NATIVE_SPILL_FRAME_BYTES}"
            ),
        ));
    }
    let code = compiled.code_buffer().to_vec();
    let alignment = u64::from(compiled.buffer.alignment)
        .max(module.isa().function_alignment().minimum as u64)
        .max(module.isa().symbol_alignment());
    let relocations = compiled
        .buffer
        .relocs()
        .iter()
        .map(|relocation| {
            capture_relocation(
                module,
                ModuleReloc::from_mach_reloc(relocation, &ctx.func, func_id),
                relocation_functions,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    module.clear_context(ctx);
    Ok(DefinedRegionFunction {
        lowered_function: None,
        code,
        clif_blocks,
        alignment,
        relocations,
        native_pc_ranges: Vec::new(),
        native_stack_bytes,
        pre_regalloc,
        maximum_temporary_cache_entries: 0,
        production_lowering: Vec::new(),
    })
}

fn supported_relocation_kind(kind: Reloc) -> Option<crate::JitRelocatableKind> {
    match kind {
        Reloc::Abs8 => Some(crate::JitRelocatableKind::Abs64),
        Reloc::X86PCRel4 => Some(crate::JitRelocatableKind::X86PcRel4),
        Reloc::X86CallPCRel4 | Reloc::X86CallPLTRel4 => {
            Some(crate::JitRelocatableKind::X86CallPcRel4)
        }
        Reloc::Arm64Call => Some(crate::JitRelocatableKind::Arm64Call),
        _ => None,
    }
}

fn stable_helper_import_name(name: &str) -> String {
    #[cfg(test)]
    {
        if let Some((base, suffix)) = name.rsplit_once('_')
            && suffix.len() == 16
            && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return base.to_owned();
        }
    }
    name.to_owned()
}

fn capture_relocation(
    module: &JITModule,
    relocation: ModuleReloc,
    functions: &BTreeMap<FunctionId, FuncId>,
) -> Result<crate::JitRelocatableRelocation, CraneliftLoweringError> {
    let kind = supported_relocation_kind(relocation.kind).ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_CACHE_RELOCATION",
            format!(
                "Cranelift emitted unsupported restart-cache relocation {:?}",
                relocation.kind
            ),
        )
    })?;
    let internal_function = |func_id: FuncId| {
        functions
            .iter()
            .find_map(|(function, candidate)| (*candidate == func_id).then_some(*function))
    };
    let (target, extra_addend) = match relocation.name {
        ModuleRelocTarget::User {
            namespace: 0,
            index,
        } => {
            let func_id = FuncId::from_u32(index);
            if let Some(function) = internal_function(func_id) {
                (crate::JitRelocatableTarget::InternalFunction(function), 0)
            } else {
                let declaration = module.declarations().get_function_decl(func_id);
                if declaration.linkage != Linkage::Import {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_CACHE_SYMBOL",
                        format!("relocation target {func_id} is neither graph-local nor imported"),
                    ));
                }
                let name = declaration.name.as_deref().ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_CACHE_SYMBOL",
                        format!("imported relocation target {func_id} has no stable name"),
                    )
                })?;
                (
                    crate::JitRelocatableTarget::Helper(stable_helper_import_name(name)),
                    0,
                )
            }
        }
        ModuleRelocTarget::FunctionOffset(func_id, offset) => {
            let function = internal_function(func_id).ok_or_else(|| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_CACHE_SYMBOL",
                    format!("function-offset relocation target {func_id} is not graph-local"),
                )
            })?;
            (
                crate::JitRelocatableTarget::InternalFunction(function),
                i64::from(offset),
            )
        }
        other => {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_CACHE_SYMBOL",
                format!("unsupported restart-cache relocation target {other}"),
            ));
        }
    };
    Ok(crate::JitRelocatableRelocation {
        offset: u64::from(relocation.offset),
        kind,
        target,
        addend: relocation.addend.saturating_add(extra_addend),
    })
}

#[allow(clippy::too_many_arguments)]
fn define_region_graph_function(
    module: &mut JITModule,
    ctx: &mut cranelift_codegen::Context,
    builder_context: &mut FunctionBuilderContext,
    region: &RegionGraph,
    constants: &[IrConstant],
    value_flow: &ExecutableValueFlow,
    func_id: FuncId,
    functions: &BTreeMap<FunctionId, FuncId>,
    inline_constants: &BTreeMap<FunctionId, BoundedInlineValue>,
    tail_forwards: &BTreeMap<(FunctionId, u32), FunctionId>,
    function_params: &BTreeMap<FunctionId, NativeFunctionMetadata>,
    external_function_signatures: &[crate::JitExternalFunctionSignature],
    tier_operations: NativeTierOperations,
    register_liveness: &NativeRegisterLiveness,
    fragment: Option<NativeFragmentDefinition<'_>>,
    unit_identity: u64,
    compilation_mode: crate::cranelift_lowering::baseline_streaming::NativeCompilationMode,
    inline_fragment_entry: bool,
    preflight_only: bool,
) -> Result<DefinedRegionFunction, CraneliftLoweringError> {
    let pointer_type = module.target_config().pointer_type();
    let mut maximum_temporary_cache_entries = 0_usize;
    let mut production_lowering = Vec::new();
    ctx.func.signature = if fragment.is_some() && !inline_fragment_entry {
        region_fragment_signature(module, region)?
    } else {
        region_graph_signature(module, region)?
    };
    ctx.func.name = UserFuncName::user(0, func_id.as_u32());
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, builder_context);
        let owned_blocks = region
            .blocks
            .iter()
            .filter(|block| {
                fragment.is_none_or(|fragment| fragment.fragment.blocks.contains(&block.id))
            })
            .collect::<Vec<_>>();
        let blocks = if let Some(fragment) = fragment {
            fragment
                .fragment
                .blocks
                .iter()
                .chain(&fragment.fragment.external_targets)
                .map(|block| (*block, builder.create_block()))
                .collect::<BTreeMap<_, _>>()
        } else {
            create_region_cranelift_blocks(&mut builder, region)?
        };
        // An optimizing guard failure transfers once to the matching
        // baseline-native continuation. Baseline code deliberately remains
        // in that tier until the PHP call returns: instruction/block-level
        // ping-pong required two independently computed sparse-live layouts
        // to share a positional ABI and could silently restore one register
        // into another. It also rebuilt transition state at every CFG edge.
        let terminator_blocks = blocks.clone();
        // Only true resumable native transitions need an instruction-entry
        // block. Ordinary Region instructions are lowered directly into their
        // PHP CFG block (or the continuation block created by a fallible
        // helper). Creating an entry block for every instruction turns a
        // large but ordinary PHP function into a pathological Cranelift CFG
        // before regalloc2 sees it.
        let transition_blocks = owned_blocks
            .iter()
            .flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .filter(|instruction| {
                        instruction_has_native_resume_entry(
                            instruction,
                            region.compile_metadata.tier,
                        )
                    })
                    .map(|instruction| instruction.continuation_id)
                    .chain(
                        block_terminator_has_native_transition(block, region.compile_metadata.tier)
                            .then_some(block.terminator_continuation_id),
                    )
            })
            .map(|continuation| (continuation, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let suspension_blocks = owned_blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter(|instruction| {
                matches!(instruction.kind, RegionInstructionKind::NativeSuspend(_))
            })
            .map(|instruction| (instruction.continuation_id, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let terminal_exit = builder.create_block();
        builder.set_cold_block(terminal_exit);
        builder.append_block_param(terminal_exit, types::I32);
        builder.append_block_param(terminal_exit, types::I64);
        let optimizing_baseline_resume =
            matches!(tier_operations, NativeTierOperations::Optimizing { .. })
                .then(|| builder.create_block());
        let normal_entry = blocks.values().next().copied().ok_or_else(|| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_HELPER_CONTROL_FLOW",
                "executable region requires at least one block",
            )
        })?;
        let native_entry = builder.create_block();
        builder.append_block_params_for_function_params(native_entry);
        builder.switch_to_block(native_entry);
        let params = builder.block_params(native_entry).to_vec();
        let runtime = params[0];
        let frame_layout = fragment.map(|fragment| &fragment.layout.frame);
        let fragment_frame = if fragment.is_some() {
            if inline_fragment_entry {
                let frame_bytes = frame_layout
                    .expect("inline fragment frame layout")
                    .frame_bytes()?;
                let frame_slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    frame_bytes,
                    3,
                ));
                Some(builder.ins().stack_addr(pointer_type, frame_slot, 0))
            } else {
                Some(params[1])
            }
        } else {
            None
        };
        let streaming_state_frame = if compilation_mode.streams_cfg_state_through_slots() {
            fragment_frame
        } else {
            None
        };
        let (arguments, result_out, deopt_out, resume_id, resume_state, fragment_entry_id) =
            if let Some(frame) = fragment_frame {
                let layout = frame_layout.expect("fragment frame layout");
                let (arguments, result_out, deopt_out, resume_id, resume_state, entry_id) =
                    if inline_fragment_entry {
                        let entry_id = builder.ins().iconst(types::I32, 0);
                        for (value, offset) in [
                            (params[1], layout.arguments_offset()),
                            (params[2], layout.result_out_offset()),
                            (params[3], layout.deopt_out_offset()),
                            (params[5], layout.resume_state_offset()),
                        ] {
                            builder
                                .ins()
                                .store(MemFlagsData::new(), value, frame, offset);
                        }
                        builder.ins().store(
                            MemFlagsData::new(),
                            params[4],
                            frame,
                            layout.resume_id_offset(),
                        );
                        builder.ins().store(
                            MemFlagsData::new(),
                            entry_id,
                            frame,
                            layout.entry_id_offset(),
                        );
                        (
                            params[1], params[2], params[3], params[4], params[5], entry_id,
                        )
                    } else {
                        (
                            builder.ins().load(
                                pointer_type,
                                MemFlagsData::new(),
                                frame,
                                layout.arguments_offset(),
                            ),
                            builder.ins().load(
                                pointer_type,
                                MemFlagsData::new(),
                                frame,
                                layout.result_out_offset(),
                            ),
                            builder.ins().load(
                                pointer_type,
                                MemFlagsData::new(),
                                frame,
                                layout.deopt_out_offset(),
                            ),
                            builder.ins().load(
                                types::I32,
                                MemFlagsData::new(),
                                frame,
                                layout.resume_id_offset(),
                            ),
                            builder.ins().load(
                                pointer_type,
                                MemFlagsData::new(),
                                frame,
                                layout.resume_state_offset(),
                            ),
                            builder.ins().load(
                                types::I32,
                                MemFlagsData::new(),
                                frame,
                                layout.entry_id_offset(),
                            ),
                        )
                    };
                (
                    arguments,
                    result_out,
                    deopt_out,
                    resume_id,
                    resume_state,
                    Some(entry_id),
                )
            } else {
                (params[1], params[2], params[3], params[4], params[5], None)
            };
        if region.compile_metadata.tier == NativeCompilerTier::Baseline
            && (fragment.is_none() || inline_fragment_entry)
        {
            lower_baseline_function_entry(&mut builder, deopt_out, region.function)?;
        }
        let (
            native_call_helper,
            native_dynamic_code_helper,
            mut baseline_operations,
            baseline_value_release_commit,
            execution_poll,
        ) = match tier_operations {
            NativeTierOperations::Baseline {
                call,
                dynamic_code,
                operations,
                value_release_commit,
                ..
            } => {
                let operations =
                    operations
                        .with_runtime(runtime)
                        .with_terminal_exit(NativeTerminalExit {
                            block: terminal_exit,
                        });
                (
                    call.map(|helper| helper.with_runtime(runtime)),
                    dynamic_code.map(|helper| helper.with_runtime(runtime)),
                    Some(operations),
                    Some(module.declare_func_in_func(value_release_commit, builder.func)),
                    operations.execution_poll,
                )
            }
            NativeTierOperations::Optimizing { .. } => {
                let NativeTierOperations::Optimizing { operations } = tier_operations else {
                    unreachable!("optimizing tier was matched above")
                };
                (
                    None,
                    None,
                    None,
                    None,
                    operations
                        .execution_poll
                        .map(|helper| helper.with_runtime(runtime))
                        .map(|helper| {
                            helper.with_terminal_exit(NativeTerminalExit {
                                block: terminal_exit,
                            })
                        }),
                )
            }
        };
        // These guards read the request-owned runtime view directly and only
        // call Rust for reference, warning, destructor, or unsupported dynamic
        // cases. Baseline code needs the same fast paths: forcing every local,
        // scalar comparison, and retain/release through helpers dominated warm
        // execution long after compilation had finished.
        if let Some(native_operations) = baseline_operations.as_mut() {
            native_operations.value_release = native_operations
                .value_release
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.compare = native_operations
                .compare
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.local_fetch = native_operations
                .local_fetch
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.local_store = native_operations
                .local_store
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.truthy = native_operations
                .truthy
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.type_predicate = native_operations
                .type_predicate
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.stable_length = native_operations
                .stable_length
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.array_fetch = native_operations
                .array_fetch
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.foreach_next = native_operations
                .foreach_next
                .map(NativeHelper::with_inline_runtime_view);
            native_operations.string_predicate = native_operations
                .string_predicate
                .map(NativeHelper::with_inline_runtime_view);
        }
        let (arguments, resume_id) = if region.compile_metadata.tier == NativeCompilerTier::Baseline
            && (fragment.is_none() || inline_fragment_entry)
        {
            lower_baseline_bind_packed_arguments(
                module,
                &mut builder,
                baseline_operations
                    .expect("baseline entry requires baseline operations")
                    .argument_check,
                &region.params,
                region
                    .parameter_locals
                    .len()
                    .saturating_sub(region.params.len()),
                arguments,
                result_out,
                deopt_out,
                resume_id,
                region.function,
            )?
        } else {
            (arguments, resume_id)
        };
        let local_ids = fragment.map_or_else(
            || {
                (0..region.local_count)
                    .map(LocalId::new)
                    .collect::<BTreeSet<_>>()
            },
            |fragment| fragment.fragment.locals.clone(),
        );
        let locals = if let Some(frame) = streaming_state_frame {
            let layout = frame_layout.expect("streaming frame layout");
            local_ids
                .into_iter()
                .map(|local| {
                    Ok((
                        local,
                        NativeLocalStorage::FrameSlot {
                            frame,
                            offset: layout.local_offset(local)?,
                        },
                    ))
                })
                .collect::<Result<NativeLocalMap, CraneliftLoweringError>>()?
        } else {
            local_ids
                .into_iter()
                .map(|local| {
                    (
                        local,
                        NativeLocalStorage::Variable(builder.declare_var(types::I64)),
                    )
                })
                .collect::<NativeLocalMap>()
        };
        let streaming_call_exit = streaming_state_frame
            .filter(|_| {
                owned_blocks.iter().any(|block| {
                    block.instructions.iter().any(|instruction| {
                        matches!(instruction.kind, RegionInstructionKind::NativeCall(_))
                    })
                })
            })
            .map(|_| {
                let block = builder.create_block();
                builder.set_cold_block(block);
                builder.append_block_param(block, types::I32);
                builder.append_block_param(block, types::I64);
                builder.append_block_param(block, types::I32);
                builder.append_block_param(block, types::I64);
                for _ in 0..crate::JIT_DEOPT_LOCAL_MASK_WORDS {
                    builder.append_block_param(block, types::I64);
                }
                NativeStreamingCallExit { block }
            });
        let register_types = region_register_types(region);
        let register_live_in = &register_liveness.block_live_in;
        let transition_register_liveness = &register_liveness.transition_live;
        let register_ids = fragment.map_or_else(
            || {
                (0..region.register_count)
                    .map(RegId::new)
                    .collect::<BTreeSet<_>>()
            },
            |fragment| fragment.fragment.registers.clone(),
        );
        let register_variables = register_ids
            .into_iter()
            .map(|register| {
                let type_ = register_types.get(&register).copied().unwrap_or(types::I64);
                let storage = if let Some(frame) = streaming_state_frame {
                    frame_layout
                        .expect("streaming frame layout")
                        .register_offset_if_present(
                            fragment.expect("streaming fragment definition").fragment.id,
                            register,
                        )
                        .map_or(NativeRegisterStorage::Transient { type_ }, |offset| {
                            NativeRegisterStorage::FrameSlot {
                                frame,
                                offset,
                                type_,
                            }
                        })
                } else {
                    NativeRegisterStorage::Variable(builder.declare_var(type_))
                };
                (register, storage)
            })
            .collect::<NativeRegisterMap>();
        let pending_status = builder.declare_var(types::I32);
        let pending_value = builder.declare_var(types::I64);
        let continue_status = builder
            .ins()
            .iconst(types::I32, i64::from(crate::JitCallStatus::CONTINUE.0));
        let empty_value = builder.ins().iconst(types::I64, 0);
        let native_version =
            u32::from(region.compile_metadata.tier == NativeCompilerTier::Optimizing);
        builder.def_var(pending_status, continue_status);
        builder.def_var(pending_value, empty_value);
        if let Some(frame) = fragment_frame
            && !inline_fragment_entry
        {
            let status = builder.ins().load(
                types::I32,
                MemFlagsData::new(),
                frame,
                frame_layout
                    .expect("fragment frame layout")
                    .pending_status_offset(),
            );
            let value = builder.ins().load(
                types::I64,
                MemFlagsData::new(),
                frame,
                frame_layout
                    .expect("fragment frame layout")
                    .pending_value_offset(),
            );
            builder.def_var(pending_status, status);
            builder.def_var(pending_value, value);
            if let Some(frame) = streaming_state_frame {
                builder.ins().store(
                    MemFlagsData::new(),
                    status,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .pending_status_offset(),
                );
                builder.ins().store(
                    MemFlagsData::new(),
                    value,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .pending_value_offset(),
                );
            }
        } else if let Some(frame) = fragment_frame {
            let layout = frame_layout.expect("inline fragment frame layout");
            builder.ins().store(
                MemFlagsData::new(),
                continue_status,
                frame,
                layout.pending_status_offset(),
            );
            builder.ins().store(
                MemFlagsData::new(),
                empty_value,
                frame,
                layout.pending_value_offset(),
            );
        }
        let uninitialized_value = builder.ins().iconst(
            types::I64,
            crate::jit_encode_constant(crate::JIT_VALUE_UNINITIALIZED),
        );
        for (local, storage) in &locals {
            if let NativeLocalStorage::Variable(variable) = *storage {
                let initial = if matches!(
                    value_flow.local_storage(*local),
                    crate::region_ir::LocalStorageClass::RequestGlobal
                        | crate::region_ir::LocalStorageClass::Superglobal
                ) {
                    lower_trusted_request_local_reference(
                        &mut builder,
                        deopt_out,
                        region.function,
                        *local,
                    )
                } else {
                    uninitialized_value
                };
                builder.def_var(variable, initial);
            }
        }
        if fragment.is_none() {
            for (index, param) in region.parameter_locals.iter().enumerate() {
                let value = builder.ins().load(
                    types::I64,
                    MemFlagsData::new(),
                    arguments,
                    i32::try_from(index.saturating_mul(8)).map_err(|_| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_REGION_ARITY",
                            "packed region argument offset does not fit the native ABI",
                        )
                    })?,
                );
                if value_flow.owns_parameter_at_entry(*param) {
                    lower_optimizing_retain(&mut builder, value, deopt_out);
                }
                define_local_variable(&mut builder, &locals, *param, value)?;
            }
        } else if inline_fragment_entry {
            let frame = fragment_frame.expect("inline fragment frame");
            let layout = frame_layout.expect("inline fragment frame layout");
            for local in layout.local_slots.keys().copied() {
                let initial = if matches!(
                    value_flow.local_storage(local),
                    crate::region_ir::LocalStorageClass::RequestGlobal
                        | crate::region_ir::LocalStorageClass::Superglobal
                ) {
                    lower_trusted_request_local_reference(
                        &mut builder,
                        deopt_out,
                        region.function,
                        local,
                    )
                } else {
                    uninitialized_value
                };
                builder.ins().store(
                    MemFlagsData::new(),
                    initial,
                    frame,
                    layout.local_offset(local)?,
                );
            }
            for (index, local) in region.parameter_locals.iter().enumerate() {
                let value = builder.ins().load(
                    types::I64,
                    MemFlagsData::new(),
                    arguments,
                    i32::try_from(index.saturating_mul(8)).map_err(|_| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_REGION_ARITY",
                            "packed region argument offset does not fit the native ABI",
                        )
                    })?,
                );
                if value_flow.owns_parameter_at_entry(*local) {
                    lower_optimizing_retain(&mut builder, value, deopt_out);
                }
                builder.ins().store(
                    MemFlagsData::new(),
                    value,
                    frame,
                    layout.local_offset(*local)?,
                );
            }
        }
        let handler_resume_blocks = region
            .exception_regions
            .iter()
            .flat_map(|handler| [handler.catch, handler.finally])
            .flatten()
            .filter(|target| {
                fragment.is_none_or(|fragment| fragment.fragment.blocks.contains(target))
            })
            .collect::<std::collections::BTreeSet<_>>();
        let handler_exception_locals = region
            .exception_regions
            .iter()
            .filter_map(|handler| Some((handler.catch?, handler.exception_local?)))
            .fold(
                BTreeMap::<BlockId, std::collections::BTreeSet<LocalId>>::new(),
                |mut locals, (block, local)| {
                    locals.entry(block).or_default().insert(local);
                    locals
                },
            );
        let handler_resume_loaders = handler_resume_blocks
            .iter()
            .map(|target| (*target, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let transition_resume_loaders = owned_blocks
            .iter()
            .flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .filter(|instruction| {
                        instruction_has_native_resume_entry(
                            instruction,
                            region.compile_metadata.tier,
                        )
                    })
                    .map(|instruction| instruction.continuation_id)
                    .chain(
                        block_terminator_has_native_transition(block, region.compile_metadata.tier)
                            .then_some(block.terminator_continuation_id),
                    )
            })
            .filter(|continuation| {
                transition_register_liveness
                    .get(continuation)
                    .is_some_and(|registers| registers.len() <= crate::JIT_DEOPT_MAX_REGISTERS)
            })
            .map(|continuation| (continuation, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let optimizing_block_resume_loaders =
            if region.compile_metadata.tier == NativeCompilerTier::Optimizing {
                {
                    owned_blocks
                        .iter()
                        .map(|block| (block.id, builder.create_block()))
                        .collect::<BTreeMap<_, _>>()
                }
            } else {
                Default::default()
            };
        let osr_entries = region
            .osr_entries()
            .into_iter()
            .filter(|entry| {
                fragment.is_none_or(|fragment| fragment.fragment.blocks.contains(&entry.block))
            })
            .collect::<Vec<_>>();
        let osr_resume_loaders = osr_entries
            .iter()
            .map(|entry| (entry.id, builder.create_block()))
            .collect::<BTreeMap<_, _>>();
        let has_resume_entries = !handler_resume_loaders.is_empty()
            || !suspension_blocks.is_empty()
            || !transition_resume_loaders.is_empty()
            || !optimizing_block_resume_loaders.is_empty()
            || !osr_resume_loaders.is_empty();
        let resume_default = has_resume_entries.then(|| builder.create_block());
        let mut resume_switch = Switch::new();
        let streaming_resume_restore =
            (has_resume_entries && streaming_state_frame.is_some()).then(|| builder.create_block());
        for (target, loader) in &handler_resume_loaders {
            let resume = u128::from(crate::native_handler_resume_id(*target) as u32);
            resume_switch.set_entry(resume, *loader);
        }
        for (continuation, resume_block) in &suspension_blocks {
            let resume = u128::from(crate::native_suspension_resume_id(*continuation) as u32);
            resume_switch.set_entry(resume, *resume_block);
        }
        for (continuation, loader) in &transition_resume_loaders {
            let resume = u128::from(crate::native_transition_resume_id(*continuation) as u32);
            resume_switch.set_entry(resume, *loader);
        }
        for (block, loader) in &optimizing_block_resume_loaders {
            let continuation = region_block_entry_continuation(&region.blocks[block.index()]);
            let resume =
                u128::from(crate::native_optimizing_continuation_resume_id(continuation) as u32);
            resume_switch.set_entry(resume, *loader);
        }
        for (id, loader) in &osr_resume_loaders {
            let resume = u128::from(*id);
            resume_switch.set_entry(resume, *loader);
        }
        let resume_dispatch = if let Some(resume_default) = resume_default {
            let dispatch = builder.create_block();
            builder.set_cold_block(dispatch);
            if let Some(restore) = streaming_resume_restore {
                let is_normal_entry = builder.ins().icmp_imm(IntCC::Equal, resume_id, -1);
                builder
                    .ins()
                    .brif(is_normal_entry, resume_default, &[], restore, &[]);
                builder.switch_to_block(restore);
                builder.set_cold_block(restore);
                let local_restore_done = builder.create_block();
                builder.set_cold_block(local_restore_done);
                emit_streaming_local_restore_loop(
                    &mut builder,
                    pointer_type,
                    resume_state,
                    streaming_state_frame.expect("streaming resume frame"),
                    region.local_count,
                    local_restore_done,
                );
                builder.switch_to_block(local_restore_done);
                let control_status = builder.ins().load(
                    types::I32,
                    MemFlagsData::new(),
                    resume_state,
                    std::mem::offset_of!(crate::JitDeoptState, control_status) as i32,
                );
                let control_value = builder.ins().load(
                    types::I64,
                    MemFlagsData::new(),
                    resume_state,
                    std::mem::offset_of!(crate::JitDeoptState, control_value) as i32,
                );
                builder.def_var(pending_status, control_status);
                builder.def_var(pending_value, control_value);
                let frame = streaming_state_frame.expect("streaming resume frame");
                let layout = frame_layout.expect("streaming resume frame layout");
                builder.ins().store(
                    MemFlagsData::new(),
                    control_status,
                    frame,
                    layout.pending_status_offset(),
                );
                builder.ins().store(
                    MemFlagsData::new(),
                    control_value,
                    frame,
                    layout.pending_value_offset(),
                );
                builder.ins().jump(dispatch, &[]);
            } else {
                builder.ins().jump(dispatch, &[]);
            }
            Some(dispatch)
        } else {
            None
        };

        for target in handler_resume_blocks {
            let loader = handler_resume_loaders[&target];
            builder.switch_to_block(loader);
            builder.set_cold_block(loader);
            let status = builder.ins().load(
                types::I32,
                MemFlagsData::new(),
                resume_state,
                std::mem::offset_of!(crate::JitDeoptState, control_status) as i32,
            );
            let value = builder.ins().load(
                types::I64,
                MemFlagsData::new(),
                resume_state,
                std::mem::offset_of!(crate::JitDeoptState, control_value) as i32,
            );
            builder.def_var(pending_status, status);
            builder.def_var(pending_value, value);
            let target_block = region.blocks.get(target.index()).ok_or_else(|| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_NATIVE_HANDLER",
                    format!("native handler block {} is missing", target.raw()),
                )
            })?;
            let resume_locals = target_block
                .entry_live_locals
                .iter()
                .copied()
                .chain(
                    handler_exception_locals
                        .get(&target)
                        .into_iter()
                        .flatten()
                        .copied(),
                )
                .collect::<std::collections::BTreeSet<_>>();
            if streaming_state_frame.is_none() {
                restore_native_local_state_values(
                    &mut builder,
                    resume_state,
                    &locals,
                    &resume_locals.into_iter().collect::<Vec<_>>(),
                )?;
            }
            // A call-originated throw reaches a handler through the published
            // control value, not through a pre-existing caller local slot.
            // Install that authoritative native throwable directly into the
            // catch local after restoring the caller frame. Restoring the
            // uninitialized snapshot slot here previously replaced every
            // caught Error with NULL.
            if let Some(exception_locals) = handler_exception_locals.get(&target) {
                if matches!(tier_operations, NativeTierOperations::Optimizing { .. }) {
                    // Catch binding can overwrite an object whose destructor
                    // re-enters PHP. Exception paths are cold, so hand the
                    // complete pre-bind frame and pending throwable to the
                    // exact baseline handler entry once instead of embedding
                    // a partial release/store sequence in optimizing code.
                    let transition_locals = target_block
                        .entry_live_locals
                        .iter()
                        .copied()
                        .chain(exception_locals.iter().copied())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();
                    publish_native_continuation_state(
                        &mut builder,
                        deopt_out,
                        region.function,
                        region.local_count,
                        // Catch-bind transitions resume by exact handler block,
                        // including an empty handler with no instruction
                        // continuation of its own.
                        target.raw(),
                        &transition_locals,
                        &locals,
                        native_version,
                    )?;
                    builder.ins().store(
                        MemFlagsData::new(),
                        status,
                        deopt_out,
                        std::mem::offset_of!(crate::JitDeoptState, control_status) as i32,
                    );
                    let detail = builder.ins().iconst(
                        types::I32,
                        i64::from(crate::JIT_NATIVE_CATCH_BIND_TRANSITION_DETAIL),
                    );
                    builder.ins().store(
                        MemFlagsData::new(),
                        detail,
                        deopt_out,
                        std::mem::offset_of!(crate::JitDeoptState, control_reserved) as i32,
                    );
                    builder.ins().store(
                        MemFlagsData::new(),
                        value,
                        deopt_out,
                        std::mem::offset_of!(crate::JitDeoptState, control_value) as i32,
                    );
                    let empty = builder
                        .ins()
                        .iconst(types::I64, crate::jit_encode_constant(u32::MAX));
                    builder
                        .ins()
                        .store(MemFlagsData::new(), empty, result_out, 0);
                    builder.ins().jump(
                        optimizing_baseline_resume
                            .expect("optimizing catch transition requires baseline resume"),
                        &[],
                    );
                    let unreachable = builder.create_block();
                    builder.switch_to_block(unreachable);
                    builder.seal_block(unreachable);
                } else {
                    for local in exception_locals {
                        let current = use_local_variable(&mut builder, &locals, *local)?;
                        let function = builder
                            .ins()
                            .iconst(types::I64, i64::from(region.function.raw()));
                        let local_value = builder.ins().iconst(types::I64, i64::from(local.raw()));
                        let stored = lower_native_value_operation(
                            module,
                            &mut builder,
                            baseline_operations
                                .expect("baseline catch binding requires baseline operations")
                                .local_store,
                            crate::JIT_LOCAL_STORE_MOVE_INPUT,
                            &[current, value, function, local_value],
                            result_out,
                        )?;
                        define_local_variable(&mut builder, &locals, *local, stored)?;
                    }
                }
            }
            builder.ins().jump(cranelift_block(&blocks, target)?, &[]);
        }
        for region_block in &owned_blocks {
            for instruction in &region_block.instructions {
                if let Some(live_registers) = transition_register_liveness
                    .get(&instruction.continuation_id)
                    .filter(|_| {
                        instruction_has_native_resume_entry(
                            instruction,
                            region.compile_metadata.tier,
                        )
                    })
                    .filter(|registers| registers.len() <= crate::JIT_DEOPT_MAX_REGISTERS)
                {
                    let loader = transition_resume_loaders[&instruction.continuation_id];
                    builder.switch_to_block(loader);
                    builder.set_cold_block(loader);
                    if let Some(frame) = streaming_state_frame {
                        let fragment = fragment.expect("streaming transition fragment");
                        let layout = frame_layout.expect("streaming transition frame layout");
                        for (snapshot_slot, register) in live_registers.iter().enumerate() {
                            let source_offset =
                                std::mem::offset_of!(crate::JitDeoptState, registers)
                                    .saturating_add(snapshot_slot.saturating_mul(8));
                            let value = builder.ins().load(
                                types::I64,
                                MemFlagsData::new(),
                                resume_state,
                                source_offset as i32,
                            );
                            builder.ins().store(
                                MemFlagsData::new(),
                                value,
                                frame,
                                layout.register_offset(fragment.fragment.id, *register)?,
                            );
                        }
                        builder
                            .ins()
                            .jump(transition_blocks[&instruction.continuation_id], &[]);
                        continue;
                    }
                    let control_status = builder.ins().load(
                        types::I32,
                        MemFlagsData::new(),
                        resume_state,
                        std::mem::offset_of!(crate::JitDeoptState, control_status) as i32,
                    );
                    let control_value = builder.ins().load(
                        types::I64,
                        MemFlagsData::new(),
                        resume_state,
                        std::mem::offset_of!(crate::JitDeoptState, control_value) as i32,
                    );
                    builder.def_var(pending_status, control_status);
                    builder.def_var(pending_value, control_value);
                    restore_native_local_state_values(
                        &mut builder,
                        resume_state,
                        &locals,
                        &instruction.live_locals,
                    )?;
                    let mut restored_registers = register_variables.clone();
                    for (snapshot_slot, register) in live_registers.iter().enumerate() {
                        let type_ = register_types.get(register).copied().unwrap_or(types::I64);
                        let offset = std::mem::offset_of!(crate::JitDeoptState, registers)
                            .saturating_add(snapshot_slot.saturating_mul(8));
                        let value = builder.ins().load(
                            types::I64,
                            MemFlagsData::new(),
                            resume_state,
                            offset as i32,
                        );
                        let value = if type_ == types::I64 {
                            value
                        } else {
                            builder.ins().ireduce(type_, value)
                        };
                        define_region_register(
                            &mut builder,
                            &register_variables,
                            &mut restored_registers,
                            *register,
                            value,
                        )?;
                    }
                    builder
                        .ins()
                        .jump(transition_blocks[&instruction.continuation_id], &[]);
                }
            }
        }
        for region_block in &owned_blocks {
            let continuation = region_block.terminator_continuation_id;
            let Some(live_registers) = transition_register_liveness
                .get(&continuation)
                .filter(|_| {
                    block_terminator_has_native_transition(
                        region_block,
                        region.compile_metadata.tier,
                    )
                })
                .filter(|registers| registers.len() <= crate::JIT_DEOPT_MAX_REGISTERS)
            else {
                continue;
            };
            let loader = transition_resume_loaders[&continuation];
            builder.switch_to_block(loader);
            builder.set_cold_block(loader);
            if let Some(frame) = streaming_state_frame {
                let fragment = fragment.expect("streaming terminator transition fragment");
                let layout = frame_layout.expect("streaming terminator transition frame layout");
                for (snapshot_slot, register) in live_registers.iter().enumerate() {
                    let source_offset = std::mem::offset_of!(crate::JitDeoptState, registers)
                        .saturating_add(snapshot_slot.saturating_mul(8));
                    let value = builder.ins().load(
                        types::I64,
                        MemFlagsData::new(),
                        resume_state,
                        source_offset as i32,
                    );
                    builder.ins().store(
                        MemFlagsData::new(),
                        value,
                        frame,
                        layout.register_offset(fragment.fragment.id, *register)?,
                    );
                }
            } else {
                restore_native_local_state_values(
                    &mut builder,
                    resume_state,
                    &locals,
                    &region_block.terminator_live_locals,
                )?;
                let mut restored_registers = register_variables.clone();
                for (snapshot_slot, register) in live_registers.iter().enumerate() {
                    let type_ = register_types.get(register).copied().unwrap_or(types::I64);
                    let offset = std::mem::offset_of!(crate::JitDeoptState, registers)
                        .saturating_add(snapshot_slot.saturating_mul(8));
                    let value = builder.ins().load(
                        types::I64,
                        MemFlagsData::new(),
                        resume_state,
                        offset as i32,
                    );
                    let value = if type_ == types::I64 {
                        value
                    } else {
                        builder.ins().ireduce(type_, value)
                    };
                    define_region_register(
                        &mut builder,
                        &register_variables,
                        &mut restored_registers,
                        *register,
                        value,
                    )?;
                }
            }
            builder.ins().jump(transition_blocks[&continuation], &[]);
        }
        for (block_id, loader) in &optimizing_block_resume_loaders {
            let target = region.blocks.get(block_id.index()).ok_or_else(|| {
                CraneliftLoweringError::new(
                    "JIT_CRANELIFT_OPTIMIZING_REENTRY_BLOCK",
                    format!("optimizing re-entry block {} is missing", block_id.raw()),
                )
            })?;
            builder.switch_to_block(*loader);
            builder.set_cold_block(*loader);
            restore_native_local_state_values(
                &mut builder,
                resume_state,
                &locals,
                &target.entry_state_locals,
            )?;
            let mut restored_registers = register_variables.clone();
            for (snapshot_slot, register) in register_live_in
                .get(block_id)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let type_ = register_types.get(register).copied().unwrap_or(types::I64);
                let offset = std::mem::offset_of!(crate::JitDeoptState, registers)
                    .saturating_add(snapshot_slot.saturating_mul(8));
                let value = builder.ins().load(
                    types::I64,
                    MemFlagsData::new(),
                    resume_state,
                    offset as i32,
                );
                let value = if type_ == types::I64 {
                    value
                } else {
                    builder.ins().ireduce(type_, value)
                };
                define_region_register(
                    &mut builder,
                    &register_variables,
                    &mut restored_registers,
                    *register,
                    value,
                )?;
            }
            builder
                .ins()
                .jump(cranelift_block(&blocks, *block_id)?, &[]);
        }
        for osr_entry in &osr_entries {
            let loader = osr_resume_loaders[&osr_entry.id];
            builder.switch_to_block(loader);
            builder.set_cold_block(loader);
            if streaming_state_frame.is_none() {
                restore_native_local_state_values(
                    &mut builder,
                    resume_state,
                    &locals,
                    &osr_entry.live_locals,
                )?;
            }
            builder
                .ins()
                .jump(cranelift_block(&blocks, osr_entry.block)?, &[]);
        }
        if let Some(resume_default) = resume_default {
            builder.switch_to_block(resume_default);
        }
        if let Some(fragment) = fragment {
            let frame = fragment_frame.expect("fragment signature has a native frame");
            let entry_id = fragment_entry_id.expect("fragment signature has an entry id");
            let invalid_entry = builder.create_block();
            let entry_loaders = fragment
                .fragment
                .normal_entries
                .iter()
                .map(|entry| (*entry, builder.create_block()))
                .collect::<BTreeMap<_, _>>();
            let mut entry_switch = Switch::new();
            for (entry, loader) in &entry_loaders {
                entry_switch.set_entry(u128::from(entry.raw()), *loader);
            }
            entry_switch.emit(&mut builder, entry_id, invalid_entry);
            for entry in &fragment.fragment.normal_entries {
                let loader = entry_loaders[entry];
                builder.switch_to_block(loader);
                let entry_block = region.blocks.get(entry.index()).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_ENTRY",
                        format!("fragment entry block {} is missing", entry.raw()),
                    )
                })?;
                if streaming_state_frame.is_none() {
                    let mut entry_locals = entry_block
                        .entry_state_locals
                        .iter()
                        .copied()
                        .collect::<BTreeSet<_>>();
                    if entry.raw() == 0 {
                        entry_locals.extend(region.parameter_locals.iter().copied());
                    }
                    for local in entry_locals {
                        let value = builder.ins().load(
                            types::I64,
                            MemFlagsData::new(),
                            frame,
                            frame_layout
                                .expect("streaming frame layout")
                                .local_offset(local)?,
                        );
                        define_local_variable(&mut builder, &locals, local, value)?;
                    }
                    let mut restored_registers = register_variables.clone();
                    for register in register_live_in.get(entry).into_iter().flatten() {
                        let type_ = register_types.get(register).copied().unwrap_or(types::I64);
                        let value = builder.ins().load(
                            types::I64,
                            MemFlagsData::new(),
                            frame,
                            frame_layout
                                .expect("optimizing fragment frame layout")
                                .register_offset(fragment.fragment.id, *register)?,
                        );
                        let value = if type_ == types::I64 {
                            value
                        } else {
                            builder.ins().ireduce(type_, value)
                        };
                        define_region_register(
                            &mut builder,
                            &register_variables,
                            &mut restored_registers,
                            *register,
                            value,
                        )?;
                    }
                }
                builder.ins().jump(cranelift_block(&blocks, *entry)?, &[]);
            }
            builder.switch_to_block(invalid_entry);
            builder.set_cold_block(invalid_entry);
            let invalid_entry_marker = builder.ins().iconst(types::I32, 0x4652_4147);
            builder.ins().store(
                MemFlagsData::new(),
                invalid_entry_marker,
                deopt_out,
                std::mem::offset_of!(crate::JitDeoptState, control_reserved) as i32,
            );
            let invalid_entry_value = builder.ins().sextend(types::I64, entry_id);
            builder.ins().store(
                MemFlagsData::new(),
                invalid_entry_value,
                deopt_out,
                std::mem::offset_of!(crate::JitDeoptState, control_value) as i32,
            );
            let invalid = builder
                .ins()
                .iconst(types::I32, i64::from(crate::JitCallStatus::RUNTIME_ERROR.0));
            builder.ins().return_(&[invalid]);
        } else {
            builder.ins().jump(normal_entry, &[]);
        }

        let loop_headers = region
            .osr_entries()
            .into_iter()
            .filter(|entry| {
                fragment.is_none_or(|fragment| fragment.fragment.blocks.contains(&entry.block))
            })
            .map(|entry| entry.block)
            .collect::<BTreeSet<_>>();
        for region_block in &owned_blocks {
            let mut registers = register_variables.clone();
            builder.switch_to_block(cranelift_block(&blocks, region_block.id)?);
            if let Some(frame) = streaming_state_frame {
                for register in register_live_in.get(&region_block.id).into_iter().flatten() {
                    let type_ = register_types.get(register).copied().unwrap_or(types::I64);
                    let value = builder.ins().load(
                        types::I64,
                        MemFlagsData::new(),
                        frame,
                        frame_layout
                            .expect("streaming frame layout")
                            .register_offset(
                                fragment.expect("streaming fragment definition").fragment.id,
                                *register,
                            )?,
                    );
                    let value = if type_ == types::I64 {
                        value
                    } else {
                        builder.ins().ireduce(type_, value)
                    };
                    // One load per real block live-in is cheaper than
                    // reloading the same slot at every operand use. The frame
                    // remains authoritative; this cache is discarded at the
                    // next real CFG boundary.
                    registers.insert(*register, NativeRegisterStorage::Cached(value));
                }
            }
            if loop_headers.contains(&region_block.id)
                && let Some(helper) = execution_poll
            {
                let count_visits = builder.create_block();
                let poll = builder.create_block();
                let continue_execution = builder.create_block();
                let runtime_view = lower_active_runtime_view(&mut builder, deopt_out);
                let counter_address = builder.ins().load(
                    types::I64,
                    MemFlagsData::new(),
                    runtime_view,
                    std::mem::offset_of!(crate::JitNativeRuntimeView, poll_counter) as i32,
                );
                let pointer_type = module.target_config().pointer_type();
                let counter_address = if pointer_type == types::I64 {
                    counter_address
                } else {
                    builder.ins().ireduce(pointer_type, counter_address)
                };
                let counter_available = builder.ins().icmp_imm(IntCC::NotEqual, counter_address, 0);
                builder
                    .ins()
                    .brif(counter_available, count_visits, &[], poll, &[]);

                builder.switch_to_block(count_visits);
                let counter =
                    builder
                        .ins()
                        .load(types::I32, MemFlagsData::new(), counter_address, 0);
                let counter = builder.ins().iadd_imm(counter, 1);
                let counter = builder.ins().band_imm(counter, 4095);
                builder
                    .ins()
                    .store(MemFlagsData::new(), counter, counter_address, 0);
                let deadline_check = builder.ins().icmp_imm(IntCC::Equal, counter, 0);
                builder
                    .ins()
                    .brif(deadline_check, poll, &[], continue_execution, &[]);

                builder.switch_to_block(poll);
                let call = call_native_helper(module, &mut builder, helper, &[]);
                let status = builder.inst_results(call)[0];
                require_native_operation_ok(&mut builder, status, helper.terminal_exit()?)?;
                builder.ins().jump(continue_execution, &[]);
                builder.switch_to_block(continue_execution);
            }
            let mut terminated = false;
            for instruction in &region_block.instructions {
                let transition_block = transition_blocks.get(&instruction.continuation_id).copied();
                if let Some(transition_block) = transition_block {
                    builder.ins().jump(transition_block, &[]);
                    builder.switch_to_block(transition_block);
                    // A resume loader may enter this instruction without
                    // executing earlier instructions in the Region block.
                    // The compact frame is authoritative at that boundary;
                    // block-local cached SSA values would not dominate the
                    // resume edge.
                    if streaming_state_frame.is_some() {
                        registers = register_variables.clone();
                    }
                }
                builder.set_srcloc(ir::SourceLoc::new(
                    instruction.continuation_id.saturating_add(1),
                ));
                if let Some(target) = tail_forwards
                    .get(&(region.function, instruction.continuation_id))
                    .and_then(|target| functions.get(target))
                {
                    let callee = module.declare_func_in_func(*target, builder.func);
                    builder.ins().return_call(
                        callee,
                        &[
                            runtime,
                            arguments,
                            result_out,
                            deopt_out,
                            resume_id,
                            resume_state,
                        ],
                    );
                    terminated = true;
                    break;
                }
                let transition_live_registers = transition_register_liveness
                    .get(&instruction.continuation_id)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                match tier_operations {
                    NativeTierOperations::Optimizing { operations } => {
                        lower_optimizing_region_instruction(
                            module,
                            &mut builder,
                            &register_variables,
                            &suspension_blocks,
                            &blocks,
                            &locals,
                            &mut registers,
                            instruction,
                            transition_live_registers,
                            constants,
                            value_flow,
                            inline_constants,
                            function_params,
                            external_function_signatures,
                            runtime,
                            result_out,
                            deopt_out,
                            resume_state,
                            pending_status,
                            pending_value,
                            region.function,
                            region.local_count,
                            region.flags.is_top_level,
                            native_version,
                            unit_identity,
                            operations.with_runtime(runtime),
                            optimizing_baseline_resume
                                .expect("optimizing instruction requires baseline resume"),
                        )
                        .map(|emitted| {
                            production_lowering.push(crate::JitProductionLoweringMetadata {
                                function: region.function,
                                continuation_id: instruction.continuation_id,
                                operation: crate::region_ir::baseline_instruction_lowering(
                                    &instruction.source_kind,
                                )
                                .variant
                                .to_owned(),
                                class: emitted.class,
                                operation_local_transition: emitted.operation_local_transition,
                            });
                        })
                    }
                    NativeTierOperations::Baseline { .. } => lower_baseline_region_instruction(
                        module,
                        &mut builder,
                        functions,
                        inline_constants,
                        function_params,
                        external_function_signatures,
                        native_call_helper,
                        native_dynamic_code_helper,
                        baseline_operations
                            .expect("baseline instruction requires baseline operations"),
                        baseline_value_release_commit,
                        &register_variables,
                        &blocks,
                        &suspension_blocks,
                        &locals,
                        &mut registers,
                        region_block.source_block,
                        instruction,
                        transition_live_registers,
                        constants,
                        value_flow,
                        streaming_call_exit,
                        result_out,
                        deopt_out,
                        resume_state,
                        pending_status,
                        pending_value,
                        region.function,
                        region.return_type.is_some(),
                        region.local_count,
                        native_version,
                        region.flags.is_top_level,
                        &region.locals,
                        unit_identity,
                        pointer_type,
                    ),
                }
                .map_err(|error| {
                    CraneliftLoweringError::new(
                        error.code,
                        format!(
                            "{} in Region block {} continuation {} ({:?})",
                            error.detail,
                            region_block.id.raw(),
                            instruction.continuation_id,
                            instruction.source_kind,
                        ),
                    )
                })?;
                maximum_temporary_cache_entries = maximum_temporary_cache_entries.max(
                    registers
                        .values()
                        .filter(|storage| matches!(storage, NativeRegisterStorage::Cached(_)))
                        .count(),
                );
                if matches!(instruction.kind, RegionInstructionKind::RuntimeFatal { .. }) {
                    terminated = true;
                    break;
                }
            }
            if terminated {
                continue;
            }
            if let Some(transition_block) =
                transition_blocks.get(&region_block.terminator_continuation_id)
            {
                builder.ins().jump(*transition_block, &[]);
                builder.switch_to_block(*transition_block);
                // The normal edge and a resume loader both enter this block.
                // Values cached while lowering the normal predecessor do not
                // dominate the resume edge; the compact frame does.
                if streaming_state_frame.is_some() {
                    registers = register_variables.clone();
                }
            }
            builder.set_srcloc(ir::SourceLoc::new(
                region_block.terminator_continuation_id.saturating_add(1),
            ));
            // Streaming definitions store through to every externally live
            // frame slot immediately. Re-emitting all successor live-ins here
            // duplicated stores on every CFG edge and inflated both baseline
            // code and execution traffic; successor blocks already reload the
            // authoritative slots above.
            match tier_operations {
                NativeTierOperations::Optimizing { operations } => {
                    let value_release_validate = module
                        .declare_func_in_func(operations.value_release_validate, builder.func);
                    let value_release_commit =
                        module.declare_func_in_func(operations.value_release_commit, builder.func);
                    lower_optimizing_region_terminator(
                        module,
                        &mut builder,
                        &blocks,
                        &locals,
                        &registers,
                        result_out,
                        deopt_out,
                        region.function,
                        region.local_count,
                        region_block.terminator_continuation_id,
                        &region_block.terminator_live_locals,
                        transition_register_liveness
                            .get(&region_block.terminator_continuation_id)
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        native_version,
                        value_release_validate,
                        value_release_commit,
                        operations.numeric_string,
                        operations
                            .string_cast
                            .map(|helper| helper.with_runtime(runtime)),
                        region.strict_types,
                        region.return_type.as_ref(),
                        &region_block.terminator,
                        constants,
                        value_flow,
                        optimizing_baseline_resume
                            .expect("optimizing terminator requires baseline resume"),
                    )
                    .map(|emitted| {
                        production_lowering.push(crate::JitProductionLoweringMetadata {
                            function: region.function,
                            continuation_id: region_block.terminator_continuation_id,
                            operation: crate::region_ir::baseline_terminator_lowering(
                                &region_block.source_terminator,
                            )
                            .variant
                            .to_owned(),
                            class: emitted.class,
                            operation_local_transition: emitted.operation_local_transition,
                        });
                    })
                }
                NativeTierOperations::Baseline { .. } => lower_region_terminator(
                    &mut builder,
                    &terminator_blocks,
                    &locals,
                    &registers,
                    result_out,
                    deopt_out,
                    pending_status,
                    pending_value,
                    module,
                    baseline_operations.expect("baseline terminator requires baseline operations"),
                    region.function,
                    region.local_count,
                    region_block.terminator_continuation_id,
                    native_version,
                    region.return_type.is_some(),
                    &region_block.terminator,
                    constants,
                    value_flow,
                ),
            }?;
        }
        if let Some(fragment) = fragment {
            let frame = fragment_frame.expect("fragment signature has a native frame");
            for target in &fragment.fragment.external_targets {
                builder.switch_to_block(cranelift_block(&blocks, *target)?);
                let target_block = region.blocks.get(target.index()).ok_or_else(|| {
                    CraneliftLoweringError::new(
                        "JIT_CRANELIFT_FRAGMENT_EXIT_TARGET",
                        format!("fragment exit target {} is missing", target.raw()),
                    )
                })?;
                if streaming_state_frame.is_none() {
                    for local in &target_block.entry_state_locals {
                        let value = use_local_variable(&mut builder, &locals, *local)?;
                        builder.ins().store(
                            MemFlagsData::new(),
                            value,
                            frame,
                            frame_layout
                                .expect("fragment frame layout")
                                .local_offset(*local)?,
                        );
                    }
                }
                if streaming_state_frame.is_none() {
                    for register in register_live_in.get(target).into_iter().flatten() {
                        let value =
                            use_region_register(&mut builder, &register_variables, *register)?;
                        let value = if builder.func.dfg.value_type(value) == types::I64 {
                            value
                        } else {
                            builder.ins().uextend(types::I64, value)
                        };
                        builder.ins().store(
                            MemFlagsData::new(),
                            value,
                            frame,
                            frame_layout
                                .expect("fragment frame layout")
                                .register_offset(fragment.fragment.id, *register)?,
                        );
                    }
                }
                let status = builder.use_var(pending_status);
                let value = builder.use_var(pending_value);
                builder.ins().store(
                    MemFlagsData::new(),
                    status,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .pending_status_offset(),
                );
                builder.ins().store(
                    MemFlagsData::new(),
                    value,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .pending_value_offset(),
                );
                let target_fragment = fragment.layout.block_owner[target];
                let callee =
                    module.declare_func_in_func(fragment.functions[&target_fragment], builder.func);
                let no_resume = builder.ins().iconst(types::I32, -1);
                let entry = builder.ins().iconst(types::I32, i64::from(target.raw()));
                builder.ins().store(
                    MemFlagsData::new(),
                    entry,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .entry_id_offset(),
                );
                builder.ins().store(
                    MemFlagsData::new(),
                    no_resume,
                    frame,
                    frame_layout
                        .expect("fragment frame layout")
                        .resume_id_offset(),
                );
                builder.ins().return_call(callee, &[runtime, frame]);
            }
        }
        if let (Some(streaming_call_exit), Some(frame)) =
            (streaming_call_exit, streaming_state_frame)
        {
            builder.switch_to_block(streaming_call_exit.block);
            let params = builder.block_params(streaming_call_exit.block).to_vec();
            let status = params[0];
            let value = params[1];
            let continuation = params[2];
            let suspension_link = params[3];
            let store_i32 = |builder: &mut FunctionBuilder<'_>, offset: usize, value: ir::Value| {
                builder
                    .ins()
                    .store(MemFlagsData::new(), value, deopt_out, offset as i32);
            };
            let function_id = builder
                .ins()
                .iconst(types::I32, i64::from(region.function.raw()));
            let slot_count = builder
                .ins()
                .iconst(types::I32, i64::from(region.local_count));
            let native_version_value = builder.ins().iconst(types::I32, i64::from(native_version));
            store_i32(
                &mut builder,
                std::mem::offset_of!(crate::JitDeoptState, function_id),
                function_id,
            );
            store_i32(
                &mut builder,
                std::mem::offset_of!(crate::JitDeoptState, continuation_id),
                continuation,
            );
            store_i32(
                &mut builder,
                std::mem::offset_of!(crate::JitDeoptState, slot_count),
                slot_count,
            );
            store_i32(
                &mut builder,
                std::mem::offset_of!(crate::JitDeoptState, native_version),
                native_version_value,
            );
            for (word, mask) in params[4..].iter().copied().enumerate() {
                builder.ins().store(
                    MemFlagsData::new(),
                    mask,
                    deopt_out,
                    std::mem::offset_of!(crate::JitDeoptState, initialized_mask)
                        .saturating_add(word.saturating_mul(8)) as i32,
                );
            }
            builder
                .ins()
                .store(MemFlagsData::new(), value, result_out, 0);
            publish_native_fiber_suspension_link(&mut builder, deopt_out, suspension_link);
            let finished = builder.create_block();
            builder.set_cold_block(finished);
            emit_streaming_local_snapshot_loop(
                &mut builder,
                pointer_type,
                deopt_out,
                frame,
                region.local_count,
                finished,
            );
            builder.switch_to_block(finished);
            builder.ins().return_(&[status]);
        }
        if let (Some(dispatch), Some(resume_default)) = (resume_dispatch, resume_default) {
            builder.switch_to_block(dispatch);
            resume_switch.emit(&mut builder, resume_id, resume_default);
        }
        if let Some(baseline_resume) = optimizing_baseline_resume {
            builder.switch_to_block(baseline_resume);
            builder.set_cold_block(baseline_resume);
            let runtime_view = lower_active_runtime_view(&mut builder, deopt_out);
            let baseline_entries = builder.ins().load(
                pointer_type,
                MemFlagsData::new(),
                runtime_view,
                std::mem::offset_of!(crate::JitNativeRuntimeView, trusted_function_entries) as i32,
            );
            let rejected_function = builder.ins().load(
                types::I32,
                MemFlagsData::new(),
                deopt_out,
                std::mem::offset_of!(crate::JitDeoptState, function_id) as i32,
            );
            let rejected_function = builder.ins().uextend(pointer_type, rejected_function);
            let rejected_offset = builder
                .ins()
                .imul_imm(rejected_function, i64::from(pointer_type.bytes()));
            let rejected_entry = builder.ins().iadd(baseline_entries, rejected_offset);
            let rejected_address =
                builder
                    .ins()
                    .atomic_load(pointer_type, MemFlagsData::new(), rejected_entry);
            let resume_published = builder.create_block();
            let resume_unavailable = builder.create_block();
            let published = builder.ins().icmp_imm(IntCC::NotEqual, rejected_address, 0);
            builder
                .ins()
                .brif(published, resume_published, &[], resume_unavailable, &[]);

            builder.switch_to_block(resume_unavailable);
            let recompile = builder.ins().iconst(
                types::I32,
                i64::from(crate::JitCallStatus::RECOMPILE_REQUESTED.0),
            );
            builder.ins().return_(&[recompile]);

            builder.switch_to_block(resume_published);
            let rejected_continuation = builder.ins().load(
                types::I32,
                MemFlagsData::new(),
                deopt_out,
                std::mem::offset_of!(crate::JitDeoptState, continuation_id) as i32,
            );
            let transition_resume_id = builder.ins().bor_imm(
                rejected_continuation,
                i64::from(crate::JIT_NATIVE_TRANSITION_RESUME_TAG),
            );
            let handler_resume_id = builder.ins().bor_imm(
                rejected_continuation,
                i64::from(crate::JIT_NATIVE_HANDLER_RESUME_TAG),
            );
            let control_reserved = builder.ins().load(
                types::I32,
                MemFlagsData::new(),
                deopt_out,
                std::mem::offset_of!(crate::JitDeoptState, control_reserved) as i32,
            );
            let catch_bind = builder.ins().icmp_imm(
                IntCC::Equal,
                control_reserved,
                i64::from(crate::JIT_NATIVE_CATCH_BIND_TRANSITION_DETAIL),
            );
            let rejected_resume_id =
                builder
                    .ins()
                    .select(catch_bind, handler_resume_id, transition_resume_id);
            let signature = builder.import_signature(native_php_entry_signature(module));
            let resumed = builder.ins().call_indirect(
                signature,
                rejected_address,
                &[
                    runtime,
                    arguments,
                    result_out,
                    deopt_out,
                    rejected_resume_id,
                    deopt_out,
                ],
            );
            let resumed_status = builder.inst_results(resumed)[0];
            builder.ins().return_(&[resumed_status]);
        }
        builder.switch_to_block(terminal_exit);
        let terminal_status = builder.block_params(terminal_exit)[0];
        let terminal_value = builder.block_params(terminal_exit)[1];
        builder
            .ins()
            .store(MemFlagsData::new(), terminal_value, result_out, 0);
        builder.ins().return_(&[terminal_status]);
        builder.seal_all_blocks();
        builder.finalize();
    }
    let mut pre_regalloc = match validate_pre_regalloc_structure(
        &ctx.func,
        region,
        fragment.map(|fragment| fragment.fragment.id),
    ) {
        Ok(metrics) => metrics,
        Err(error) => {
            module.clear_context(ctx);
            return Err(error);
        }
    };
    let source_instructions = fragment.map_or_else(
        || {
            region
                .blocks
                .iter()
                .map(|block| block.instructions.len())
                .sum()
        },
        |fragment| {
            fragment
                .fragment
                .blocks
                .iter()
                .map(|block| region.blocks[block.index()].instructions.len())
                .sum::<usize>()
        },
    );
    if source_instructions != 0 {
        pre_regalloc.loads_per_source_instruction_milli = pre_regalloc
            .loads
            .saturating_mul(1_000)
            .div_ceil(source_instructions);
        pre_regalloc.stores_per_source_instruction_milli = pre_regalloc
            .stores
            .saturating_mul(1_000)
            .div_ceil(source_instructions);
    }
    let verifier_flags = settings::Flags::new(settings::builder());
    if let Err(error) = verify_function(&ctx.func, &verifier_flags) {
        let error = CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_VERIFIER",
            format!("Cranelift verifier rejected executable Region IR: {error}"),
        );
        module.clear_context(ctx);
        return Err(error);
    }
    let clif_blocks = ctx.func.layout.blocks().count();
    if preflight_only {
        let lowered_function = std::mem::replace(&mut ctx.func, ir::Function::new());
        module.clear_context(ctx);
        return Ok(DefinedRegionFunction {
            lowered_function: Some(lowered_function),
            code: Vec::new(),
            clif_blocks,
            alignment: 1,
            relocations: Vec::new(),
            native_pc_ranges: Vec::new(),
            native_stack_bytes: 0,
            pre_regalloc,
            maximum_temporary_cache_entries,
            production_lowering,
        });
    }
    if let Err(error) = module.define_function(func_id, ctx) {
        let error = CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_DEFINE",
            format!("failed to define native function: {error}"),
        );
        module.clear_context(ctx);
        return Err(error);
    }
    let compiled = ctx.compiled_code().ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_CACHE_CODE",
            "Cranelift returned no compiled machine-code buffer",
        )
    })?;
    let native_stack_bytes = compiled
        .buffer
        .frame_layout()
        .map_or(0, |layout| layout.frame_to_fp_offset);
    if native_stack_bytes > MAX_NATIVE_SPILL_FRAME_BYTES {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_NATIVE_STACK_LIMIT",
            format!(
                "function {} requires {native_stack_bytes} native stack bytes; limit is {MAX_NATIVE_SPILL_FRAME_BYTES}",
                region.function_name
            ),
        ));
    }
    let code = compiled.code_buffer().to_vec();
    let alignment = u64::from(compiled.buffer.alignment)
        .max(module.isa().function_alignment().minimum as u64)
        .max(module.isa().symbol_alignment());
    let relocations = compiled
        .buffer
        .relocs()
        .iter()
        .map(|relocation| {
            capture_relocation(
                module,
                ModuleReloc::from_mach_reloc(relocation, &ctx.func, func_id),
                functions,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let native_pc_ranges = ctx
        .compiled_code()
        .into_iter()
        .flat_map(|compiled| compiled.buffer.get_srclocs_sorted())
        .filter_map(|range| {
            let source = range.loc.bits();
            (source != 0 && source != u32::MAX).then_some(crate::JitNativePcRange {
                function: region.function,
                start: range.start,
                end: range.end,
                continuation_id: source - 1,
            })
        })
        .collect();
    module.clear_context(ctx);
    Ok(DefinedRegionFunction {
        lowered_function: None,
        code,
        clif_blocks,
        alignment,
        relocations,
        native_pc_ranges,
        native_stack_bytes,
        pre_regalloc,
        maximum_temporary_cache_entries,
        production_lowering,
    })
}

include!("executable_region/direct_value_support.rs");

#[allow(clippy::too_many_arguments)]
fn region_graph_metadata<'a>(
    root: FunctionId,
    root_local_count: u32,
    regions: impl Iterator<Item = &'a RegionGraph>,
    native_pc_ranges: Vec<crate::JitNativePcRange>,
    function_entries: Vec<crate::JitNativeFunctionEntryMetadata>,
    root_register_liveness: Option<&NativeRegisterLiveness>,
    value_flows: &BTreeMap<FunctionId, ExecutableValueFlow>,
    mut emitted_production_lowering: Vec<crate::JitProductionLoweringMetadata>,
) -> crate::JitRegionStateMetadata {
    let regions = regions.collect::<Vec<_>>();
    emitted_production_lowering.sort_by_key(|entry| (entry.function, entry.continuation_id));
    emitted_production_lowering.dedup_by_key(|entry| (entry.function, entry.continuation_id));
    let transition_liveness = regions
        .iter()
        .map(|region| {
            let liveness = root_register_liveness
                .filter(|_| region.function == root)
                .map_or_else(
                    || NativeRegisterLiveness::analyze(region).transition_live,
                    |liveness| liveness.transition_live.clone(),
                );
            (region.function, liveness)
        })
        .collect::<BTreeMap<_, _>>();
    let continuations = regions
        .iter()
        .flat_map(|region| {
            region.blocks.iter().flat_map(move |block| {
                block
                    .instructions
                    .iter()
                    .map(move |instruction| crate::JitContinuationMetadata {
                        id: instruction.continuation_id,
                        function: region.function,
                        block: block.id,
                        instruction: Some(instruction.id),
                        span: instruction.span,
                        live_locals: instruction.live_locals.clone(),
                    })
                    .chain(std::iter::once(crate::JitContinuationMetadata {
                        id: block.terminator_continuation_id,
                        function: region.function,
                        block: block.id,
                        instruction: None,
                        span: block.terminator_span,
                        live_locals: block.terminator_live_locals.clone(),
                    }))
            })
        })
        .collect();
    let osr_entries = regions
        .iter()
        .flat_map(|region| {
            region
                .osr_entries()
                .into_iter()
                .map(move |entry| crate::JitOsrEntryMetadata {
                    id: entry.id,
                    function: region.function,
                    block: entry.block,
                    continuation_id: entry.continuation_id,
                    live_locals: entry.live_locals,
                })
        })
        .collect();
    let root_direct_call_sites = function_entries
        .iter()
        .find(|entry| entry.function == root)
        .map_or(0, |entry| entry.direct_call_sites);
    let root_direct_method_call_sites = function_entries
        .iter()
        .find(|entry| entry.function == root)
        .map_or(0, |entry| entry.direct_method_call_sites);
    let root_inlining = function_entries
        .iter()
        .find(|entry| entry.function == root)
        .map(|entry| {
            (
                entry.inlined_call_sites,
                entry.inline_bytes_added,
                entry.tail_call_sites,
                entry.inline_rejected_by_reason.clone(),
            )
        })
        .unwrap_or_default();
    let direct_callees = regions
        .iter()
        .flat_map(|region| region.direct_callees())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    crate::JitRegionStateMetadata {
        local_count: root_local_count,
        compiler_tier: regions
            .first()
            .map(|region| region.compile_metadata.tier)
            .unwrap_or_default(),
        native_version: u32::from(
            regions.first().is_some_and(|region| {
                region.compile_metadata.tier == NativeCompilerTier::Optimizing
            }),
        ),
        compiled_to_compiled_call_sites: root_direct_call_sites,
        compiled_to_compiled_method_call_sites: root_direct_method_call_sites,
        inlined_call_sites: root_inlining.0,
        inline_bytes_added: root_inlining.1,
        tail_call_sites: root_inlining.2,
        inline_rejected_by_reason: root_inlining.3,
        direct_callees,
        continuations,
        native_pc_ranges,
        osr_entries,
        exception_handlers: regions
            .iter()
            .flat_map(|region| {
                region.exception_regions.iter().filter_map(move |handler| {
                    let enter_continuation = region
                        .blocks
                        .get(handler.block.index())?
                        .instructions
                        .iter()
                        .find(|instruction| instruction.id == handler.instruction)?
                        .continuation_id;
                    Some(crate::JitExceptionHandlerMetadata {
                        function: region.function,
                        enter_continuation,
                        protected_blocks: handler.protected_blocks.clone(),
                        catch: handler.catch,
                        catch_types: handler.catch_types.clone(),
                        finally: handler.finally,
                        after: handler.after,
                        exception_local: handler.exception_local,
                    })
                })
            })
            .collect(),
        safepoints: regions
            .iter()
            .flat_map(|region| {
                region.blocks.iter().flat_map(move |block| {
                    block
                        .instructions
                        .iter()
                        .filter(move |instruction| {
                            crate::region_ir::baseline_instruction_lowering(
                                &instruction.source_kind,
                            )
                            .requires_safepoint
                        })
                        .map(move |instruction| crate::JitNativeSafepointMetadata {
                            function: region.function,
                            continuation_id: instruction.continuation_id,
                            baseline_frame_slots: instruction.live_locals.clone(),
                            optimized_roots_required: region.compile_metadata.tier
                                == NativeCompilerTier::Optimizing,
                        })
                })
            })
            .collect(),
        suspensions: regions
            .iter()
            .flat_map(|region| {
                let liveness = &transition_liveness[&region.function];
                let value_flow = &value_flows[&region.function];
                region.blocks.iter().flat_map(move |block| {
                    block.instructions.iter().filter_map(move |instruction| {
                        let RegionInstructionKind::NativeSuspend(suspend) = &instruction.kind
                        else {
                            return None;
                        };
                        let kind = match suspend {
                            RegionNativeSuspend::GeneratorYield { .. } => {
                                crate::JitNativeSuspendKind::GENERATOR_YIELD
                            }
                            RegionNativeSuspend::GeneratorDelegate { .. } => {
                                crate::JitNativeSuspendKind::GENERATOR_DELEGATE
                            }
                            RegionNativeSuspend::FiberSuspend { .. } => {
                                crate::JitNativeSuspendKind::FIBER_SUSPEND
                            }
                        };
                        let live_registers = liveness
                            .get(&instruction.continuation_id)
                            .cloned()
                            .unwrap_or_default();
                        let owned_locals = instruction
                            .live_locals
                            .iter()
                            .copied()
                            .filter(|local| {
                                value_flow.local_storage(*local).is_native_frame_local()
                            })
                            .collect();
                        let owned_registers = live_registers
                            .iter()
                            .copied()
                            .filter(|register| {
                                crate::region_ir::value_release_required(
                                    value_flow.register_fact(*register),
                                )
                            })
                            .collect();
                        Some(crate::JitNativeSuspensionMetadata {
                            function: region.function,
                            continuation_id: instruction.continuation_id,
                            resume_id: crate::native_suspension_resume_id(
                                instruction.continuation_id,
                            ),
                            kind,
                            span: instruction.span,
                            live_locals: instruction.live_locals.clone(),
                            owned_locals,
                            live_registers,
                            owned_registers,
                            owning_generation_required: true,
                        })
                    })
                })
            })
            .collect(),
        dynamic_code: regions
            .iter()
            .flat_map(|region| {
                region.blocks.iter().flat_map(move |block| {
                    block.instructions.iter().filter_map(move |instruction| {
                        let RegionInstructionKind::NativeDynamicCode(operation) = &instruction.kind
                        else {
                            return None;
                        };
                        let (kind, declared_function) = match operation {
                            RegionNativeDynamicCode::Include { kind, .. } => (
                                match kind {
                                    php_ir::instruction::IncludeKind::Include => {
                                        crate::JitNativeDynamicCodeKind::INCLUDE
                                    }
                                    php_ir::instruction::IncludeKind::IncludeOnce => {
                                        crate::JitNativeDynamicCodeKind::INCLUDE_ONCE
                                    }
                                    php_ir::instruction::IncludeKind::Require => {
                                        crate::JitNativeDynamicCodeKind::REQUIRE
                                    }
                                    php_ir::instruction::IncludeKind::RequireOnce => {
                                        crate::JitNativeDynamicCodeKind::REQUIRE_ONCE
                                    }
                                },
                                None,
                            ),
                            RegionNativeDynamicCode::Eval { .. } => {
                                (crate::JitNativeDynamicCodeKind::EVAL, None)
                            }
                            RegionNativeDynamicCode::DeclareFunction { function, .. } => (
                                crate::JitNativeDynamicCodeKind::DECLARE_FUNCTION,
                                Some(*function),
                            ),
                            RegionNativeDynamicCode::DeclareClass { .. } => {
                                (crate::JitNativeDynamicCodeKind::DECLARE_CLASS, None)
                            }
                            RegionNativeDynamicCode::RegisterConstant { .. } => {
                                (crate::JitNativeDynamicCodeKind::REGISTER_CONSTANT, None)
                            }
                            RegionNativeDynamicCode::EmitDiagnostic => {
                                (crate::JitNativeDynamicCodeKind::EMIT_DIAGNOSTIC, None)
                            }
                            RegionNativeDynamicCode::MakeClosure { function, .. } => (
                                crate::JitNativeDynamicCodeKind::MAKE_CLOSURE,
                                Some(*function),
                            ),
                        };
                        Some(crate::JitNativeDynamicCodeMetadata {
                            function: region.function,
                            continuation_id: instruction.continuation_id,
                            kind,
                            declared_function,
                            span: instruction.span,
                            process_cache: true,
                            restart_cache: true,
                        })
                    })
                })
            })
            .collect(),
        native_transitions: regions
            .iter()
            .flat_map(|region| {
                let liveness = &transition_liveness[&region.function];
                let value_flow = &value_flows[&region.function];
                let mut transitions = region
                    .blocks
                    .iter()
                    .flat_map(|block| &block.instructions)
                    .filter_map(|instruction| {
                        if !instruction_has_native_transition(
                            instruction,
                            region.compile_metadata.tier,
                        ) {
                            return None;
                        }
                        let live_registers = liveness.get(&instruction.continuation_id)?;
                        (live_registers.len() <= crate::JIT_DEOPT_MAX_REGISTERS).then(|| {
                            let owned_locals = instruction
                                .live_locals
                                .iter()
                                .copied()
                                .filter(|local| {
                                    value_flow.local_storage(*local).is_native_frame_local()
                                })
                                .collect();
                            let owned_registers = live_registers
                                .iter()
                                .copied()
                                .filter(|register| {
                                    crate::region_ir::value_release_required(
                                        value_flow.register_fact(*register),
                                    )
                                })
                                .collect();
                            crate::JitNativeTransitionMetadata {
                                function: region.function,
                                native_version: u32::from(
                                    region.compile_metadata.tier == NativeCompilerTier::Optimizing,
                                ),
                                continuation_id: instruction.continuation_id,
                                resume_id: crate::native_transition_resume_id(
                                    instruction.continuation_id,
                                ),
                                span: instruction.span,
                                live_locals: instruction.live_locals.clone(),
                                live_registers: live_registers.clone(),
                                owned_locals,
                                owned_registers,
                                result_register: region_instruction_result_register(
                                    &instruction.kind,
                                ),
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                transitions.extend(region.blocks.iter().filter_map(|block| {
                    if !block_terminator_has_native_transition(block, region.compile_metadata.tier)
                    {
                        return None;
                    }
                    let continuation_id = block.terminator_continuation_id;
                    let live_registers = liveness.get(&continuation_id)?;
                    (live_registers.len() <= crate::JIT_DEOPT_MAX_REGISTERS).then(|| {
                        let owned_locals = block
                            .terminator_live_locals
                            .iter()
                            .copied()
                            .filter(|local| {
                                value_flow.local_storage(*local).is_native_frame_local()
                            })
                            .collect();
                        let owned_registers = live_registers
                            .iter()
                            .copied()
                            .filter(|register| {
                                crate::region_ir::value_release_required(
                                    value_flow.register_fact(*register),
                                )
                            })
                            .collect();
                        crate::JitNativeTransitionMetadata {
                            function: region.function,
                            native_version: u32::from(
                                region.compile_metadata.tier == NativeCompilerTier::Optimizing,
                            ),
                            continuation_id,
                            resume_id: crate::native_transition_resume_id(continuation_id),
                            span: block.terminator_span,
                            live_locals: block.terminator_live_locals.clone(),
                            live_registers: live_registers.clone(),
                            owned_locals,
                            owned_registers,
                            result_register: None,
                        }
                    })
                }));
                transitions
            })
            .collect(),
        production_lowering: emitted_production_lowering,
        function_entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_native_closure_construction_does_not_require_a_trampoline() {
        let mut builder = php_ir::IrBuilder::new(php_ir::UnitId::new(7_001));
        let file = builder.add_file("direct-native-closure.php");
        let span = php_ir::IrSpan::new(file, 0, 1);

        let closure = builder.start_function(
            "direct_native_closure_body",
            php_ir::FunctionFlags::default(),
            span,
        );
        let closure_block = builder.append_block(closure);
        builder.terminate_return(closure, closure_block, None, span);

        let factory = builder.start_function(
            "direct_native_closure_factory",
            php_ir::FunctionFlags::default(),
            span,
        );
        let factory_block = builder.append_block(factory);
        let result = builder.alloc_register(factory);
        builder.emit(
            factory,
            factory_block,
            php_ir::InstructionKind::MakeClosure {
                dst: result,
                function: closure,
                captures: Vec::new(),
            },
            span,
        );
        builder.terminate_return(
            factory,
            factory_block,
            Some(php_ir::Operand::Register(result)),
            span,
        );

        let unit = builder.finish();
        let function = &unit.functions[factory.index()];
        assert!(!ir_function_requires_non_reference_trampoline(function));
        assert!(!ir_function_requires_trampoline(function));
    }

    #[test]
    fn native_exception_control_does_not_require_a_trampoline() {
        let mut builder = php_ir::IrBuilder::new(php_ir::UnitId::new(7_002));
        let file = builder.add_file("direct-native-exception.php");
        let span = php_ir::IrSpan::new(file, 0, 1);
        let function = builder.start_function(
            "direct_native_exception",
            php_ir::FunctionFlags::default(),
            span,
        );
        let block = builder.append_block(function);
        let message = builder.intern_constant(php_ir::IrConstant::String("native".to_owned()));
        let exception = builder.alloc_register(function);
        builder.emit(
            function,
            block,
            php_ir::InstructionKind::MakeException {
                dst: exception,
                class_name: "runtimeexception".to_owned(),
                message: php_ir::Operand::Constant(message),
            },
            span,
        );
        builder.emit(
            function,
            block,
            php_ir::InstructionKind::Throw {
                value: php_ir::Operand::Register(exception),
            },
            span,
        );
        builder.terminate_return(function, block, None, span);

        let unit = builder.finish();
        let function = &unit.functions[function.index()];
        assert!(!ir_function_requires_non_reference_trampoline(function));
        assert!(!ir_function_requires_trampoline(function));
    }

    #[test]
    fn debug_backtrace_requires_a_complete_native_frame_trampoline() {
        let mut builder = php_ir::IrBuilder::new(php_ir::UnitId::new(7_004));
        let file = builder.add_file("native-debug-backtrace-frame.php");
        let span = php_ir::IrSpan::new(file, 0, 1);
        let function = builder.start_function(
            "native_debug_backtrace_frame",
            php_ir::FunctionFlags::default(),
            span,
        );
        let block = builder.append_block(function);
        let result = builder.alloc_register(function);
        builder.emit(
            function,
            block,
            php_ir::InstructionKind::CallFunction {
                dst: result,
                name: "debug_backtrace".to_owned(),
                args: Vec::new(),
            },
            span,
        );
        builder.terminate_return(
            function,
            block,
            Some(php_ir::Operand::Register(result)),
            span,
        );

        let unit = builder.finish();
        let function = &unit.functions[function.index()];
        assert!(ir_function_requires_non_reference_trampoline(function));
        assert!(ir_function_requires_trampoline(function));
    }

    #[test]
    fn catch_admission_is_distinct_from_finally_only_control() {
        let mut builder = php_ir::IrBuilder::new(php_ir::UnitId::new(7_003));
        let file = builder.add_file("native-catch-admission.php");
        let span = php_ir::IrSpan::new(file, 0, 1);

        let catching = builder.start_function("catching", php_ir::FunctionFlags::default(), span);
        let catching_entry = builder.append_block(catching);
        let catch = builder.append_block(catching);
        let catching_after = builder.append_block(catching);
        let exception_local = builder.intern_local(catching, "exception");
        builder.emit(
            catching,
            catching_entry,
            php_ir::InstructionKind::EnterTry {
                catch: Some(catch),
                catch_types: vec!["throwable".to_owned()],
                finally: None,
                after: catching_after,
                exception_local: Some(exception_local),
            },
            span,
        );
        builder.terminate_jump(catching, catching_entry, catching_after, span);
        builder.terminate_jump(catching, catch, catching_after, span);
        builder.terminate_return(catching, catching_after, None, span);

        let finalizing =
            builder.start_function("finalizing", php_ir::FunctionFlags::default(), span);
        let finalizing_entry = builder.append_block(finalizing);
        let finally = builder.append_block(finalizing);
        let finalizing_after = builder.append_block(finalizing);
        builder.emit(
            finalizing,
            finalizing_entry,
            php_ir::InstructionKind::EnterTry {
                catch: None,
                catch_types: Vec::new(),
                finally: Some(finally),
                after: finalizing_after,
                exception_local: None,
            },
            span,
        );
        builder.terminate_jump(finalizing, finalizing_entry, finally, span);
        builder.emit(
            finalizing,
            finally,
            php_ir::InstructionKind::EndFinally {
                after: finalizing_after,
            },
            span,
        );
        builder.terminate_jump(finalizing, finally, finalizing_after, span);
        builder.terminate_return(finalizing, finalizing_after, None, span);

        let unit = builder.finish();
        assert!(ir_function_has_exception_handler(
            &unit.functions[catching.index()]
        ));
        assert!(ir_function_has_exception_handler(
            &unit.functions[finalizing.index()]
        ));
    }
}
