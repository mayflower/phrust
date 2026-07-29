fn lower_direct_new_array(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    helper: Option<NativeHelper>,
    result_out: ir::Value,
    deopt_out: ir::Value,
    optimizing_transition: Option<NativeOptimizingTransition<'_>>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let accepted = builder.create_block();
    let rejected = builder.create_block();
    let merge = builder.create_block();
    builder.append_block_param(merge, types::I64);
    let pointer_type = builder.func.dfg.value_type(deopt_out);
    let view = lower_active_runtime_view(builder, deopt_out);
    let entry_next_ptr = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_next) as i32,
    );
    let entry_next = builder
        .ins()
        .load(types::I32, MemFlagsData::new(), entry_next_ptr, 0);
    let entry_end = builder.ins().iadd_imm(
        entry_next,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_INITIAL_CAPACITY),
    );
    let entry_room = builder.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        entry_end,
        crate::JIT_NATIVE_DIRECT_ARRAY_ENTRY_CAPACITY as i64,
    );
    builder.ins().brif(entry_room, accepted, &[], rejected, &[]);

    builder.switch_to_block(rejected);
    let placeholder = if let Some(transition) = optimizing_transition {
        transition.emit_value(builder)?
    } else {
        lower_native_value_operation(module, builder, helper, 0, &[], result_out)?
    };
    builder.ins().jump(merge, &[placeholder.into()]);

    builder.switch_to_block(accepted);
    let next = lower_reserve_direct_value_index(builder, deopt_out, rejected);
    builder
        .ins()
        .store(MemFlagsData::new(), entry_end, entry_next_ptr, 0);
    let slots = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_value_slots) as i32,
    );
    let entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_entries) as i32,
    );
    let next_pointer = builder.ins().uextend(pointer_type, next);
    let slot_offset = builder.ins().ishl_imm(next_pointer, 5);
    let slot = builder.ins().iadd(slots, slot_offset);
    let entry_pointer = builder.ins().uextend(pointer_type, entry_next);
    let entry_offset = builder.ins().ishl_imm(entry_pointer, 4);
    let entry = builder.ins().iadd(entries, entry_offset);
    for (value, offset) in [
        (
            builder.ins().iconst(types::I32, 1),
            std::mem::offset_of!(crate::JitNativeValueSlot, refcount),
        ),
        (
            builder.ins().iconst(
                types::I32,
                i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
            ),
            std::mem::offset_of!(crate::JitNativeValueSlot, kind),
        ),
        (
            builder.ins().iconst(
                types::I32,
                i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION),
            ),
            std::mem::offset_of!(crate::JitNativeValueSlot, flags),
        ),
        (
            builder.ins().iconst(
                types::I32,
                i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_INITIAL_CAPACITY),
            ),
            std::mem::offset_of!(crate::JitNativeValueSlot, reserved),
        ),
    ] {
        builder
            .ins()
            .store(MemFlagsData::new(), value, slot, offset as i32);
    }
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().store(
        MemFlagsData::new(),
        zero,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    builder.ins().store(
        MemFlagsData::new(),
        entry,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let encoded_index = builder
        .ins()
        .iadd_imm(next, i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE));
    let encoded_index = builder.ins().uextend(types::I64, encoded_index);
    let encoded = builder
        .ins()
        .bor_imm(encoded_index, crate::JIT_VALUE_RUNTIME_ARRAY_TAG as i64);
    let state = lower_direct_array_state_address(builder, encoded, deopt_out);
    builder.ins().store(
        MemFlagsData::new(),
        zero,
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, next_append_key) as i32,
    );
    let zero32 = builder.ins().iconst(types::I32, 0);
    builder.ins().store(
        MemFlagsData::new(),
        zero32,
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, has_next_append_key) as i32,
    );
    builder.ins().jump(merge, &[encoded.into()]);

    builder.switch_to_block(merge);
    Ok(builder.block_params(merge)[0])
}

fn lower_direct_array_require_supported_key(
    builder: &mut FunctionBuilder<'_>,
    key: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<(), CraneliftLoweringError> {
    let accepted = builder.create_block();
    let rejected = builder.create_block();
    let integer = lower_optimizing_integer_candidate(builder, key, transition.deopt_out).0;
    let (string, _, _) = lower_native_string_key_descriptor(builder, key, transition.deopt_out);
    let supported = builder.ins().bor(integer, string);
    builder.ins().brif(supported, accepted, &[], rejected, &[]);

    builder.switch_to_block(rejected);
    let _ = transition.emit_value(builder)?;
    builder.ins().jump(accepted, &[]);

    builder.switch_to_block(accepted);
    Ok(())
}

fn lower_direct_array_state_address(
    builder: &mut FunctionBuilder<'_>,
    array: ir::Value,
    deopt_out: ir::Value,
) -> ir::Value {
    let pointer_type = builder.func.dfg.value_type(deopt_out);
    let view = lower_active_runtime_view(builder, deopt_out);
    let states = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_states) as i32,
    );
    let encoded_index = builder.ins().ireduce(types::I32, array);
    let index = builder.ins().iadd_imm(
        encoded_index,
        -i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let wide_index = builder.ins().uextend(pointer_type, index);
    let offset = builder.ins().ishl_imm(
        wide_index,
        std::mem::size_of::<crate::JitNativeDirectArrayState>().trailing_zeros() as i64,
    );
    builder.ins().iadd(states, offset)
}

