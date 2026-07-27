use super::native_builtins::format_native_php_diagnostic;
use super::{dereference_native_callable_value, native_backtrace_frame};

#[test]
fn exact_execution_poll_uses_only_published_deadline_capability() {
    use php_ir::builder::IrBuilder;
    use php_ir::{FunctionFlags, IrSpan, UnitId};

    let mut builder = IrBuilder::new(UnitId::new(9_941));
    let file = builder.add_file("native-exact-deadline-poll.php");
    let span = IrSpan::new(file, 0, 16);
    let entry = builder.start_function("main", FunctionFlags::default(), span);
    let block = builder.append_block(entry);
    builder.terminate_return(entry, block, None, span);
    builder.set_entry(entry);
    let compiled = crate::compiled_unit::CompiledUnit::new(builder.finish());
    let options = super::super::VmOptions {
        runtime_context: php_runtime::api::RuntimeContext::controlled_cli(
            "native-exact-deadline-poll.php",
            Vec::new(),
        )
        .with_execution_time_limit(Some(std::time::Duration::ZERO)),
        ..super::super::VmOptions::default()
    };
    let worker = super::super::VmWorkerState::default();
    let mut context = super::NativeRequestOwner::new(
        &compiled,
        compiled.artifact_identity(),
        &options,
        &worker,
        php_runtime::api::OutputBuffer::new(),
        std::sync::Arc::new(std::collections::BTreeMap::new()),
    );
    let runtime = context.fast_state;
    let _activation = super::activate_native_context(&mut context);

    let status = super::runtime_ops::jit_native_execution_poll_abi(runtime);

    assert_eq!(status, php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32);
    assert_eq!(
        context
            .diagnostic
            .as_ref()
            .map(|diagnostic| diagnostic.id()),
        Some("E_PHP_VM_EXECUTION_TIMEOUT")
    );
}

#[test]
fn root_deployment_attachment_publishes_its_dynamic_execution_scope() {
    use php_ir::builder::IrBuilder;
    use php_ir::{FunctionFlags, IrSpan, UnitId};

    let mut builder = IrBuilder::new(UnitId::new(9_942));
    let file = builder.add_file("native-root-execution-scope.php");
    let span = IrSpan::new(file, 0, 16);
    let entry = builder.start_function("main", FunctionFlags::default(), span);
    let block = builder.append_block(entry);
    builder.terminate_return(entry, block, None, span);
    builder.set_entry(entry);
    let compiled = crate::compiled_unit::CompiledUnit::new(builder.finish());
    let options = super::super::VmOptions::default();
    let worker = super::super::VmWorkerState::default();
    let mut context = super::NativeRequestOwner::new(
        &compiled,
        compiled.artifact_identity(),
        &options,
        &worker,
        php_runtime::api::OutputBuffer::new(),
        std::sync::Arc::new(std::collections::BTreeMap::new()),
    );

    context.attach_root_deployment_image(compiled.clone());

    assert_eq!(context.current_dynamic_unit, Some(0));
    let scope = context
        .native_execution_scopes
        .get(context.current_native_execution_scope as usize - 1)
        .expect("attached deployment keeps a published execution scope");
    assert_eq!(
        scope.unit,
        Some(0),
        "closures created by the root deployment must retain its unit-local function owner"
    );
}

#[test]
fn native_root_mutation_invalidates_cross_unit_graph_cache() {
    use php_ir::builder::IrBuilder;
    use php_ir::{FunctionFlags, IrSpan, UnitId};

    let mut builder = IrBuilder::new(UnitId::new(9_943));
    let file = builder.add_file("native-root-mutation.php");
    let span = IrSpan::new(file, 0, 16);
    let entry = builder.start_function("main", FunctionFlags::default(), span);
    let block = builder.append_block(entry);
    builder.terminate_return(entry, block, None, span);
    builder.set_entry(entry);
    let compiled = crate::compiled_unit::CompiledUnit::new(builder.finish());
    let options = super::super::VmOptions::default();
    let worker = super::super::VmWorkerState::default();
    let mut context = super::NativeRequestOwner::new(
        &compiled,
        compiled.artifact_identity(),
        &options,
        &worker,
        php_runtime::api::OutputBuffer::new(),
        std::sync::Arc::new(std::collections::BTreeMap::new()),
    );

    context.cross_unit_stable_values.extend([7, 11]);
    *context.native_root_mutation_pending = 1;
    context.consume_native_root_mutation();

    assert!(
        context.cross_unit_stable_values.is_empty(),
        "a native store may have inserted a new unit-local literal"
    );
    assert_eq!(*context.native_root_mutation_pending, 0);
}

#[test]
fn shutdown_object_sweep_balances_native_receiver_ownership() {
    use php_ir::builder::IrBuilder;
    use php_ir::{FunctionFlags, IrSpan, UnitId};

    let mut builder = IrBuilder::new(UnitId::new(9_944));
    let file = builder.add_file("native-shutdown-object-sweep.php");
    let span = IrSpan::new(file, 0, 16);
    let entry = builder.start_function("main", FunctionFlags::default(), span);
    let block = builder.append_block(entry);
    builder.terminate_return(entry, block, None, span);
    builder.set_entry(entry);
    let compiled = crate::compiled_unit::CompiledUnit::new(builder.finish());
    let options = super::super::VmOptions::default();
    let worker = super::super::VmWorkerState::default();
    let mut context = super::NativeRequestOwner::new(
        &compiled,
        compiled.artifact_identity(),
        &options,
        &worker,
        php_runtime::api::OutputBuffer::new(),
        std::sync::Arc::new(std::collections::BTreeMap::new()),
    );
    let class = php_runtime::api::ClassEntry {
        name: "PlainShutdownObject".to_owned().into(),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: php_runtime::api::ClassFlags::default(),
    };
    let objects = (0..3)
        .map(|_| php_runtime::api::ObjectRef::new(&class))
        .collect::<Vec<_>>();
    let encoded = objects
        .iter()
        .cloned()
        .map(|object| {
            context
                .encode_native_object_owner(object)
                .expect("plain object enters the authoritative native plane")
        })
        .collect::<Vec<_>>();
    let indices = encoded
        .iter()
        .map(|value| {
            php_jit::jit_decode_runtime_value(*value)
                .expect("direct object runtime index")
                .checked_sub(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)
                .expect("direct object slot index") as usize
        })
        .collect::<Vec<_>>();
    assert!(
        indices
            .iter()
            .all(|index| context.direct_value_slots[*index].refcount == 1)
    );

    context
        .run_shutdown_callbacks()
        .expect("native shutdown object sweep");

    assert!(
        indices
            .iter()
            .all(|index| context.direct_value_slots[*index].refcount == 1),
        "shutdown must release each temporary destructor receiver"
    );
    assert!(
        context.destroyed_objects.is_empty(),
        "objects without __destruct need no shutdown publication"
    );
    assert!(context.shutdown_destructor_queue.is_none());

    context
        .run_shutdown_callbacks()
        .expect("repeated shutdown sweep is idempotent");
    assert!(
        indices
            .iter()
            .all(|index| context.direct_value_slots[*index].refcount == 1)
    );

    for value in encoded {
        context
            .release(value)
            .expect("test releases the original native object owner");
    }
}

