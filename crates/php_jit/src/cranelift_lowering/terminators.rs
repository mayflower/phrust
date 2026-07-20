use super::*;

#[allow(clippy::too_many_arguments)]
fn lower_region_condition(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    locals: &NativeLocalMap,
    registers: &NativeRegisterMap,
    native_operations: NativeOperationFunctions,
    deopt_out: ir::Value,
    condition: RegionOperand,
    constants: &[IrConstant],
    value_flow: &ExecutableValueFlow,
) -> Result<ir::Value, CraneliftLoweringError> {
    let value = lower_region_operand(builder, locals, registers, condition)?;
    let fact = value_flow.operand_fact(constants, condition);
    match fact.class {
        SsaValueClass::Int if fact.certainty != crate::region_ir::SsaCertainty::Unknown => {
            return Ok(builder.ins().icmp_imm(IntCC::NotEqual, value, 0));
        }
        SsaValueClass::Null if fact.certainty != crate::region_ir::SsaCertainty::Unknown => {
            return Ok(builder.ins().icmp(IntCC::NotEqual, value, value));
        }
        SsaValueClass::Bool if fact.certainty != crate::region_ir::SsaCertainty::Unknown => {
            return Ok(builder.ins().icmp_imm(
                IntCC::Equal,
                value,
                crate::jit_encode_constant(crate::JIT_VALUE_TRUE),
            ));
        }
        _ => {}
    }
    if let Some(helper) = native_operations.truthy {
        lower_guarded_unknown_condition(module, builder, helper, value, deopt_out)
    } else if builder.func.dfg.value_type(value) == types::I64 {
        Ok(builder.ins().icmp_imm(IntCC::NotEqual, value, 0))
    } else {
        Ok(value)
    }
}