fn lower_direct_array_next_integer_key(
    builder: &mut FunctionBuilder<'_>,
    array: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let inspect = builder.create_block();
    let check = builder.create_block();
    let scan = builder.create_block();
    let scan_entry = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(scan, types::I64);
    builder.append_block_param(done, types::I64);

    let is_array = lower_value_has_tag(builder, array, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let encoded_index = builder.ins().ireduce(types::I32, array);
    let direct_index = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        encoded_index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let direct = builder.ins().band(is_array, direct_index);
    builder.ins().brif(direct, inspect, &[], rejected, &[]);

    builder.switch_to_block(inspect);
    let slot = lower_optimizing_slot_address(builder, array, transition.deopt_out);
    let kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let direct_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    builder.ins().brif(direct_kind, check, &[], rejected, &[]);

    builder.switch_to_block(check);
    let state = lower_direct_array_state_address(builder, array, transition.deopt_out);
    let next = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, next_append_key) as i32,
    );
    let has_next = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, has_next_append_key) as i32,
    );
    let absent = builder.ins().icmp_imm(IntCC::Equal, has_next, 0);
    let zero = builder.ins().iconst(types::I64, 0);
    let next = builder.ins().select(absent, zero, next);
    let at_maximum = builder.ins().icmp_imm(IntCC::Equal, next, i64::MAX);
    builder
        .ins()
        .brif(at_maximum, scan, &[zero.into()], done, &[next.into()]);

    builder.switch_to_block(scan);
    let scan_index = builder.block_params(scan)[0];
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let exhausted = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, scan_index, length);
    builder
        .ins()
        .brif(exhausted, done, &[next.into()], scan_entry, &[]);

    builder.switch_to_block(scan_entry);
    let pointer_type = builder.func.dfg.value_type(transition.deopt_out);
    let entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let wide_index = if pointer_type == types::I64 {
        scan_index
    } else {
        builder.ins().ireduce(pointer_type, scan_index)
    };
    let entry_offset = builder.ins().ishl_imm(wide_index, 4);
    let entry = builder.ins().iadd(entries, entry_offset);
    let candidate = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), entry, 0);
    let (integer, candidate_raw) =
        lower_native_array_key_integer_candidate(builder, candidate, transition.deopt_out);
    let maximum = builder
        .ins()
        .icmp_imm(IntCC::Equal, candidate_raw, i64::MAX);
    let occupied = builder.ins().band(integer, maximum);
    let following = builder.ins().iadd_imm(scan_index, 1);
    builder
        .ins()
        .brif(occupied, rejected, &[], scan, &[following.into()]);

    builder.switch_to_block(rejected);
    let placeholder = transition.emit_value(builder)?;
    builder.ins().jump(done, &[placeholder.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_array_append(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    array: ir::Value,
    key: Option<ir::Value>,
    value: ir::Value,
    move_value: bool,
    result_out: ir::Value,
    deopt_out: ir::Value,
    fallback: NativeArrayAppendFallback<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let literal_value_borrowed = builder.ins().iconst(types::I8, 0);
    let pointer_type = builder.func.dfg.value_type(deopt_out);
    let inspect = builder.create_block();
    let inspect_capacity = builder.create_block();
    let inspect_growth = builder.create_block();
    let reuse_growth = builder.create_block();
    let bump_growth = builder.create_block();
    let growth_allocated = builder.create_block();
    let copy_entries = builder.create_block();
    let copy_entry = builder.create_block();
    let growth_done = builder.create_block();
    let prepare_append = builder.create_block();
    let scan_append_key = builder.create_block();
    let scan_append_entry = builder.create_block();
    let finish_append_key = builder.create_block();
    let append = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(copy_entries, types::I64);
    builder.append_block_param(growth_allocated, pointer_type);
    builder.append_block_param(scan_append_key, types::I64);
    builder.append_block_param(scan_append_key, types::I64);
    builder.append_block_param(scan_append_key, types::I8);
    builder.append_block_param(finish_append_key, types::I64);
    builder.append_block_param(finish_append_key, types::I8);
    builder.append_block_param(append, types::I64);
    builder.append_block_param(done, types::I64);
    let array_kind = lower_value_has_tag(builder, array, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let index = builder.ins().ireduce(types::I32, array);
    let direct_index = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let direct = builder.ins().band(array_kind, direct_index);
    builder.ins().brif(direct, inspect, &[], rejected, &[]);

    builder.switch_to_block(inspect);
    let slot = lower_optimizing_slot_address(builder, array, deopt_out);
    let kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let refcount = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, refcount) as i32,
    );
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let capacity = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, reserved) as i32,
    );
    let direct_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    let unique = builder.ins().icmp_imm(IntCC::Equal, refcount, 1);
    let admitted = builder.ins().band(direct_kind, unique);
    builder
        .ins()
        .brif(admitted, inspect_capacity, &[], rejected, &[]);

    builder.switch_to_block(inspect_capacity);
    let capacity_wide = builder.ins().uextend(types::I64, capacity);
    let has_room = builder
        .ins()
        .icmp(IntCC::UnsignedLessThan, length, capacity_wide);
    builder
        .ins()
        .brif(has_room, prepare_append, &[], inspect_growth, &[]);

    // A direct array owns a contiguous slice in the request arena. Growing it
    // allocates a new slice, copies encoded entries without changing their
    // ownership, and atomically switches the descriptor before appending. The
    // old slice is dead arena storage, not a second owner. This removes the
    // previous capacity-eight transition into the Rust PhpArray path.
    builder.switch_to_block(inspect_growth);
    let view = lower_active_runtime_view(builder, deopt_out);
    let next_ptr = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_next) as i32,
    );
    let arena = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_entries) as i32,
    );
    let next = builder
        .ins()
        .load(types::I32, MemFlagsData::new(), next_ptr, 0);
    let doubled = builder.ins().imul_imm(capacity, 2);
    let minimum = builder.ins().iconst(
        types::I32,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_INITIAL_CAPACITY),
    );
    let capacity_is_zero = builder.ins().icmp_imm(IntCC::Equal, capacity, 0);
    let grown_capacity = builder.ins().select(capacity_is_zero, minimum, doubled);
    let free_heads = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_free_heads) as i32,
    );
    let grown_leading_zeros = builder.ins().clz(grown_capacity);
    let bit_index_ceiling = builder.ins().iconst(types::I32, 31);
    let bucket = builder.ins().isub(bit_index_ceiling, grown_leading_zeros);
    let bucket_wide = builder.ins().uextend(pointer_type, bucket);
    let bucket_offset = builder.ins().ishl_imm(bucket_wide, 2);
    let free_head_ptr = builder.ins().iadd(free_heads, bucket_offset);
    let free_head = builder
        .ins()
        .load(types::I32, MemFlagsData::new(), free_head_ptr, 0);
    let has_free = builder.ins().icmp_imm(
        IntCC::NotEqual,
        free_head,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE),
    );
    let old_entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    builder
        .ins()
        .brif(has_free, reuse_growth, &[], bump_growth, &[]);

    builder.switch_to_block(reuse_growth);
    let free_head_wide = builder.ins().uextend(pointer_type, free_head);
    let free_offset = builder.ins().ishl_imm(free_head_wide, 4);
    let reused_entries = builder.ins().iadd(arena, free_offset);
    let preceding_head = builder
        .ins()
        .load(types::I32, MemFlagsData::new(), reused_entries, 0);
    builder
        .ins()
        .store(MemFlagsData::new(), preceding_head, free_head_ptr, 0);
    let reused_bytes_ptr = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        view,
        std::mem::offset_of!(crate::JitNativeRuntimeView, direct_array_reused_bytes) as i32,
    );
    let reused_bytes = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), reused_bytes_ptr, 0);
    let grown_capacity_wide = builder.ins().uextend(types::I64, grown_capacity);
    let reused_delta = builder.ins().imul_imm(
        grown_capacity_wide,
        std::mem::size_of::<crate::JitNativeDirectArrayEntry>() as i64,
    );
    let reused_bytes = builder.ins().iadd(reused_bytes, reused_delta);
    builder
        .ins()
        .store(MemFlagsData::new(), reused_bytes, reused_bytes_ptr, 0);
    builder
        .ins()
        .jump(growth_allocated, &[reused_entries.into()]);

    builder.switch_to_block(bump_growth);
    let grown_end = builder.ins().iadd(next, grown_capacity);
    let arena_room = builder.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        grown_end,
        crate::JIT_NATIVE_DIRECT_ARRAY_ENTRY_CAPACITY as i64,
    );
    let next_wide = builder.ins().uextend(pointer_type, next);
    let grown_offset = builder.ins().ishl_imm(next_wide, 4);
    let bumped_entries = builder.ins().iadd(arena, grown_offset);
    let bump_accepted = builder.create_block();
    builder
        .ins()
        .brif(arena_room, bump_accepted, &[], rejected, &[]);
    builder.switch_to_block(bump_accepted);
    builder
        .ins()
        .store(MemFlagsData::new(), grown_end, next_ptr, 0);
    builder
        .ins()
        .jump(growth_allocated, &[bumped_entries.into()]);

    builder.switch_to_block(growth_allocated);
    let grown_entries = builder.block_params(growth_allocated)[0];
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().jump(copy_entries, &[zero.into()]);

    builder.switch_to_block(copy_entries);
    let copy_index = builder.block_params(copy_entries)[0];
    let copied_all = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, copy_index, length);
    builder
        .ins()
        .brif(copied_all, growth_done, &[], copy_entry, &[]);

    builder.switch_to_block(copy_entry);
    let copy_pointer = if pointer_type == types::I64 {
        copy_index
    } else {
        builder.ins().ireduce(pointer_type, copy_index)
    };
    let copy_offset = builder.ins().ishl_imm(copy_pointer, 4);
    let old_entry = builder.ins().iadd(old_entries, copy_offset);
    let new_entry = builder.ins().iadd(grown_entries, copy_offset);
    let copied_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), old_entry, 0);
    let copied_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        old_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    builder
        .ins()
        .store(MemFlagsData::new(), copied_key, new_entry, 0);
    builder.ins().store(
        MemFlagsData::new(),
        copied_value,
        new_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let next_copy = builder.ins().iadd_imm(copy_index, 1);
    builder.ins().jump(copy_entries, &[next_copy.into()]);

    builder.switch_to_block(growth_done);
    // The copied range is no longer an owner. Publish it in the exact-size
    // request-local free bucket so the next growth reuses it without Rust.
    let old_leading_zeros = builder.ins().clz(capacity);
    let old_bit_index_ceiling = builder.ins().iconst(types::I32, 31);
    let old_bucket = builder.ins().isub(old_bit_index_ceiling, old_leading_zeros);
    let old_bucket_wide = builder.ins().uextend(pointer_type, old_bucket);
    let old_bucket_offset = builder.ins().ishl_imm(old_bucket_wide, 2);
    let old_head_ptr = builder.ins().iadd(free_heads, old_bucket_offset);
    let old_head = builder
        .ins()
        .load(types::I32, MemFlagsData::new(), old_head_ptr, 0);
    let old_offset = builder.ins().isub(old_entries, arena);
    let old_index_wide = builder.ins().ushr_imm(old_offset, 4);
    let old_index = builder.ins().ireduce(types::I32, old_index_wide);
    builder
        .ins()
        .store(MemFlagsData::new(), old_head, old_entries, 0);
    builder
        .ins()
        .store(MemFlagsData::new(), old_index, old_head_ptr, 0);
    builder.ins().store(
        MemFlagsData::new(),
        grown_entries,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    builder.ins().store(
        MemFlagsData::new(),
        grown_capacity,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, reserved) as i32,
    );
    builder.ins().jump(prepare_append, &[]);

    builder.switch_to_block(prepare_append);
    if let Some(entry_key) = key {
        builder.ins().jump(append, &[entry_key.into()]);
    } else {
        let state = lower_direct_array_state_address(builder, array, deopt_out);
        let next_key = builder.ins().load(
            types::I64,
            MemFlagsData::new(),
            state,
            std::mem::offset_of!(crate::JitNativeDirectArrayState, next_append_key) as i32,
        );
        let has_next = builder.ins().load(
            types::I32,
            MemFlagsData::new(),
            state,
            std::mem::offset_of!(crate::JitNativeDirectArrayState, has_next_append_key) as i32,
        );
        let absent = builder.ins().icmp_imm(IntCC::Equal, has_next, 0);
        let zero = builder.ins().iconst(types::I64, 0);
        let next_key = builder.ins().select(absent, zero, next_key);
        let at_maximum = builder.ins().icmp_imm(IntCC::Equal, next_key, i64::MAX);
        let no = builder.ins().iconst(types::I8, 0);
        builder.ins().brif(
            at_maximum,
            scan_append_key,
            &[zero.into(), next_key.into(), no.into()],
            append,
            &[next_key.into()],
        );
    }

    if key.is_none() {
        // At i64::MAX PHP admits one append only while that exact key is absent.
        // The authoritative auto-index state handles every ordinary append;
        // this scan is therefore confined to the terminal-key edge case.
        builder.switch_to_block(scan_append_key);
        let scan_index = builder.block_params(scan_append_key)[0];
        let next_key = builder.block_params(scan_append_key)[1];
        let found_maximum = builder.block_params(scan_append_key)[2];
        let scanned_all = builder
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, scan_index, length);
        builder.ins().brif(
            scanned_all,
            finish_append_key,
            &[next_key.into(), found_maximum.into()],
            scan_append_entry,
            &[],
        );

        builder.switch_to_block(scan_append_entry);
        let entries = builder.ins().load(
            pointer_type,
            MemFlagsData::new(),
            slot,
            std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
        );
        let scan_pointer = if pointer_type == types::I64 {
            scan_index
        } else {
            builder.ins().ireduce(pointer_type, scan_index)
        };
        let scan_offset = builder.ins().ishl_imm(scan_pointer, 4);
        let scan_entry = builder.ins().iadd(entries, scan_offset);
        let candidate = builder
            .ins()
            .load(types::I64, MemFlagsData::new(), scan_entry, 0);
        let (candidate_integer, candidate_raw) =
            lower_native_array_key_integer_candidate(builder, candidate, deopt_out);
        let maximum = builder
            .ins()
            .icmp_imm(IntCC::Equal, candidate_raw, i64::MAX);
        let found = builder.ins().band(candidate_integer, maximum);
        let found_maximum = builder.ins().bor(found_maximum, found);
        let next_scan = builder.ins().iadd_imm(scan_index, 1);
        builder.ins().jump(
            scan_append_key,
            &[next_scan.into(), next_key.into(), found_maximum.into()],
        );

        builder.switch_to_block(finish_append_key);
        let next_key = builder.block_params(finish_append_key)[0];
        let overflow = builder.block_params(finish_append_key)[1];
        builder
            .ins()
            .brif(overflow, rejected, &[], append, &[next_key.into()]);
    }

    builder.switch_to_block(append);
    let entry_key = builder.block_params(append)[0];
    let entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let pointer_type = builder.func.dfg.value_type(entries);
    let entry_index = if pointer_type == types::I64 {
        length
    } else {
        builder.ins().ireduce(pointer_type, length)
    };
    let entry_offset = builder.ins().ishl_imm(entry_index, 4);
    let entry = builder.ins().iadd(entries, entry_offset);
    lower_optimizing_retain(builder, entry_key, deopt_out);
    if !move_value {
        lower_optimizing_retain(builder, value, deopt_out);
    } else {
        lower_optimizing_retain_if(builder, value, literal_value_borrowed, deopt_out);
    }
    builder
        .ins()
        .store(MemFlagsData::new(), entry_key, entry, 0);
    builder.ins().store(
        MemFlagsData::new(),
        value,
        entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let next_length = builder.ins().iadd_imm(length, 1);
    builder.ins().store(
        MemFlagsData::new(),
        next_length,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let state = lower_direct_array_state_address(builder, array, deopt_out);
    let current_next = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, next_append_key) as i32,
    );
    let has_current_next = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, has_next_append_key) as i32,
    );
    let (integer_key, integer_raw) =
        lower_native_array_key_integer_candidate(builder, entry_key, deopt_out);
    let maximum_key = builder.ins().icmp_imm(IntCC::Equal, integer_raw, i64::MAX);
    let incremented_key = builder.ins().iadd_imm(integer_raw, 1);
    let candidate_next = builder
        .ins()
        .select(maximum_key, integer_raw, incremented_key);
    let advances = builder
        .ins()
        .icmp(IntCC::SignedGreaterThan, candidate_next, current_next);
    let absent = builder.ins().icmp_imm(IntCC::Equal, has_current_next, 0);
    let advances = builder.ins().bor(absent, advances);
    let advances = builder.ins().band(integer_key, advances);
    let next_append_key = builder.ins().select(advances, candidate_next, current_next);
    builder.ins().store(
        MemFlagsData::new(),
        next_append_key,
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, next_append_key) as i32,
    );
    let has_next = builder.ins().icmp_imm(IntCC::NotEqual, integer_key, 0);
    let had_next = builder.ins().icmp_imm(IntCC::NotEqual, has_current_next, 0);
    let has_next = builder.ins().bor(has_next, had_next);
    let has_next = builder.ins().uextend(types::I32, has_next);
    builder.ins().store(
        MemFlagsData::new(),
        has_next,
        state,
        std::mem::offset_of!(crate::JitNativeDirectArrayState, has_next_append_key) as i32,
    );
    // PhpArray initializes an absent internal pointer when the first entry is
    // appended (including after the pointer ran past the end). Preserve that
    // behavior in the authoritative dense representation.
    let flags = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, flags) as i32,
    );
    let cursor = builder
        .ins()
        .ushr_imm(flags, crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_SHIFT as i64);
    let absent = builder.ins().icmp_imm(
        IntCC::Equal,
        cursor,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_NONE),
    );
    let first = builder.ins().iconst(
        types::I32,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION),
    );
    let flags = builder.ins().select(absent, first, flags);
    builder.ins().store(
        MemFlagsData::new(),
        flags,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, flags) as i32,
    );
    builder.ins().jump(done, &[array.into()]);

    builder.switch_to_block(rejected);
    let null = builder
        .ins()
        .iconst(types::I64, crate::jit_encode_constant(u32::MAX));
    let updated = lower_array_write_fallback(
        module,
        builder,
        fallback,
        array,
        key.unwrap_or(null),
        value,
        result_out,
        deopt_out,
    )?;
    // A slow-path COW separation may return a distinct array handle.
    builder.ins().jump(done, &[updated.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_array_insert(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    array: ir::Value,
    key: ir::Value,
    constant_string_key: bool,
    value: ir::Value,
    move_value: bool,
    result_out: ir::Value,
    deopt_out: ir::Value,
    fallback: NativeArrayAppendFallback<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    if constant_string_key && matches!(fallback, NativeArrayAppendFallback::Baseline { .. }) {
        return lower_array_write_fallback(
            module, builder, fallback, array, key, value, result_out, deopt_out,
        );
    }
    let inspect = builder.create_block();
    let search = builder.create_block();
    let compare = builder.create_block();
    let next = builder.create_block();
    let found = builder.create_block();
    let replace = builder.create_block();
    let missing = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    let pointer_type = module.target_config().pointer_type();
    builder.append_block_param(search, types::I64);
    builder.append_block_param(next, types::I64);
    builder.append_block_param(found, pointer_type);
    builder.append_block_param(done, types::I64);

    let array_kind = lower_value_has_tag(builder, array, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let index = builder.ins().ireduce(types::I32, array);
    let direct_index = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let direct = builder.ins().band(array_kind, direct_index);
    builder.ins().brif(direct, inspect, &[], rejected, &[]);

    builder.switch_to_block(inspect);
    let slot = lower_optimizing_slot_address(builder, array, deopt_out);
    let kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let refcount = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, refcount) as i32,
    );
    let direct_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    let unique = builder.ins().icmp_imm(IntCC::Equal, refcount, 1);
    let supported_key = match fallback {
        NativeArrayAppendFallback::Optimizing(_) => {
            let integer = lower_optimizing_integer_candidate(builder, key, deopt_out).0;
            let (string, _, _) = lower_native_string_key_descriptor(builder, key, deopt_out);
            builder.ins().bor(integer, string)
        }
        // Baseline keeps the complete PHP key-conversion semantics behind its
        // single typed continuation. String literals are already published
        // native values here, so the continuation never sees a unit-local
        // constant encoding.
        NativeArrayAppendFallback::Baseline { .. } => {
            let key_runtime = lower_is_runtime_handle(builder, key);
            let key_constant =
                lower_value_has_namespace_tag(builder, key, crate::JIT_VALUE_CONSTANT_TAG);
            let immediate = builder.ins().icmp_imm(IntCC::Equal, key_runtime, 0);
            builder.ins().band_not(immediate, key_constant)
        }
    };
    let _ = constant_string_key;
    let admitted = builder.ins().band(direct_kind, unique);
    let admitted = builder.ins().band(admitted, supported_key);
    let zero = builder.ins().iconst(types::I64, 0);
    builder
        .ins()
        .brif(admitted, search, &[zero.into()], rejected, &[]);

    builder.switch_to_block(search);
    let search_index = builder.block_params(search)[0];
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let exhausted = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, search_index, length);
    builder.ins().brif(exhausted, missing, &[], compare, &[]);

    builder.switch_to_block(compare);
    let entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let entry_index = if pointer_type == types::I64 {
        search_index
    } else {
        builder.ins().ireduce(pointer_type, search_index)
    };
    let entry_offset = builder.ins().ishl_imm(entry_index, 4);
    let entry = builder.ins().iadd(entries, entry_offset);
    let candidate = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), entry, 0);
    let matches = match fallback {
        NativeArrayAppendFallback::Optimizing(_) => {
            lower_native_array_key_equal(builder, candidate, key, deopt_out)
        }
        NativeArrayAppendFallback::Baseline { .. } => {
            builder.ins().icmp(IntCC::Equal, candidate, key)
        }
    };
    builder.ins().brif(
        matches,
        found,
        &[entry.into()],
        next,
        &[search_index.into()],
    );

    builder.switch_to_block(next);
    let current_index = builder.block_params(next)[0];
    let next_index = builder.ins().iadd_imm(current_index, 1);
    builder.ins().jump(search, &[next_index.into()]);

    builder.switch_to_block(found);
    let entry = builder.block_params(found)[0];
    let old = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let unchanged = builder.ins().icmp(IntCC::Equal, old, value);
    builder
        .ins()
        .brif(unchanged, done, &[array.into()], replace, &[]);

    builder.switch_to_block(replace);
    let literal_value_borrowed = builder.ins().iconst(types::I8, 0);
    let stored_value = match fallback {
        NativeArrayAppendFallback::Optimizing(transition) => {
            // PHP assignment to an array element that is already a
            // reference writes through that reference. Replacing the entry
            // itself detaches aliases, which is observably wrong for
            // variadic by-reference parameters and ordinary referenced
            // array elements alike.
            lower_optimizing_store_reference_scalar(
                builder,
                old,
                value,
                true,
                !move_value,
                transition,
            )?
        }
        NativeArrayAppendFallback::Baseline {
            lifecycle,
            operation,
            ..
        } => {
            let _ = lower_guarded_value_release(
                module,
                builder,
                lifecycle,
                operation | 1,
                old,
                result_out,
                deopt_out,
            )?;
            if !move_value {
                lower_optimizing_retain(builder, value, deopt_out);
            } else {
                lower_optimizing_retain_if(builder, value, literal_value_borrowed, deopt_out);
            }
            value
        }
    };
    builder.ins().store(
        MemFlagsData::new(),
        stored_value,
        entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    builder.ins().jump(done, &[array.into()]);

    builder.switch_to_block(missing);
    let (key, key_owned, normalized_transition) = match fallback {
        NativeArrayAppendFallback::Optimizing(transition) => {
            let original_key = key;
            let (array_integer, integer_raw) =
                lower_native_array_key_integer_candidate(builder, original_key, deopt_out);
            let ordinary_integer =
                lower_optimizing_integer_candidate(builder, original_key, deopt_out).0;
            let numeric_string = builder.ins().band_not(array_integer, ordinary_integer);
            let normalize = builder.create_block();
            let preserve = builder.create_block();
            let normalized = builder.create_block();
            builder.append_block_param(normalized, types::I64);
            builder.append_block_param(normalized, types::I8);
            builder
                .ins()
                .brif(numeric_string, normalize, &[], preserve, &[]);

            builder.switch_to_block(normalize);
            let normalized_key =
                lower_direct_int_key_or_reject(builder, integer_raw, deopt_out, rejected);
            let owned = builder.ins().iconst(types::I8, 1);
            builder
                .ins()
                .jump(normalized, &[normalized_key.into(), owned.into()]);

            builder.switch_to_block(preserve);
            let borrowed = builder.ins().iconst(types::I8, 0);
            builder
                .ins()
                .jump(normalized, &[original_key.into(), borrowed.into()]);

            builder.switch_to_block(normalized);
            (
                builder.block_params(normalized)[0],
                builder.block_params(normalized)[1],
                Some(transition),
            )
        }
        NativeArrayAppendFallback::Baseline { .. } => {
            let borrowed = builder.ins().iconst(types::I8, 0);
            (key, borrowed, None)
        }
    };
    let updated = lower_direct_array_append(
        module,
        builder,
        array,
        Some(key),
        value,
        move_value,
        result_out,
        deopt_out,
        fallback,
    )?;
    if let Some(transition) = normalized_transition {
        let release = builder.create_block();
        let complete = builder.create_block();
        builder.ins().brif(key_owned, release, &[], complete, &[]);
        builder.switch_to_block(release);
        lower_optimizing_release(builder, key, transition)?;
        builder.ins().jump(complete, &[]);
        builder.switch_to_block(complete);
    }
    builder.ins().jump(done, &[updated.into()]);

    builder.switch_to_block(rejected);
    let updated = lower_array_write_fallback(
        module, builder, fallback, array, key, value, result_out, deopt_out,
    )?;
    builder.ins().jump(done, &[updated.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_array_ensure_unique_capacity(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    operation: FuncId,
    array: ir::Value,
    additional: ir::Value,
    consume_owner: bool,
    _result_out: ir::Value,
    deopt_out: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let accepted = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(done, types::I64);
    let callee = module.declare_func_in_func(operation, builder.func);
    let consume_owner = builder.ins().iconst(types::I8, i64::from(consume_owner));
    let call = builder
        .ins()
        .call(callee, &[deopt_out, array, additional, consume_owner]);
    let status = builder.inst_results(call)[0];
    let value = builder.inst_results(call)[1];
    let succeeded = builder.ins().icmp_imm(IntCC::Equal, status, 0);
    builder.ins().brif(succeeded, accepted, &[], rejected, &[]);

    builder.switch_to_block(accepted);
    builder.ins().jump(done, &[value.into()]);

    builder.switch_to_block(rejected);
    let placeholder = transition.emit_value(builder)?;
    builder.ins().jump(done, &[placeholder.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}

fn lower_direct_array_child_entry(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    operation: FuncId,
    array: ir::Value,
    key: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<(ir::Value, ir::Value), CraneliftLoweringError> {
    let pointer_type = builder.func.dfg.value_type(transition.deopt_out);
    let accepted = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(done, types::I64);
    builder.append_block_param(done, pointer_type);
    let (value, entry) = lower_direct_array_lookup_child_entry(
        module,
        builder,
        operation,
        array,
        key,
        transition.deopt_out,
    );
    let succeeded = builder.ins().icmp_imm(IntCC::NotEqual, entry, 0);
    builder.ins().brif(succeeded, accepted, &[], rejected, &[]);

    builder.switch_to_block(accepted);
    builder.ins().jump(done, &[value.into(), entry.into()]);

    builder.switch_to_block(rejected);
    let placeholder = transition.emit_value(builder)?;
    let null_entry = builder.ins().iconst(pointer_type, 0);
    builder
        .ins()
        .jump(done, &[placeholder.into(), null_entry.into()]);

    builder.switch_to_block(done);
    Ok((builder.block_params(done)[0], builder.block_params(done)[1]))
}

/// Looks up one entry through the compiled native array primitive without
/// deciding that absence is a transition. Lvalue operations use a null entry
/// as the direct "create this element" case; read-only paths wrap this helper
/// with their exact missing-key continuation.
fn lower_direct_array_lookup_child_entry(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    operation: FuncId,
    array: ir::Value,
    key: ir::Value,
    deopt_out: ir::Value,
) -> (ir::Value, ir::Value) {
    let callee = module.declare_func_in_func(operation, builder.func);
    let call = builder.ins().call(callee, &[deopt_out, array, key]);
    (builder.inst_results(call)[0], builder.inst_results(call)[1])
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_nested_array_path(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    array_ensure_unique: FuncId,
    array_child_entry: FuncId,
    root: ir::Value,
    keys: &[(ir::Value, bool)],
    final_additional: u32,
    result_out: ir::Value,
    deopt_out: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<(ir::Value, ir::Value), CraneliftLoweringError> {
    let root_additional = builder.ins().iconst(
        types::I64,
        if keys.is_empty() {
            i64::from(final_additional)
        } else {
            0
        },
    );
    let root = lower_direct_array_ensure_unique_capacity(
        module,
        builder,
        array_ensure_unique,
        root,
        root_additional,
        true,
        result_out,
        deopt_out,
        transition,
    )?;
    let current = lower_direct_nested_array_path_from_unique_root(
        module,
        builder,
        array_ensure_unique,
        array_child_entry,
        root,
        keys,
        final_additional,
        result_out,
        deopt_out,
        transition,
    )?;
    Ok((root, current))
}

/// Descends through an already unique, already published root. Every child
/// COW replacement is stored into its parent entry before the next operation,
/// so a later exact continuation always observes the authoritative tree.
#[allow(clippy::too_many_arguments)]
fn lower_direct_nested_array_path_from_unique_root(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    array_ensure_unique: FuncId,
    array_child_entry: FuncId,
    root: ir::Value,
    keys: &[(ir::Value, bool)],
    final_additional: u32,
    result_out: ir::Value,
    deopt_out: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let mut current = root;
    for (index, (key, _)) in keys.iter().copied().enumerate() {
        lower_direct_array_require_supported_key(builder, key, transition)?;
        let (previous_child, parent_entry) = lower_direct_array_child_entry(
            module,
            builder,
            array_child_entry,
            current,
            key,
            transition,
        )?;
        let additional = builder.ins().iconst(
            types::I64,
            if index + 1 == keys.len() {
                i64::from(final_additional)
            } else {
                0
            },
        );
        let child = lower_direct_array_ensure_unique_capacity(
            module,
            builder,
            array_ensure_unique,
            previous_child,
            additional,
            false,
            result_out,
            deopt_out,
            transition,
        )?;
        let unchanged = builder.ins().icmp(IntCC::Equal, previous_child, child);
        let replace = builder.create_block();
        let descend = builder.create_block();
        builder.ins().brif(unchanged, descend, &[], replace, &[]);
        builder.switch_to_block(replace);
        let previous_slot =
            lower_optimizing_slot_address(builder, previous_child, transition.deopt_out);
        let previous_refcount = builder.ins().load(
            types::I32,
            MemFlagsData::new(),
            previous_slot,
            std::mem::offset_of!(crate::JitNativeValueSlot, refcount) as i32,
        );
        let remaining = builder.ins().iadd_imm(previous_refcount, -1);
        builder.ins().store(
            MemFlagsData::new(),
            remaining,
            previous_slot,
            std::mem::offset_of!(crate::JitNativeValueSlot, refcount) as i32,
        );
        builder.ins().store(
            MemFlagsData::new(),
            child,
            parent_entry,
            std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
        );
        builder.ins().jump(descend, &[]);
        builder.switch_to_block(descend);
        current = child;
    }
    Ok(current)
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_array_spread(
    module: &mut JITModule,
    builder: &mut FunctionBuilder<'_>,
    array_ensure_unique: FuncId,
    array: ir::Value,
    source: ir::Value,
    result_out: ir::Value,
    deopt_out: ir::Value,
    transition: NativeOptimizingTransition<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let pointer_type = builder.func.dfg.value_type(deopt_out);
    let inspect = builder.create_block();
    let scan_target = builder.create_block();
    let scan_target_entry = builder.create_block();
    let scan_source = builder.create_block();
    let inspect_source_entry = builder.create_block();
    let source_integer = builder.create_block();
    let source_string = builder.create_block();
    let scan_collision = builder.create_block();
    let compare_collision = builder.create_block();
    let collision_found = builder.create_block();
    let collision_release = builder.create_block();
    let advance_source = builder.create_block();
    let preflight_done = builder.create_block();
    let reserve = builder.create_block();
    let mutate = builder.create_block();
    let mutate_entry = builder.create_block();
    let mutate_integer = builder.create_block();
    let mutate_string = builder.create_block();
    let mutate_next = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(scan_target, types::I64);
    builder.append_block_param(scan_target, types::I64);
    builder.append_block_param(scan_target, types::I8);
    builder.append_block_param(scan_source, types::I64);
    builder.append_block_param(scan_source, types::I64);
    builder.append_block_param(scan_source, types::I64);
    builder.append_block_param(scan_source, types::I8);
    builder.append_block_param(scan_collision, types::I64);
    builder.append_block_param(advance_source, types::I64);
    builder.append_block_param(advance_source, types::I64);
    builder.append_block_param(advance_source, types::I64);
    builder.append_block_param(advance_source, types::I8);
    builder.append_block_param(preflight_done, types::I64);
    builder.append_block_param(preflight_done, types::I64);
    builder.append_block_param(preflight_done, types::I8);
    builder.append_block_param(mutate, types::I64);
    builder.append_block_param(mutate, types::I64);
    builder.append_block_param(mutate_next, types::I64);
    builder.append_block_param(mutate_next, types::I64);
    builder.append_block_param(done, types::I64);

    let array_tag = lower_value_has_tag(builder, array, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let source_tag = lower_value_has_tag(builder, source, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let array_index = builder.ins().ireduce(types::I32, array);
    let source_index = builder.ins().ireduce(types::I32, source);
    let array_direct = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        array_index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let source_direct = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        source_index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let distinct = builder.ins().icmp(IntCC::NotEqual, array, source);
    let admitted = builder.ins().band(array_tag, source_tag);
    let admitted = builder.ins().band(admitted, array_direct);
    let admitted = builder.ins().band(admitted, source_direct);
    let admitted = builder.ins().band(admitted, distinct);
    builder.ins().brif(admitted, inspect, &[], rejected, &[]);

    builder.switch_to_block(inspect);
    let array_slot = lower_optimizing_slot_address(builder, array, deopt_out);
    let source_slot = lower_optimizing_slot_address(builder, source, deopt_out);
    let array_kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        array_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let source_kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        source_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let array_kind_ok = builder.ins().icmp_imm(
        IntCC::Equal,
        array_kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    let source_kind_ok = builder.ins().icmp_imm(
        IntCC::Equal,
        source_kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    let admitted = builder.ins().band(array_kind_ok, source_kind_ok);
    let zero = builder.ins().iconst(types::I64, 0);
    let no = builder.ins().iconst(types::I8, 0);
    let array_length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        array_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let source_length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        source_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let array_entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        array_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let source_entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        source_slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    builder.ins().brif(
        admitted,
        scan_target,
        &[zero.into(), zero.into(), no.into()],
        rejected,
        &[],
    );

    // Establish the target's greatest integer key once. Numeric spread keys
    // are discarded and appended from this sequence, including PHP 8.3+
    // negative-key continuation semantics.
    builder.switch_to_block(scan_target);
    let target_index = builder.block_params(scan_target)[0];
    let greatest = builder.block_params(scan_target)[1];
    let found_integer = builder.block_params(scan_target)[2];
    let target_done = builder.ins().icmp(
        IntCC::UnsignedGreaterThanOrEqual,
        target_index,
        array_length,
    );
    builder.ins().brif(
        target_done,
        scan_source,
        &[
            zero.into(),
            zero.into(),
            greatest.into(),
            found_integer.into(),
        ],
        scan_target_entry,
        &[],
    );

    builder.switch_to_block(scan_target_entry);
    let wide_target_index = if pointer_type == types::I64 {
        target_index
    } else {
        builder.ins().ireduce(pointer_type, target_index)
    };
    let target_offset = builder.ins().ishl_imm(wide_target_index, 4);
    let target_entry = builder.ins().iadd(array_entries, target_offset);
    let target_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), target_entry, 0);
    let (target_integer, target_raw) =
        lower_native_array_key_integer_candidate(builder, target_key, deopt_out);
    let greater = builder
        .ins()
        .icmp(IntCC::SignedGreaterThan, target_raw, greatest);
    let first = builder.ins().icmp_imm(IntCC::Equal, found_integer, 0);
    let replace = builder.ins().bor(first, greater);
    let replace = builder.ins().band(target_integer, replace);
    let greatest = builder.ins().select(replace, target_raw, greatest);
    let found_integer = builder.ins().bor(found_integer, target_integer);
    let next_target = builder.ins().iadd_imm(target_index, 1);
    builder.ins().jump(
        scan_target,
        &[next_target.into(), greatest.into(), found_integer.into()],
    );

    // Validate every source key and every overwrite release before changing
    // the target. This makes the operation's one continuation restart-safe.
    builder.switch_to_block(scan_source);
    let spread_index = builder.block_params(scan_source)[0];
    let numeric_count = builder.block_params(scan_source)[1];
    let greatest = builder.block_params(scan_source)[2];
    let found_integer = builder.block_params(scan_source)[3];
    let source_done = builder.ins().icmp(
        IntCC::UnsignedGreaterThanOrEqual,
        spread_index,
        source_length,
    );
    builder.ins().brif(
        source_done,
        preflight_done,
        &[numeric_count.into(), greatest.into(), found_integer.into()],
        inspect_source_entry,
        &[],
    );

    builder.switch_to_block(inspect_source_entry);
    let wide_spread_index = if pointer_type == types::I64 {
        spread_index
    } else {
        builder.ins().ireduce(pointer_type, spread_index)
    };
    let source_offset = builder.ins().ishl_imm(wide_spread_index, 4);
    let source_entry = builder.ins().iadd(source_entries, source_offset);
    let source_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), source_entry, 0);
    let source_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        source_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let source_value =
        lower_optimizing_reference_scalar(builder, source_value, false, transition)?;
    let source_is_integer =
        lower_native_array_key_integer_candidate(builder, source_key, deopt_out).0;
    let (source_is_string, _, _) =
        lower_native_string_key_descriptor(builder, source_key, deopt_out);
    builder
        .ins()
        .brif(source_is_integer, source_integer, &[], source_string, &[]);

    builder.switch_to_block(source_integer);
    let next_source = builder.ins().iadd_imm(spread_index, 1);
    let next_numeric_count = builder.ins().iadd_imm(numeric_count, 1);
    builder.ins().jump(
        scan_source,
        &[
            next_source.into(),
            next_numeric_count.into(),
            greatest.into(),
            found_integer.into(),
        ],
    );

    builder.switch_to_block(source_string);
    builder.ins().brif(
        source_is_string,
        scan_collision,
        &[zero.into()],
        rejected,
        &[],
    );

    builder.switch_to_block(scan_collision);
    let collision_index = builder.block_params(scan_collision)[0];
    let collision_done = builder.ins().icmp(
        IntCC::UnsignedGreaterThanOrEqual,
        collision_index,
        array_length,
    );
    let inspect_collision = builder.create_block();
    let next_source = builder.ins().iadd_imm(spread_index, 1);
    builder.ins().brif(
        collision_done,
        advance_source,
        &[
            next_source.into(),
            numeric_count.into(),
            greatest.into(),
            found_integer.into(),
        ],
        inspect_collision,
        &[],
    );

    builder.switch_to_block(inspect_collision);
    let wide_collision_index = if pointer_type == types::I64 {
        collision_index
    } else {
        builder.ins().ireduce(pointer_type, collision_index)
    };
    let collision_offset = builder.ins().ishl_imm(wide_collision_index, 4);
    let collision_entry = builder.ins().iadd(array_entries, collision_offset);
    let collision_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), collision_entry, 0);
    let equal = lower_native_array_key_equal(builder, collision_key, source_key, deopt_out);
    builder
        .ins()
        .brif(equal, collision_found, &[], compare_collision, &[]);

    builder.switch_to_block(compare_collision);
    let next_collision = builder.ins().iadd_imm(collision_index, 1);
    builder.ins().jump(scan_collision, &[next_collision.into()]);

    builder.switch_to_block(collision_found);
    let old_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        collision_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let unchanged = builder.ins().icmp(IntCC::Equal, old_value, source_value);
    let next_source = builder.ins().iadd_imm(spread_index, 1);
    builder.ins().brif(
        unchanged,
        advance_source,
        &[
            next_source.into(),
            numeric_count.into(),
            greatest.into(),
            found_integer.into(),
        ],
        collision_release,
        &[],
    );

    builder.switch_to_block(collision_release);
    let validate = builder
        .ins()
        .call(transition.value_release_validate, &[deopt_out, old_value]);
    let releasable = builder.inst_results(validate)[0];
    let next_source = builder.ins().iadd_imm(spread_index, 1);
    builder.ins().brif(
        releasable,
        advance_source,
        &[
            next_source.into(),
            numeric_count.into(),
            greatest.into(),
            found_integer.into(),
        ],
        rejected,
        &[],
    );

    builder.switch_to_block(advance_source);
    let next_source = builder.block_params(advance_source)[0];
    let numeric_count = builder.block_params(advance_source)[1];
    let greatest = builder.block_params(advance_source)[2];
    let found_integer = builder.block_params(advance_source)[3];
    builder.ins().jump(
        scan_source,
        &[
            next_source.into(),
            numeric_count.into(),
            greatest.into(),
            found_integer.into(),
        ],
    );

    builder.switch_to_block(preflight_done);
    let numeric_count = builder.block_params(preflight_done)[0];
    let greatest = builder.block_params(preflight_done)[1];
    let found_integer = builder.block_params(preflight_done)[2];
    let has_numeric = builder.ins().icmp_imm(IntCC::NotEqual, numeric_count, 0);
    let maximum = builder.ins().iconst(types::I64, i64::MAX);
    let maximum_start = builder.ins().isub(maximum, numeric_count);
    let greatest_fits = builder
        .ins()
        .icmp(IntCC::SignedLessThanOrEqual, greatest, maximum_start);
    let needs_bound = builder.ins().band(found_integer, has_numeric);
    let overflow = builder.ins().band_not(needs_bound, greatest_fits);
    builder.ins().brif(overflow, rejected, &[], reserve, &[]);

    builder.switch_to_block(reserve);
    let current = lower_direct_array_ensure_unique_capacity(
        module,
        builder,
        array_ensure_unique,
        array,
        source_length,
        true,
        result_out,
        deopt_out,
        transition,
    )?;
    builder.ins().jump(mutate, &[zero.into(), current.into()]);

    builder.switch_to_block(mutate);
    let spread_index = builder.block_params(mutate)[0];
    let current = builder.block_params(mutate)[1];
    let finished = builder.ins().icmp(
        IntCC::UnsignedGreaterThanOrEqual,
        spread_index,
        source_length,
    );
    builder
        .ins()
        .brif(finished, done, &[current.into()], mutate_entry, &[]);

    builder.switch_to_block(mutate_entry);
    let wide_spread_index = if pointer_type == types::I64 {
        spread_index
    } else {
        builder.ins().ireduce(pointer_type, spread_index)
    };
    let source_offset = builder.ins().ishl_imm(wide_spread_index, 4);
    let source_entry = builder.ins().iadd(source_entries, source_offset);
    let key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), source_entry, 0);
    let value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        source_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let value = lower_optimizing_reference_scalar(builder, value, false, transition)?;
    let integer = lower_native_array_key_integer_candidate(builder, key, transition.deopt_out).0;
    builder
        .ins()
        .brif(integer, mutate_integer, &[], mutate_string, &[]);

    builder.switch_to_block(mutate_integer);
    let updated = lower_direct_array_append(
        module,
        builder,
        current,
        None,
        value,
        false,
        result_out,
        deopt_out,
        NativeArrayAppendFallback::Optimizing(transition),
    )?;
    let next_spread = builder.ins().iadd_imm(spread_index, 1);
    builder
        .ins()
        .jump(mutate_next, &[next_spread.into(), updated.into()]);

    builder.switch_to_block(mutate_string);
    let updated = lower_direct_array_insert(
        module,
        builder,
        current,
        key,
        true,
        value,
        false,
        result_out,
        deopt_out,
        NativeArrayAppendFallback::Optimizing(transition),
    )?;
    let next_spread = builder.ins().iadd_imm(spread_index, 1);
    builder
        .ins()
        .jump(mutate_next, &[next_spread.into(), updated.into()]);

    builder.switch_to_block(mutate_next);
    let spread_index = builder.block_params(mutate_next)[0];
    let current = builder.block_params(mutate_next)[1];
    builder
        .ins()
        .jump(mutate, &[spread_index.into(), current.into()]);

    builder.switch_to_block(rejected);
    let placeholder = transition.emit_value(builder)?;
    builder.ins().jump(done, &[placeholder.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}

fn lower_direct_array_unset(
    builder: &mut FunctionBuilder<'_>,
    array: ir::Value,
    key: ir::Value,
    constant_string_key: bool,
    transition: NativeOptimizingTransition<'_>,
) -> Result<ir::Value, CraneliftLoweringError> {
    let pointer_type = builder.func.dfg.value_type(transition.deopt_out);
    let inspect = builder.create_block();
    let search = builder.create_block();
    let compare = builder.create_block();
    let next = builder.create_block();
    let found = builder.create_block();
    let release = builder.create_block();
    let shift = builder.create_block();
    let shift_entry = builder.create_block();
    let finish = builder.create_block();
    let rejected = builder.create_block();
    let done = builder.create_block();
    builder.append_block_param(search, types::I64);
    builder.append_block_param(next, types::I64);
    builder.append_block_param(found, types::I64);
    builder.append_block_param(release, pointer_type);
    builder.append_block_param(release, types::I64);
    builder.append_block_param(shift, types::I64);
    builder.append_block_param(done, types::I64);

    let array_tag = lower_value_has_tag(builder, array, crate::JIT_VALUE_RUNTIME_ARRAY_TAG);
    let encoded_index = builder.ins().ireduce(types::I32, array);
    let direct_index = builder.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        encoded_index,
        i64::from(crate::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE),
    );
    let direct = builder.ins().band(array_tag, direct_index);
    builder.ins().brif(direct, inspect, &[], rejected, &[]);

    builder.switch_to_block(inspect);
    let slot = lower_optimizing_slot_address(builder, array, transition.deopt_out);
    let kind = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, kind) as i32,
    );
    let refcount = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, refcount) as i32,
    );
    let direct_kind = builder.ins().icmp_imm(
        IntCC::Equal,
        kind,
        i64::from(crate::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY),
    );
    let unique = builder.ins().icmp_imm(IntCC::Equal, refcount, 1);
    let integer = lower_optimizing_integer_candidate(builder, key, transition.deopt_out).0;
    let (string, _, _) = lower_native_string_key_descriptor(builder, key, transition.deopt_out);
    let supported_key = builder.ins().bor(integer, string);
    let _ = constant_string_key;
    let admitted = builder.ins().band(direct_kind, unique);
    let admitted = builder.ins().band(admitted, supported_key);
    let zero = builder.ins().iconst(types::I64, 0);
    builder
        .ins()
        .brif(admitted, search, &[zero.into()], rejected, &[]);

    builder.switch_to_block(search);
    let search_index = builder.block_params(search)[0];
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let exhausted = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, search_index, length);
    builder
        .ins()
        .brif(exhausted, done, &[array.into()], compare, &[]);

    builder.switch_to_block(compare);
    let entries = builder.ins().load(
        pointer_type,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, aux) as i32,
    );
    let wide_index = if pointer_type == types::I64 {
        search_index
    } else {
        builder.ins().ireduce(pointer_type, search_index)
    };
    let entry_offset = builder.ins().ishl_imm(wide_index, 4);
    let entry = builder.ins().iadd(entries, entry_offset);
    let candidate = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), entry, 0);
    let matches = lower_native_array_key_equal(builder, candidate, key, transition.deopt_out);
    builder.ins().brif(
        matches,
        found,
        &[search_index.into()],
        next,
        &[search_index.into()],
    );

    builder.switch_to_block(next);
    let current_index = builder.block_params(next)[0];
    let next_index = builder.ins().iadd_imm(current_index, 1);
    builder.ins().jump(search, &[next_index.into()]);

    builder.switch_to_block(found);
    let found_index = builder.block_params(found)[0];
    let found_wide = if pointer_type == types::I64 {
        found_index
    } else {
        builder.ins().ireduce(pointer_type, found_index)
    };
    let found_offset = builder.ins().ishl_imm(found_wide, 4);
    let found_entry = builder.ins().iadd(entries, found_offset);
    let removed_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), found_entry, 0);
    let removed_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        found_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let validate_key = builder.ins().call(
        transition.value_release_validate,
        &[transition.deopt_out, removed_key],
    );
    let key_releasable = builder.inst_results(validate_key)[0];
    let validate_value = builder.ins().call(
        transition.value_release_validate,
        &[transition.deopt_out, removed_value],
    );
    let value_releasable = builder.inst_results(validate_value)[0];
    let releasable = builder.ins().band(key_releasable, value_releasable);
    builder.ins().brif(
        releasable,
        release,
        &[found_entry.into(), found_index.into()],
        rejected,
        &[],
    );

    builder.switch_to_block(release);
    let found_entry = builder.block_params(release)[0];
    let found_index = builder.block_params(release)[1];
    let removed_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), found_entry, 0);
    let removed_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        found_entry,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let _ = builder.ins().call(
        transition.value_release_commit,
        &[transition.deopt_out, removed_key],
    );
    let _ = builder.ins().call(
        transition.value_release_commit,
        &[transition.deopt_out, removed_value],
    );
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let flags = builder.ins().load(
        types::I32,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, flags) as i32,
    );
    let cursor = builder
        .ins()
        .ushr_imm(flags, crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_SHIFT as i64);
    let cursor_wide = builder.ins().uextend(types::I64, cursor);
    let cursor_present = builder.ins().icmp_imm(
        IntCC::NotEqual,
        cursor,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_NONE),
    );
    let cursor_in_bounds = builder
        .ins()
        .icmp(IntCC::UnsignedLessThan, cursor_wide, length);
    let cursor_present = builder.ins().band(cursor_present, cursor_in_bounds);
    let found_narrow = builder.ins().ireduce(types::I32, found_index);
    let after_removed = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThan, cursor, found_narrow);
    let shifted_cursor = builder.ins().iadd_imm(cursor, -1);
    let adjusted = builder.ins().select(after_removed, shifted_cursor, cursor);
    let new_length = builder.ins().iadd_imm(length, -1);
    let new_length_narrow = builder.ins().ireduce(types::I32, new_length);
    let removed_current = builder.ins().icmp(IntCC::Equal, cursor, found_narrow);
    let removed_last = builder.ins().icmp(
        IntCC::UnsignedGreaterThanOrEqual,
        found_narrow,
        new_length_narrow,
    );
    let current_was_last = builder.ins().band(removed_current, removed_last);
    let none = builder.ins().iconst(
        types::I32,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_NONE),
    );
    let adjusted = builder.ins().select(current_was_last, none, adjusted);
    let adjusted = builder.ins().select(cursor_present, adjusted, none);
    let packed = builder
        .ins()
        .ishl_imm(adjusted, crate::JIT_NATIVE_DIRECT_ARRAY_CURSOR_SHIFT as i64);
    let packed = builder.ins().bor_imm(
        packed,
        i64::from(crate::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION),
    );
    builder.ins().store(
        MemFlagsData::new(),
        packed,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, flags) as i32,
    );
    let first_shift = builder.ins().iadd_imm(found_index, 1);
    builder.ins().jump(shift, &[first_shift.into()]);

    builder.switch_to_block(shift);
    let source_index = builder.block_params(shift)[0];
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let shifted_all = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, source_index, length);
    builder
        .ins()
        .brif(shifted_all, finish, &[], shift_entry, &[]);

    builder.switch_to_block(shift_entry);
    let source_wide = if pointer_type == types::I64 {
        source_index
    } else {
        builder.ins().ireduce(pointer_type, source_index)
    };
    let source_offset = builder.ins().ishl_imm(source_wide, 4);
    let source = builder.ins().iadd(entries, source_offset);
    let destination = builder.ins().iadd_imm(source, -16);
    let moved_key = builder
        .ins()
        .load(types::I64, MemFlagsData::new(), source, 0);
    let moved_value = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        source,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    builder
        .ins()
        .store(MemFlagsData::new(), moved_key, destination, 0);
    builder.ins().store(
        MemFlagsData::new(),
        moved_value,
        destination,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    let next_source = builder.ins().iadd_imm(source_index, 1);
    builder.ins().jump(shift, &[next_source.into()]);

    builder.switch_to_block(finish);
    let length = builder.ins().load(
        types::I64,
        MemFlagsData::new(),
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let new_length = builder.ins().iadd_imm(length, -1);
    builder.ins().store(
        MemFlagsData::new(),
        new_length,
        slot,
        std::mem::offset_of!(crate::JitNativeValueSlot, payload) as i32,
    );
    let last_wide = if pointer_type == types::I64 {
        new_length
    } else {
        builder.ins().ireduce(pointer_type, new_length)
    };
    let last_offset = builder.ins().ishl_imm(last_wide, 4);
    let last = builder.ins().iadd(entries, last_offset);
    let zero = builder.ins().iconst(types::I64, 0);
    builder.ins().store(MemFlagsData::new(), zero, last, 0);
    builder.ins().store(
        MemFlagsData::new(),
        zero,
        last,
        std::mem::offset_of!(crate::JitNativeDirectArrayEntry, value) as i32,
    );
    builder.ins().jump(done, &[array.into()]);

    builder.switch_to_block(rejected);
    let placeholder = transition.emit_value(builder)?;
    builder.ins().jump(done, &[placeholder.into()]);

    builder.switch_to_block(done);
    Ok(builder.block_params(done)[0])
}