#[test]
#[allow(unsafe_code)]
fn object_cast_maps_authoritative_array_properties_and_preserves_identity() {
    let mut slots =
        vec![php_jit::JitNativeValueSlot::default(); php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY];
    let mut owners = vec![0_u64; php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY];
    let key = b"first";
    let value = b"A";
    let array_value = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
    );
    let key_value = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 1,
        php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
    );
    let string_value = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 2,
        php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
    );
    let entries = [
        php_jit::JitNativeDirectArrayEntry {
            key: key_value,
            value: string_value,
        },
        php_jit::JitNativeDirectArrayEntry { key: 7, value: 42 },
    ];
    slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        payload: entries.len() as u64,
        aux: entries.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: key.len() as u64,
        aux: key.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    slots[2] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: value.len() as u64,
        aux: value.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let mut next = 3_u32;
    let mut free_head = php_jit::JIT_NATIVE_DIRECT_ARRAY_FREE_NONE;
    let mut reused_bytes = 0_u64;
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                direct_value_next: std::ptr::from_mut(&mut next) as usize as u64,
                direct_value_free_head: std::ptr::from_mut(&mut free_head) as usize as u64,
                direct_value_reused_bytes: std::ptr::from_mut(&mut reused_bytes) as usize as u64,
                direct_object_owners: owners.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };

    let cast = super::runtime_ops::jit_native_object_cast_abi(
        std::ptr::from_mut(&mut fast_state),
        array_value,
    );
    assert_eq!(cast.status, php_jit::JitCallStatus::RETURN);
    let object = fast_state
        .direct_object(cast.value)
        .expect("cast result owns a direct object")
        .clone();
    let layout_id = object.class_layout_epoch();
    assert_eq!(
        object.native_dynamic_property_slot(layout_id, "first"),
        Some(Some(php_runtime::api::NativeDeclaredPropertySlot {
            initialized: 1,
            reserved: 0,
            value: string_value,
        }))
    );
    assert_eq!(
        object.native_dynamic_property_slot(layout_id, "7"),
        Some(Some(php_runtime::api::NativeDeclaredPropertySlot {
            initialized: 1,
            reserved: 0,
            value: 42,
        }))
    );
    let order = object
        .with_native_comparison_view(layout_id, |_, _, _, dynamic_order, _| {
            dynamic_order.to_vec()
        })
        .expect("cast object keeps authoritative native properties");
    assert_eq!(order, ["first", "7"]);
    assert_eq!(slots[2].refcount, 2);

    let identity = super::runtime_ops::jit_native_object_cast_abi(
        std::ptr::from_mut(&mut fast_state),
        cast.value,
    );
    assert_eq!(identity.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(identity.value, cast.value);
    let object_index = php_jit::jit_decode_runtime_value(cast.value)
        .expect("object runtime index")
        .checked_sub(php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE)
        .expect("direct object index") as usize;
    assert_eq!(slots[object_index].refcount, 2);

    for owner in owners.into_iter().filter(|owner| *owner != 0) {
        unsafe {
            drop(Box::from_raw(
                owner as usize as *mut php_runtime::api::ObjectRef,
            ));
        }
    }
}

#[test]
#[allow(unsafe_code)]
fn dynamic_property_slot_resolver_reserves_one_stable_stdclass_tombstone() {
    let class = php_runtime::api::ClassEntry {
        name: "stdClass".to_owned().into(),
        parent: None,
        interfaces: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        constants: Vec::new(),
        enum_cases: Vec::new(),
        attributes: Vec::new(),
        enum_backing_type: None,
        constructor_id: None,
        flags: php_runtime::api::ClassFlags::default(),
    };
    let object = php_runtime::api::ObjectRef::new(&class);
    let layout_id = object.class_layout_epoch();
    let _ = object
        .take_property_slots_for_native(layout_id)
        .expect("fresh stdClass enters native storage");
    object
        .install_native_property_slots(layout_id, Box::new([]), Default::default())
        .expect("stdClass native slots install");

    let property = b"created";
    let mut slots = vec![php_jit::JitNativeValueSlot::default(); 2];
    let (declared_slots, declared_count) = object
        .native_declared_slots_view(layout_id)
        .expect("native declared view");
    slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT,
        flags: php_jit::JIT_NATIVE_OBJECT_PROPERTY_VIEW_ABI_VERSION,
        reserved: u32::try_from(declared_count).expect("declared count"),
        payload: layout_id,
        aux: declared_slots as usize as u64,
    };
    slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: property.len() as u64,
        aux: property.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let mut owners = vec![0_u64; slots.len()];
    owners[0] = std::ptr::from_ref(&object) as usize as u64;
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                direct_object_owners: owners.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };
    let object_value = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_OBJECT_TAG,
    );
    let property_value = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 1,
        php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
    );
    let first = super::runtime_ops::jit_native_dynamic_property_slot_abi(
        std::ptr::from_mut(&mut fast_state),
        object_value,
        property_value,
    );
    assert_eq!(first.status, php_jit::JitCallStatus::RETURN);
    let cell = first.value as usize as *mut php_runtime::api::NativeDeclaredPropertySlot;
    assert!(!cell.is_null());
    assert_eq!(unsafe { (*cell).initialized }, 0);
    let second = super::runtime_ops::jit_native_dynamic_property_slot_abi(
        std::ptr::from_mut(&mut fast_state),
        object_value,
        property_value,
    );
    assert_eq!(second.value, first.value);
}