/// Resolve the stable null/bool/int lanes without crossing the runtime ABI.
/// Runtime handles and opaque constant-pool handles retain the typed helper
/// slow path.
pub(super) fn lower_guarded_unknown_condition(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    helper: NativeHelper,
    value: ir::Value,
    deopt_out: ir::Value,
) -> Result<ir::Value, CraneliftLoweringError> {
    if !helper.inline_runtime_view {
        let slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let out = builder
            .ins()
            .stack_addr(module.target_config().pointer_type(), slot, 0);
        let call = call_native_helper(module, builder, helper, &[value, out]);
        require_native_operation_ok(
            builder,
            builder.inst_results(call)[0],
            helper.terminal_exit()?,
        )?;
        let truthy = builder.ins().stack_load(types::I64, slot, 0);
        return Ok(builder.ins().icmp_imm(IntCC::NotEqual, truthy, 0));
    }
    let inspect_runtime = builder.create_block();
    let inspect_non_runtime = builder.create_block();
    let inspect_descriptor = builder.create_block();
    let slow = builder.create_block();
    let merge = builder.create_block();
    builder.append_block_param(merge, types::I8);

    let is_true = builder.ins().icmp_imm(
        IntCC::Equal,
        value,
        crate::jit_encode_constant(crate::JIT_VALUE_TRUE),
    );
    let is_false = builder.ins().icmp_imm(
        IntCC::Equal,
        value,
        crate::jit_encode_constant(crate::JIT_VALUE_FALSE),
    );
    let is_null = builder
        .ins()
        .icmp_imm(IntCC::Equal, value, crate::jit_encode_constant(u32::MAX));
    let is_uninitialized = builder.ins().icmp_imm(
        IntCC::Equal,
        value,
        crate::jit_encode_constant(crate::JIT_VALUE_UNINITIALIZED),
    );
    let is_false_lane = builder.ins().bor(is_false, is_null);
    let is_false_lane = builder.ins().bor(is_false_lane, is_uninitialized);
    let is_reserved = builder.ins().bor(is_true, is_false_lane);
    let tag = builder
        .ins()
        .band_imm(value, crate::JIT_VALUE_TAG_MASK as i64);
    let is_runtime = builder
        .ins()
        .icmp_imm(IntCC::Equal, tag, crate::JIT_VALUE_RUNTIME_TAG as i64);
    let is_constant =
        builder
            .ins()
            .icmp_imm(IntCC::Equal, tag, crate::JIT_VALUE_CONSTANT_TAG as i64);
    let is_not_reserved = builder.ins().icmp_imm(IntCC::Equal, is_reserved, 0);
    let is_opaque_constant = builder.ins().band(is_constant, is_not_reserved);
    let integer_truthy = builder.ins().icmp_imm(IntCC::NotEqual, value, 0);
    let direct_truthy = builder.ins().select(is_reserved, is_true, integer_truthy);
    builder
        .ins()
        .brif(is_runtime, inspect_runtime, &[], inspect_non_runtime, &[]);

    builder.switch_to_block(inspect_non_runtime);
    builder.ins().brif(
        is_opaque_constant,
        slow,
        &[],
        merge,
        &[direct_truthy.into()],
    );

    builder.switch_to_block(inspect_runtime);
    let runtime_kind = builder
        .ins()
        .band_imm(value, crate::JIT_VALUE_RUNTIME_KIND_MASK as i64);
    let is_array = builder.ins().icmp_imm(
        IntCC::Equal,
        runtime_kind,
        crate::JIT_VALUE_RUNTIME_ARRAY_TAG as i64,
    );
    let is_string = builder.ins().icmp_imm(
        IntCC::Equal,
        runtime_kind,
        crate::JIT_VALUE_RUNTIME_STRING_TAG as i64,
    );
    let has_direct_descriptor = builder.ins().bor(is_array, is_string);
    builder
        .ins()
        .brif(has_direct_descriptor, inspect_descriptor, &[], slow, &[]);

    builder.switch_to_block(inspect_descriptor);
    let pointer_type = module.target_config().pointer_type();
    let view_offset = std::mem::offset_of!(crate::JitDeoptState, runtime_view) as i32;
    let slots = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        deopt_out,
        view_offset + std::mem::offset_of!(crate::JitNativeRuntimeView, value_slots) as i32,
    );
    let index = builder.ins().ireduce(types::I32, value);
    let index = builder.ins().uextend(pointer_type, index);
    let byte_offset = builder.ins().ishl_imm(index, 5);
    let descriptor = builder.ins().iadd(slots, byte_offset);
    let kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        descriptor,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let flags = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        descriptor,
        std::mem::offset_of!(crate::JitNativeValueSlot, flags) as i32,
    );
    let reserved = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        descriptor,
        std::mem::offset_of!(crate::JitNativeValueSlot, reserved) as i32,
    );
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        descriptor,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let array_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_ARRAY),
    );
    let array_version = builder.ins().icmp_imm(
        IntCC::Equal,
        flags,
        i64::from(crate::JIT_NATIVE_ARRAY_VIEW_ABI_VERSION),
    );
    let array_descriptor = builder.ins().band(array_kind, array_version);
    let array_ok = builder.ins().band(is_array, array_descriptor);
    let string_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_STRING),
    );
    let string_version = builder.ins().icmp_imm(
        IntCC::Equal,
        flags,
        i64::from(crate::JIT_NATIVE_STRING_VIEW_ABI_VERSION),
    );
    let string_descriptor = builder.ins().band(string_kind, string_version);
    let string_ok = builder.ins().band(is_string, string_descriptor);
    let descriptor_ok = builder.ins().bor(array_ok, string_ok);
    let non_empty = builder.ins().icmp_imm(IntCC::NotEqual, length, 0);
    let is_zero_string = builder.ins().icmp_imm(
        IntCC::Equal,
        reserved,
        i64::from(crate::JIT_NATIVE_STRING_VALUE_ZERO),
    );
    let not_zero_string = builder.ins().icmp_imm(IntCC::Equal, is_zero_string, 0);
    let string_truthy = builder.ins().band(non_empty, not_zero_string);
    let runtime_truthy = builder.ins().select(is_string, string_truthy, non_empty);
    builder
        .ins()
        .brif(descriptor_ok, merge, &[runtime_truthy.into()], slow, &[]);

    builder.switch_to_block(slow);
    let slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let out = builder
        .ins()
        .stack_addr(module.target_config().pointer_type(), slot, 0);
    let call = call_native_helper(module, builder, helper, &[value, out]);
    require_native_operation_ok(
        builder,
        builder.inst_results(call)[0],
        helper.terminal_exit()?,
    )?;
    let truthy = builder.ins().stack_load(types::I64, slot, 0);
    let truthy = builder.ins().icmp_imm(IntCC::NotEqual, truthy, 0);
    builder.ins().jump(merge, &[truthy.into()]);

    builder.switch_to_block(merge);
    Ok(builder.block_params(merge)[0])
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_region_terminator(
    builder: &mut FunctionBuilder<'_>,
    blocks: &BTreeMap<BlockId, ir::Block>,
    locals: &NativeLocalMap,
    registers: &NativeRegisterMap,
    result_out: ir::Value,
    deopt_out: ir::Value,
    pending_status: Variable,
    pending_value: Variable,
    module: &mut JITModule,
    native_operations: NativeOperationFunctions,
    function: FunctionId,
    return_check_required: bool,
    terminator: &RegionTerminator,
    constants: &[IrConstant],
    value_flow: &ExecutableValueFlow,
) -> Result<(), CraneliftLoweringError> {
    match terminator {
        RegionTerminator::Jump { target } => {
            builder.ins().jump(cranelift_block(blocks, *target)?, &[]);
        }
        RegionTerminator::JumpIfFalse {
            condition,
            target,
            fallthrough,
        } => {
            let condition = lower_region_condition(
                module,
                builder,
                locals,
                registers,
                native_operations,
                deopt_out,
                *condition,
                constants,
                value_flow,
            )?;
            let false_block = cranelift_block(blocks, *target)?;
            let true_block = cranelift_block(blocks, *fallthrough)?;
            builder
                .ins()
                .brif(condition, true_block, &[], false_block, &[]);
        }
        RegionTerminator::JumpIfTrue {
            condition,
            target,
            fallthrough,
        } => {
            let condition = lower_region_condition(
                module,
                builder,
                locals,
                registers,
                native_operations,
                deopt_out,
                *condition,
                constants,
                value_flow,
            )?;
            let true_block = cranelift_block(blocks, *target)?;
            let false_block = cranelift_block(blocks, *fallthrough)?;
            builder
                .ins()
                .brif(condition, true_block, &[], false_block, &[]);
        }
        RegionTerminator::JumpIf {
            condition,
            if_true,
            if_false,
        } => {
            let condition = lower_region_condition(
                module,
                builder,
                locals,
                registers,
                native_operations,
                deopt_out,
                *condition,
                constants,
                value_flow,
            )?;
            builder.ins().brif(
                condition,
                cranelift_block(blocks, *if_true)?,
                &[],
                cranelift_block(blocks, *if_false)?,
                &[],
            );
        }
        RegionTerminator::Return { value, finally } => {
            let value = lower_region_operand(builder, locals, registers, *value)?;
            let value = if return_check_required {
                let function_value = builder.ins().iconst(types::I64, i64::from(function.raw()));
                lower_native_value_operation(
                    module,
                    builder,
                    native_operations.return_check,
                    0,
                    &[value, function_value],
                    result_out,
                )?
            } else {
                value
            };
            let status = builder
                .ins()
                .iconst(types::I32, i64::from(crate::JitCallStatus::RETURN.0));
            lower_region_frame_exit(
                builder,
                blocks,
                locals,
                result_out,
                pending_status,
                pending_value,
                value,
                status,
                *finally,
                module,
                native_operations,
                value_flow,
                function,
            )?;
        }
        RegionTerminator::ReturnReference { local, finally } => {
            let value = use_local_variable(builder, locals, *local)?;
            let status = builder.ins().iconst(
                types::I32,
                i64::from(crate::JitCallStatus::RETURN_REFERENCE.0),
            );
            lower_region_frame_exit(
                builder,
                blocks,
                locals,
                result_out,
                pending_status,
                pending_value,
                value,
                status,
                *finally,
                module,
                native_operations,
                value_flow,
                function,
            )?;
        }
        RegionTerminator::Exit { value, finally } => {
            let value = value
                .map(|value| lower_region_operand(builder, locals, registers, value))
                .transpose()?
                .unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            let status = builder
                .ins()
                .iconst(types::I32, i64::from(crate::JitCallStatus::EXIT.0));
            lower_region_frame_exit(
                builder,
                blocks,
                locals,
                result_out,
                pending_status,
                pending_value,
                value,
                status,
                *finally,
                module,
                native_operations,
                value_flow,
                function,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_region_frame_exit(
    builder: &mut FunctionBuilder<'_>,
    blocks: &BTreeMap<BlockId, ir::Block>,
    locals: &NativeLocalMap,
    result_out: ir::Value,
    pending_status: Variable,
    pending_value: Variable,
    value: ir::Value,
    status: ir::Value,
    finally: Option<BlockId>,
    module: &mut JITModule,
    native_operations: NativeOperationFunctions,
    value_flow: &ExecutableValueFlow,
    function: FunctionId,
) -> Result<(), CraneliftLoweringError> {
    if let Some(finally) = finally {
        builder.def_var(pending_status, status);
        builder.def_var(pending_value, value);
        builder.ins().jump(cranelift_block(blocks, finally)?, &[]);
    } else {
        lower_owned_frame_locals(
            module,
            builder,
            locals,
            native_operations,
            value_flow,
            function,
            result_out,
        )?;
        builder
            .ins()
            .store(MemFlagsData::new(), value, result_out, 0);
        builder.ins().return_(&[status]);
    }
    Ok(())
}

pub(super) fn lower_owned_frame_locals(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    locals: &NativeLocalMap,
    native_operations: NativeOperationFunctions,
    value_flow: &ExecutableValueFlow,
    function: FunctionId,
    result_out: ir::Value,
) -> Result<(), CraneliftLoweringError> {
    for local in locals.keys() {
        let fact = value_flow.local_fact(*local);
        if value_flow.releases_local_at_frame_exit(*local)
            && fact.has_runtime_lifecycle()
            && fact.ownership == SsaOwnership::Owned
        {
            let value = use_local_variable(builder, locals, *local)?;
            let _ = lower_native_value_operation(
                module,
                builder,
                native_operations.value_lifecycle,
                native_frame_cleanup_operation(function),
                &[value],
                result_out,
            )?;
        }
    }
    Ok(())
}