#[test]
fn native_http_build_query_reads_recursive_direct_arrays() {
    let mut slots = vec![php_jit::JitNativeValueSlot::default(); 6];
    let array_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
        )
    };
    let string_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
        )
    };
    let hello = b"hello world";
    let nested_key = b"nested key";
    let skipped_key = b"skip";
    for (index, bytes) in [(2, hello.as_slice()), (3, nested_key), (4, skipped_key)] {
        slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
            flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
            payload: bytes.len() as u64,
            aux: bytes.as_ptr() as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
    }
    let nested = [php_jit::JitNativeDirectArrayEntry {
        key: 1,
        value: php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE),
    }];
    slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        payload: nested.len() as u64,
        aux: nested.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let root = [
        php_jit::JitNativeDirectArrayEntry {
            key: 0,
            value: string_value(2),
        },
        php_jit::JitNativeDirectArrayEntry {
            key: string_value(3),
            value: array_value(1),
        },
        php_jit::JitNativeDirectArrayEntry {
            key: string_value(4),
            value: php_jit::jit_encode_constant(u32::MAX),
        },
    ];
    slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        payload: root.len() as u64,
        aux: root.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };
    assert_eq!(
        fast_state
            .native_http_build_query(array_value(0), Some(b"n_"), b";", true)
            .expect("direct query encoding"),
        b"n_0=hello%20world;nested%20key%5B1%5D=1"
    );
}

#[test]
fn exact_parse_str_publishes_keyed_native_array_through_direct_reference() {
    let query = b"plain=value&list[]=a&list[]=b&12=numeric";
    let mut buffers = super::NativeRequestBuffers::default();
    *buffers.direct_value_next = 2;
    buffers.direct_value_slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: query.len() as u64,
        aux: query.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    buffers.direct_value_slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR,
        flags: php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION,
        payload: php_jit::jit_encode_constant(u32::MAX) as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let ini = php_runtime::api::IniRegistry::default();
    let runtime_view = php_jit::JitNativeRuntimeView {
        abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
        direct_value_slots: buffers.direct_value_slots.as_mut_ptr() as usize as u64,
        direct_value_next: std::ptr::from_mut(buffers.direct_value_next.as_mut()) as usize as u64,
        direct_value_free_head: std::ptr::from_mut(buffers.direct_value_free_head.as_mut()) as usize
            as u64,
        direct_value_reused_bytes: std::ptr::from_mut(buffers.direct_value_reused_bytes.as_mut())
            as usize as u64,
        direct_array_states: buffers.direct_array_states.as_mut_ptr() as usize as u64,
        direct_array_entries: buffers.direct_array_entries.as_mut_ptr() as usize as u64,
        direct_array_next: std::ptr::from_mut(buffers.direct_array_next.as_mut()) as usize as u64,
        direct_string_bytes: buffers.direct_string_bytes.as_mut_ptr() as usize as u64,
        direct_string_next: std::ptr::from_mut(buffers.direct_string_next.as_mut()) as usize as u64,
        direct_string_free_heads: buffers.direct_string_free_heads.as_mut_ptr() as usize as u64,
        direct_string_reused_bytes: std::ptr::from_mut(buffers.direct_string_reused_bytes.as_mut())
            as usize as u64,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view,
        },
        ini_registry: std::ptr::from_ref(&ini),
        ..super::NativeRequestFastState::default()
    };
    let input = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
    );
    let output_reference = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 1,
        php_jit::JIT_VALUE_RUNTIME_REFERENCE_TAG,
    );
    let result = super::call_dispatch::jit_native_parse_str_abi(
        std::ptr::from_mut(&mut fast_state),
        2,
        input,
        output_reference,
        0,
        0,
        0,
        0,
    );
    assert_eq!(result.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(result.value, php_jit::jit_encode_constant(u32::MAX));
    let published = buffers.direct_value_slots[1].payload as i64;
    assert_eq!(
        fast_state
            .native_http_build_query(published, None, b"&", false)
            .expect("published parse_str array remains authoritative native data"),
        b"plain=value&list%5B0%5D=a&list%5B1%5D=b&12=numeric"
    );
}

#[test]
fn exact_serialization_roundtrip_never_materializes_the_value_plane() {
    let key = b"x";
    let mut buffers = super::NativeRequestBuffers::default();
    *buffers.direct_value_next = 2;
    *buffers.direct_array_next = 4;
    buffers.direct_array_entries[0] = php_jit::JitNativeDirectArrayEntry { key: 0, value: 7 };
    buffers.direct_array_entries[1] = php_jit::JitNativeDirectArrayEntry {
        key: php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 1,
            php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
        ),
        value: php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE),
    };
    buffers.direct_value_slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        reserved: 4,
        payload: 2,
        aux: buffers.direct_array_entries.as_ptr() as usize as u64,
    };
    buffers.direct_value_slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: key.len() as u64,
        aux: key.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let ini = php_runtime::api::IniRegistry::default();
    let runtime_view = php_jit::JitNativeRuntimeView {
        abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
        direct_value_slots: buffers.direct_value_slots.as_mut_ptr() as usize as u64,
        direct_value_next: std::ptr::from_mut(buffers.direct_value_next.as_mut()) as usize as u64,
        direct_value_free_head: std::ptr::from_mut(buffers.direct_value_free_head.as_mut()) as usize
            as u64,
        direct_value_reused_bytes: std::ptr::from_mut(buffers.direct_value_reused_bytes.as_mut())
            as usize as u64,
        direct_array_states: buffers.direct_array_states.as_mut_ptr() as usize as u64,
        direct_array_entries: buffers.direct_array_entries.as_mut_ptr() as usize as u64,
        direct_array_next: std::ptr::from_mut(buffers.direct_array_next.as_mut()) as usize as u64,
        direct_string_bytes: buffers.direct_string_bytes.as_mut_ptr() as usize as u64,
        direct_string_next: std::ptr::from_mut(buffers.direct_string_next.as_mut()) as usize as u64,
        direct_string_free_heads: buffers.direct_string_free_heads.as_mut_ptr() as usize as u64,
        direct_string_reused_bytes: std::ptr::from_mut(buffers.direct_string_reused_bytes.as_mut())
            as usize as u64,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let mut fast = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view,
        },
        ini_registry: std::ptr::from_ref(&ini),
        ..super::NativeRequestFastState::default()
    };
    let input = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
    );
    let serialized = super::call_dispatch::jit_native_serialize_abi(
        std::ptr::from_mut(&mut fast),
        1,
        input,
        0,
        0,
        0,
        0,
        0,
    );
    assert_eq!(serialized.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        fast.native_string_view(serialized.value)
            .expect("serialize publishes a direct native string"),
        b"a:2:{i:0;i:7;s:1:\"x\";b:1;}"
    );

    let decoded = super::call_dispatch::jit_native_unserialize_abi(
        std::ptr::from_mut(&mut fast),
        1,
        serialized.value,
        0,
        0,
        0,
        0,
        0,
    );
    assert_eq!(decoded.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        fast.native_serialize(decoded.value)
            .expect("unserialize publishes an authoritative direct array"),
        b"a:2:{i:0;i:7;s:1:\"x\";b:1;}"
    );
}

#[test]
fn exact_key_preserving_sorts_reorder_authoritative_entries_in_place() {
    let mut slots = vec![php_jit::JitNativeValueSlot::default(); 8];
    let bytes: [&[u8]; 6] = [b"a", b"c", b"b", b"item10", b"item2", b"item1"];
    for (index, value) in bytes.iter().enumerate() {
        slots[index + 2] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
            flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
            payload: value.len() as u64,
            aux: value.as_ptr() as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
    }
    let string_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
        )
    };
    let mut entries = vec![
        php_jit::JitNativeDirectArrayEntry {
            key: string_value(2),
            value: string_value(5),
        },
        php_jit::JitNativeDirectArrayEntry {
            key: string_value(3),
            value: string_value(6),
        },
        php_jit::JitNativeDirectArrayEntry {
            key: string_value(4),
            value: string_value(7),
        },
    ];
    let array = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
    );
    slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        reserved: 4,
        payload: entries.len() as u64,
        aux: entries.as_mut_ptr() as usize as u64,
    };
    slots[1] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_REFERENCE_SCALAR,
        flags: php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_ABI_VERSION,
        reserved: php_jit::JIT_NATIVE_REFERENCE_SCALAR_VIEW_PUBLISHED,
        payload: array as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let reference = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + 1,
        php_jit::JIT_VALUE_RUNTIME_REFERENCE_TAG,
    );
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };
    let runtime = std::ptr::from_mut(&mut fast_state);
    let result = super::call_dispatch::jit_native_natsort_abi(runtime, 1, reference, 0, 0, 0, 0, 0);
    assert_eq!(result.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        entries
            .iter()
            .map(|entry| fast_state
                .native_string_view(entry.key)
                .expect("string key"))
            .collect::<Vec<_>>(),
        [b"b".as_slice(), b"c".as_slice(), b"a".as_slice()]
    );
    assert_eq!(
        php_jit::jit_native_direct_array_cursor(slots[0].flags),
        Some(0)
    );

    let result = super::call_dispatch::jit_native_krsort_abi(runtime, 1, reference, 0, 0, 0, 0, 0);
    assert_eq!(result.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        entries
            .iter()
            .map(|entry| fast_state
                .native_string_view(entry.key)
                .expect("string key"))
            .collect::<Vec<_>>(),
        [b"c".as_slice(), b"b".as_slice(), b"a".as_slice()]
    );
}

#[test]
fn exact_frame_introspection_keeps_arguments_in_the_native_plane() {
    let arguments = [11_i64, 22_i64];
    let current_fixed_arguments = [33_i64, 44_i64];
    let mut buffers = super::NativeRequestBuffers::default();
    let runtime_view = php_jit::JitNativeRuntimeView {
        abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
        direct_value_slots: buffers.direct_value_slots.as_mut_ptr() as usize as u64,
        direct_value_next: std::ptr::from_mut(buffers.direct_value_next.as_mut()) as usize as u64,
        direct_value_free_head: std::ptr::from_mut(buffers.direct_value_free_head.as_mut()) as usize
            as u64,
        direct_value_reused_bytes: std::ptr::from_mut(buffers.direct_value_reused_bytes.as_mut())
            as usize as u64,
        direct_array_states: buffers.direct_array_states.as_mut_ptr() as usize as u64,
        direct_array_entries: buffers.direct_array_entries.as_mut_ptr() as usize as u64,
        direct_array_next: std::ptr::from_mut(buffers.direct_array_next.as_mut()) as usize as u64,
        direct_array_free_heads: buffers.direct_array_free_heads.as_mut_ptr() as usize as u64,
        direct_array_reused_bytes: std::ptr::from_mut(buffers.direct_array_reused_bytes.as_mut())
            as usize as u64,
        direct_string_bytes: buffers.direct_string_bytes.as_mut_ptr() as usize as u64,
        direct_string_next: std::ptr::from_mut(buffers.direct_string_next.as_mut()) as usize as u64,
        direct_string_free_heads: buffers.direct_string_free_heads.as_mut_ptr() as usize as u64,
        direct_string_reused_bytes: std::ptr::from_mut(buffers.direct_string_reused_bytes.as_mut())
            as usize as u64,
        active_call_arguments: arguments.as_ptr() as usize as u64,
        active_call_argument_count: arguments.len() as u32,
        active_call_fixed_argument_count: current_fixed_arguments.len() as u32,
        active_call_fixed_arguments: current_fixed_arguments.as_ptr() as usize as u64,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let mut fast = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            runtime_view,
            ..php_jit::JitNativeFastStateHeader::default()
        },
        ..super::NativeRequestFastState::default()
    };
    let runtime = std::ptr::from_mut(&mut fast);

    let count = super::call_dispatch::jit_native_func_num_args_abi(runtime, 0, 0, 0, 0, 0, 0, 0);
    assert_eq!(count.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(count.value, 2);

    let argument = super::call_dispatch::jit_native_func_get_arg_abi(runtime, 1, 1, 0, 0, 0, 0, 0);
    assert_eq!(argument.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(argument.value, 44);

    let all = super::call_dispatch::jit_native_func_get_args_abi(runtime, 0, 0, 0, 0, 0, 0, 0);
    assert_eq!(all.status, php_jit::JitCallStatus::RETURN);
    let entries = fast
        .native_direct_array_entries(all.value)
        .expect("func_get_args publishes a direct native array");
    assert_eq!(
        entries,
        [
            php_jit::JitNativeDirectArrayEntry { key: 0, value: 33 },
            php_jit::JitNativeDirectArrayEntry { key: 1, value: 44 },
        ]
    );
}

#[test]
fn exact_frame_introspection_reads_segmented_unpack_tail_arguments() {
    let fixed_arguments = [33_i64];
    let variadic_entries = [
        php_jit::JitNativeDirectArrayEntry { key: 0, value: 44 },
        php_jit::JitNativeDirectArrayEntry { key: 1, value: 55 },
        php_jit::JitNativeDirectArrayEntry { key: 2, value: 66 },
    ];
    let variadic_array = php_jit::jit_encode_typed_runtime_value(
        php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE,
        php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
    );
    let mut buffers = super::NativeRequestBuffers::default();
    *buffers.direct_value_next = 1;
    buffers.direct_value_slots[0] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        payload: variadic_entries.len() as u64,
        aux: variadic_entries.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let runtime_view = php_jit::JitNativeRuntimeView {
        abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
        direct_value_slots: buffers.direct_value_slots.as_mut_ptr() as usize as u64,
        direct_value_next: std::ptr::from_mut(buffers.direct_value_next.as_mut()) as usize as u64,
        direct_value_free_head: std::ptr::from_mut(buffers.direct_value_free_head.as_mut()) as usize
            as u64,
        direct_value_reused_bytes: std::ptr::from_mut(buffers.direct_value_reused_bytes.as_mut())
            as usize as u64,
        direct_array_states: buffers.direct_array_states.as_mut_ptr() as usize as u64,
        direct_array_entries: buffers.direct_array_entries.as_mut_ptr() as usize as u64,
        direct_array_next: std::ptr::from_mut(buffers.direct_array_next.as_mut()) as usize as u64,
        direct_array_free_heads: buffers.direct_array_free_heads.as_mut_ptr() as usize as u64,
        direct_array_reused_bytes: std::ptr::from_mut(buffers.direct_array_reused_bytes.as_mut())
            as usize as u64,
        direct_string_bytes: buffers.direct_string_bytes.as_mut_ptr() as usize as u64,
        direct_string_next: std::ptr::from_mut(buffers.direct_string_next.as_mut()) as usize as u64,
        direct_string_free_heads: buffers.direct_string_free_heads.as_mut_ptr() as usize as u64,
        direct_string_reused_bytes: std::ptr::from_mut(buffers.direct_string_reused_bytes.as_mut())
            as usize as u64,
        active_call_arguments: fixed_arguments.as_ptr() as usize as u64,
        active_call_argument_count: 4,
        active_call_fixed_argument_count: fixed_arguments.len() as u32,
        active_call_tail_arguments: variadic_array,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let mut fast = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            runtime_view,
            ..php_jit::JitNativeFastStateHeader::default()
        },
        ..super::NativeRequestFastState::default()
    };
    let runtime = std::ptr::from_mut(&mut fast);

    let count = super::call_dispatch::jit_native_func_num_args_abi(runtime, 0, 0, 0, 0, 0, 0, 0);
    assert_eq!(count.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(count.value, 4);

    let argument = super::call_dispatch::jit_native_func_get_arg_abi(runtime, 1, 2, 0, 0, 0, 0, 0);
    assert_eq!(argument.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(argument.value, 55);

    let all = super::call_dispatch::jit_native_func_get_args_abi(runtime, 0, 0, 0, 0, 0, 0, 0);
    assert_eq!(all.status, php_jit::JitCallStatus::RETURN);
    let entries = fast
        .native_direct_array_entries(all.value)
        .expect("segmented func_get_args publishes one direct native array");
    assert_eq!(
        entries,
        [
            php_jit::JitNativeDirectArrayEntry { key: 0, value: 33 },
            php_jit::JitNativeDirectArrayEntry { key: 1, value: 44 },
            php_jit::JitNativeDirectArrayEntry { key: 2, value: 55 },
            php_jit::JitNativeDirectArrayEntry { key: 3, value: 66 },
        ]
    );
}

#[test]
fn native_request_pool_reuses_only_reset_worker_owned_buffers() {
    fn assert_send<T: Send>() {}
    assert_send::<super::NativeRequestBuffers>();

    let mut pool = super::NativeRequestPool::default();
    let mut first = pool.checkout(37);
    let direct_value_slots = first.direct_value_slots.as_mut_ptr() as usize;
    let fiber_states = first.fiber_suspension_states.as_mut_ptr() as usize;
    let static_properties = first.static_property_slots.as_mut_ptr() as usize;
    assert!(first.native_call_encoded_scratch.capacity() >= 37);
    first
        .native_call_encoded_scratch
        .extend_from_slice(&[11, 13, 17]);
    first.direct_object_handles.reserve(64);
    let object_handle_capacity = first.direct_object_handles.capacity();
    first.direct_object_handles.clear();
    first.diagnostic_telemetry.counters.runtime_helper_calls = 23;

    pool.recycle(first);
    assert_eq!(pool.available.len(), 1);

    let mut second = pool.checkout(37);
    assert_eq!(
        second.direct_value_slots.as_mut_ptr() as usize,
        direct_value_slots
    );
    assert_eq!(
        second.fiber_suspension_states.as_mut_ptr() as usize,
        fiber_states
    );
    assert_eq!(
        second.static_property_slots.as_mut_ptr() as usize,
        static_properties
    );
    assert!(second.native_call_encoded_scratch.is_empty());
    assert!(second.native_call_encoded_scratch.capacity() >= 37);
    assert_eq!(*second.direct_value_next, 0);
    assert_eq!(*second.direct_array_next, 0);
    assert_eq!(*second.direct_string_next, 0);
    assert_eq!(*second.fiber_suspension_next, 0);
    assert_eq!(*second.static_property_next, 0);
    assert_eq!(second.native_frame_arena.high_water_bytes(), 0);
    assert_eq!(
        second.direct_object_handles.capacity(),
        object_handle_capacity
    );
    assert_eq!(second.diagnostic_telemetry.counters.runtime_helper_calls, 0);
}

#[test]
fn nested_native_activation_restores_the_outer_fast_state_view() {
    let outer_view = php_jit::JitNativeRuntimeView {
        trusted_function_entries: 0x1110,
        trusted_function_entry_count: 30,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let inner_view = php_jit::JitNativeRuntimeView {
        trusted_function_entries: 0x2220,
        trusted_function_entry_count: 64,
        ..php_jit::JitNativeRuntimeView::default()
    };
    let outer_header = php_jit::JitNativeFastStateHeader {
        abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
        flags: 0,
        runtime_view_pointer: 0,
        runtime_view: outer_view,
    };
    let mut fast_state = super::NativeRequestFastState {
        header: outer_header,
        ..super::NativeRequestFastState::default()
    };
    let _outer_runtime_view = php_jit::activate_native_runtime_view(outer_view);
    fast_state.header.runtime_view = inner_view;

    let inner = super::NativeRequestActivationGuard {
        _runtime_view: php_jit::activate_native_runtime_view(inner_view),
        fast_state: std::ptr::from_mut(&mut fast_state),
        previous_header: outer_header,
        previous_execution_scope: std::ptr::null(),
    };
    drop(inner);

    assert_eq!(
        fast_state.header.runtime_view.trusted_function_entries,
        outer_view.trusted_function_entries
    );
    assert_eq!(
        fast_state.header.runtime_view.trusted_function_entry_count,
        outer_view.trusted_function_entry_count
    );
}

#[test]
fn exact_native_array_comparison_handlers_traverse_authoritative_entries() {
    let mut slots =
        vec![php_jit::JitNativeValueSlot::default(); php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY];
    let array_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
        )
    };
    let string_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_STRING_TAG,
        )
    };
    let string_two = b"2";
    slots[7] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: string_two.len() as u64,
        aux: string_two.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let string_key = b"key";
    slots[8] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_STRING,
        flags: php_jit::JIT_NATIVE_STRING_VIEW_ABI_VERSION,
        payload: string_key.len() as u64,
        aux: string_key.as_ptr() as usize as u64,
        ..php_jit::JitNativeValueSlot::default()
    };
    let ordered = vec![
        php_jit::JitNativeDirectArrayEntry { key: 0, value: 1 },
        php_jit::JitNativeDirectArrayEntry {
            key: 1,
            value: string_value(7),
        },
    ]
    .into_boxed_slice();
    let same = ordered.clone();
    let reordered = vec![
        php_jit::JitNativeDirectArrayEntry {
            key: 1,
            value: string_value(7),
        },
        php_jit::JitNativeDirectArrayEntry { key: 0, value: 1 },
    ]
    .into_boxed_slice();
    let coercive = vec![
        php_jit::JitNativeDirectArrayEntry { key: 0, value: 1 },
        php_jit::JitNativeDirectArrayEntry { key: 1, value: 2 },
    ]
    .into_boxed_slice();
    let lower = vec![php_jit::JitNativeDirectArrayEntry {
        key: string_value(8),
        value: 1,
    }]
    .into_boxed_slice();
    let greater = vec![php_jit::JitNativeDirectArrayEntry {
        key: string_value(8),
        value: 2,
    }]
    .into_boxed_slice();
    let disjoint = vec![
        php_jit::JitNativeDirectArrayEntry { key: 2, value: 1 },
        php_jit::JitNativeDirectArrayEntry { key: 3, value: 2 },
    ]
    .into_boxed_slice();
    for (index, entries) in [
        ordered.as_ref(),
        same.as_ref(),
        reordered.as_ref(),
        coercive.as_ref(),
        lower.as_ref(),
        greater.as_ref(),
        disjoint.as_ref(),
    ]
    .into_iter()
    .enumerate()
    {
        slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
            flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
            payload: entries.len() as u64,
            aux: entries.as_ptr() as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
    }
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };
    let runtime = std::ptr::from_mut(&mut fast_state);
    let identical =
        super::runtime_ops::jit_native_array_identical_abi(runtime, array_value(0), array_value(1));
    assert_eq!(identical.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        identical.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE)
    );
    let reordered_identity =
        super::runtime_ops::jit_native_array_identical_abi(runtime, array_value(0), array_value(2));
    assert_eq!(reordered_identity.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        reordered_identity.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE)
    );
    for right in [2, 3] {
        let equal = super::runtime_ops::jit_native_array_equal_abi(
            runtime,
            array_value(0),
            array_value(right),
        );
        assert_eq!(equal.status, php_jit::JitCallStatus::RETURN);
        assert_eq!(
            equal.value,
            php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE)
        );
    }
    let unequal =
        super::runtime_ops::jit_native_array_equal_abi(runtime, array_value(0), array_value(6));
    assert_eq!(unequal.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        unequal.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE)
    );
    let compared =
        super::runtime_ops::jit_native_array_compare_abi(runtime, array_value(4), array_value(5));
    assert_eq!(compared.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(compared.value, -1);
}

#[test]
fn exact_native_object_comparison_uses_identity_and_authoritative_slots() {
    fn class_entry(name: &str) -> php_runtime::api::ClassEntry {
        php_runtime::api::ClassEntry {
            name: name.to_owned().into(),
            parent: None,
            interfaces: Vec::new(),
            methods: Vec::new(),
            properties: vec![php_runtime::api::ClassPropertyEntry {
                name: "value".to_owned(),
                default: php_runtime::api::Value::Null,
                type_: None,
                flags: php_runtime::api::ClassPropertyFlags::default(),
                hooks: php_runtime::api::ClassPropertyHooks::default(),
                attributes: Vec::new(),
            }],
            constants: Vec::new(),
            enum_cases: Vec::new(),
            attributes: Vec::new(),
            enum_backing_type: None,
            constructor_id: None,
            flags: php_runtime::api::ClassFlags::default(),
        }
    }
    fn object(class: &php_runtime::api::ClassEntry, value: i64) -> php_runtime::api::ObjectRef {
        php_runtime::api::ObjectRef::from_layout_native_slots(
            class,
            class.name.to_string(),
            vec![php_runtime::api::NativeDeclaredPropertySlot {
                initialized: 1,
                reserved: 0,
                value,
            }]
            .into_boxed_slice(),
        )
    }

    let class = class_entry("comparison_box");
    let other_class = class_entry("comparison_other");
    let left = Box::new(object(&class, 1));
    let same_properties = Box::new(object(&class, 1));
    let greater = Box::new(object(&class, 2));
    let other = Box::new(object(&other_class, 1));
    let dynamic = Box::new(object(&class, 1));
    dynamic.set_property("dynamic", php_runtime::api::Value::Int(9));

    let mut slots =
        vec![php_jit::JitNativeValueSlot::default(); php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY];
    let mut owners = vec![0_u64; php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY];
    let object_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_OBJECT_TAG,
        )
    };
    for (index, object) in [
        left.as_ref(),
        same_properties.as_ref(),
        greater.as_ref(),
        other.as_ref(),
        dynamic.as_ref(),
    ]
    .into_iter()
    .enumerate()
    {
        let layout_id = object.class_layout_epoch();
        let (properties, property_count) = object
            .native_declared_slots_view(layout_id)
            .expect("test object native slots");
        slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_OBJECT,
            flags: php_jit::JIT_NATIVE_OBJECT_PROPERTY_VIEW_ABI_VERSION,
            reserved: u32::try_from(property_count).expect("property count"),
            payload: layout_id,
            aux: properties as usize as u64,
        };
        owners[index] = object as *const php_runtime::api::ObjectRef as usize as u64;
    }
    let left_array = [php_jit::JitNativeDirectArrayEntry {
        key: 0,
        value: object_value(0),
    }];
    let same_array = [php_jit::JitNativeDirectArrayEntry {
        key: 0,
        value: object_value(1),
    }];
    for (index, entries) in [(5, left_array.as_slice()), (6, same_array.as_slice())] {
        slots[index] = php_jit::JitNativeValueSlot {
            refcount: 1,
            kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
            flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
            payload: entries.len() as u64,
            aux: entries.as_ptr() as usize as u64,
            ..php_jit::JitNativeValueSlot::default()
        };
    }
    slots[7] = php_jit::JitNativeValueSlot {
        refcount: 1,
        kind: php_jit::JIT_NATIVE_VALUE_VIEW_DIRECT_ARRAY,
        flags: php_jit::JIT_NATIVE_DIRECT_ARRAY_ABI_VERSION,
        ..php_jit::JitNativeValueSlot::default()
    };
    let array_value = |index: u32| {
        php_jit::jit_encode_typed_runtime_value(
            php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE + index,
            php_jit::JIT_VALUE_RUNTIME_ARRAY_TAG,
        )
    };
    let mut fast_state = super::NativeRequestFastState {
        header: php_jit::JitNativeFastStateHeader {
            abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
            flags: 0,
            runtime_view_pointer: 0,
            runtime_view: php_jit::JitNativeRuntimeView {
                abi_version: php_jit::JIT_RUNTIME_ABI_VERSION,
                direct_value_slots: slots.as_mut_ptr() as usize as u64,
                direct_object_owners: owners.as_mut_ptr() as usize as u64,
                ..php_jit::JitNativeRuntimeView::default()
            },
        },
        ..super::NativeRequestFastState::default()
    };
    let runtime = std::ptr::from_mut(&mut fast_state);

    let nested_identity =
        super::runtime_ops::jit_native_array_identical_abi(runtime, array_value(5), array_value(6));
    assert_eq!(nested_identity.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        nested_identity.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE),
        "different objects sharing one layout must not become identical"
    );

    let equal =
        super::runtime_ops::jit_native_object_equal_abi(runtime, object_value(0), object_value(1));
    assert_eq!(equal.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        equal.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE)
    );
    let unequal_class =
        super::runtime_ops::jit_native_object_equal_abi(runtime, object_value(0), object_value(3));
    assert_eq!(unequal_class.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        unequal_class.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE)
    );
    let compared = super::runtime_ops::jit_native_object_compare_abi(
        runtime,
        object_value(0),
        object_value(2),
    );
    assert_eq!(compared.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(compared.value, -1);

    let object_boolean = super::runtime_ops::jit_native_object_equal_abi(
        runtime,
        object_value(0),
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE),
    );
    assert_eq!(object_boolean.status, php_jit::JitCallStatus::RETURN);
    assert_eq!(
        object_boolean.value,
        php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE)
    );
    for (left, right) in [
        (
            array_value(5),
            php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE),
        ),
        (array_value(7), php_jit::jit_encode_constant(u32::MAX)),
        (
            array_value(7),
            php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE),
        ),
    ] {
        let equal = super::runtime_ops::jit_native_array_equal_abi(runtime, left, right);
        assert_eq!(equal.status, php_jit::JitCallStatus::RETURN);
        assert_eq!(
            equal.value,
            php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE)
        );
    }

    let cold_dynamic =
        super::runtime_ops::jit_native_object_equal_abi(runtime, object_value(0), object_value(4));
    assert_eq!(
        cold_dynamic.status,
        php_jit::JitCallStatus::RECOMPILE_REQUESTED
    );
}

#[test]
fn positional_builtin_arguments_do_not_require_rebinding() {
    use php_ir::instruction::{IrCallArg, IrCallArgValueKind};

    let argument = |name, unpack| IrCallArg {
        name,
        value: php_ir::Operand::Constant(php_ir::ConstId::new(0)),
        unpack,
        value_kind: IrCallArgValueKind::Direct,
        by_ref_local: None,
        by_ref_dim: None,
        by_ref_property: None,
        by_ref_property_dim: None,
    };
    let positional = [argument(None, false)];
    let named = [argument(Some("value".to_owned()), false)];
    let unpacked = [argument(None, true)];

    assert!(!super::call_support::native_builtin_arguments_require_binding(None));
    assert!(!super::call_support::native_builtin_arguments_require_binding(Some(&positional)));
    assert!(super::call_support::native_builtin_arguments_require_binding(Some(&named)));
    assert!(super::call_support::native_builtin_arguments_require_binding(Some(&unpacked)));
}

#[test]
fn normalized_builtin_names_borrow_the_common_lowercase_form() {
    use std::borrow::Cow;

    assert!(matches!(
        super::native_builtins::normalized_native_builtin_name("array_key_exists"),
        Cow::Borrowed("array_key_exists")
    ));
    assert!(matches!(
        super::native_builtins::normalized_native_builtin_name("\\strlen"),
        Cow::Borrowed("strlen")
    ));
    assert_eq!(
        super::native_builtins::normalized_native_builtin_name("StrLen"),
        Cow::<str>::Owned("strlen".to_owned())
    );
}

#[test]
fn plain_local_fetch_fast_path_keeps_observable_values_on_the_slow_path() {
    let null = php_jit::jit_encode_constant(u32::MAX);
    let uninitialized = php_jit::jit_encode_constant(php_jit::JIT_VALUE_UNINITIALIZED);

    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(42, false),
        Some(42)
    );
    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(null, false),
        Some(null)
    );
    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(uninitialized, false),
        None
    );
    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(uninitialized, true),
        Some(null)
    );
    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(php_jit::jit_encode_constant(3), true),
        None
    );
    assert_eq!(
        super::runtime_ops::fast_plain_local_fetch(php_jit::jit_encode_runtime_value(3), true),
        None
    );
}

#[test]
fn immediate_scalar_fast_paths_preserve_native_slot_encoding() {
    use super::runtime_ops::{
        fast_native_binary, fast_native_cast, fast_native_compare, fast_native_truthy,
        fast_native_unary,
    };

    let null = php_jit::jit_encode_constant(u32::MAX);
    let false_value = php_jit::jit_encode_constant(php_jit::JIT_VALUE_FALSE);
    let true_value = php_jit::jit_encode_constant(php_jit::JIT_VALUE_TRUE);
    let runtime = php_jit::jit_encode_runtime_value(7);

    assert_eq!(fast_native_truthy(0), Some(false));
    assert_eq!(fast_native_truthy(-7), Some(true));
    assert_eq!(fast_native_truthy(null), Some(false));
    assert_eq!(fast_native_truthy(true_value), Some(true));
    assert_eq!(fast_native_truthy(runtime), None);

    assert_eq!(fast_native_unary(1, 7), Some(-7));
    assert_eq!(fast_native_unary(1, i64::MIN), None);
    assert_eq!(fast_native_unary(2, false_value), Some(true_value));
    assert_eq!(fast_native_binary(0, 20, 22), Some(42));
    assert_eq!(fast_native_binary(0, i64::MAX, 1), None);
    assert_eq!(fast_native_binary(0, 0x7ff0_ffff_ffff_ffff, 1), None);
    assert_eq!(fast_native_unary(3, !0x7ff1_0000_0000_0000), None);
    assert_eq!(fast_native_binary(3, 8, 2), Some(4));
    assert_eq!(fast_native_binary(3, 7, 2), None);
    assert_eq!(fast_native_binary(10, 1, -1), None);

    assert_eq!(fast_native_compare(4, 2, 3), Some(true_value));
    assert_eq!(fast_native_compare(8, 3, 2), Some(1));
    assert_eq!(fast_native_compare(0, runtime, 1), None);
    assert_eq!(fast_native_cast(0, 0), Some(false_value));
    assert_eq!(fast_native_cast(1, true_value), Some(1));
    assert_eq!(fast_native_cast(6, runtime), Some(null));
}

#[test]
fn callable_resolution_dereferences_nested_php_references() {
    let inner = php_runtime::api::ReferenceCell::new(php_runtime::api::Value::String(
        php_runtime::api::PhpString::from_bytes(b"Fixture::run".to_vec()),
    ));
    let outer = php_runtime::api::ReferenceCell::new(php_runtime::api::Value::Reference(inner));
    let value = dereference_native_callable_value(php_runtime::api::Value::Reference(outer));

    assert!(matches!(
        value,
        php_runtime::api::Value::String(name) if name.as_bytes() == b"Fixture::run"
    ));
}

#[test]
fn native_php_diagnostics_match_cli_and_http_rendering() {
    let cli = format_native_php_diagnostic(
        "Deprecated",
        "Using null as an array offset is deprecated, use an empty string instead",
        "/srv/index.php",
        17,
        true,
        false,
    );
    assert_eq!(
        cli,
        "\nDeprecated: Using null as an array offset is deprecated, use an empty string instead in /srv/index.php on line 17\n"
    );

    let http = format_native_php_diagnostic(
        "Deprecated",
        "Using null as an array offset is deprecated, use an empty string instead",
        "/srv/index.php",
        17,
        true,
        true,
    );
    assert_eq!(
        http,
        "<br />\n<b>Deprecated</b>:  Using null as an array offset is deprecated, use an empty string instead in <b>/srv/index.php</b> on line <b>17</b><br />\n"
    );
}

#[test]
fn native_backtrace_lines_use_the_retained_source_index() {
    let root = std::env::temp_dir().join(format!(
        "phrust-native-backtrace-lines-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("temporary source root should be created");
    let path = root.join("fixture.php");
    std::fs::write(&path, "<?php\nline2\nfunction traced() {}\n")
        .expect("source fixture should be written");

    let span = php_ir::IrSpan::new(php_ir::FileId::new(0), 12, 32);
    let mut unit = php_ir::IrUnit::new(php_ir::UnitId::new(0));
    unit.files.push(php_ir::module::FileEntry {
        id: php_ir::FileId::new(0),
        path: path.to_string_lossy().into_owned(),
    });
    unit.functions.push(php_ir::IrFunction::new(
        "traced",
        php_ir::FunctionFlags::default(),
        span,
    ));
    let compiled = crate::compiled_unit::CompiledUnit::new(unit);

    std::fs::write(&path, "replaced without the original line structure")
        .expect("source fixture should be replaceable");
    let frame = native_backtrace_frame(
        &compiled,
        php_ir::FunctionId::new(0),
        None,
        None,
        Vec::new().into(),
    );
    let metadata = frame
        .metadata
        .expect("backtrace metadata should be prepared");
    assert_eq!(
        metadata.trace_file.as_deref(),
        Some(path.to_string_lossy().as_ref())
    );
    assert_eq!(metadata.trace_line, 3);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn direct_value_slots_keep_cold_iterator_state_out_of_line() {
    let value_bytes = std::mem::size_of::<php_runtime::api::Value>();
    let slot_bytes = std::mem::size_of::<super::NativeColdIterator>();
    assert!(
        slot_bytes <= value_bytes.saturating_add(std::mem::size_of::<usize>()),
        "native value arena slot grew to {slot_bytes} bytes for a {value_bytes}-byte PHP value"
    );
}
