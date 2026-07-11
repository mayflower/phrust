use super::prelude::*;

impl Vm {
    pub(super) fn try_execute_serialization_builtin(
        &self,
        compiled: &CompiledUnit,
        name: &str,
        values: &[Value],
        call_span: Option<php_ir::IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Option<VmResult> {
        match name {
            "serialize" => self.try_execute_serialize_with_magic(
                compiled, values, call_span, output, stack, state,
            ),
            "unserialize" => {
                self.try_execute_unserialize_with_autoload(compiled, values, output, stack, state)
            }
            _ => None,
        }
    }

    pub(super) fn try_execute_serialize_with_magic(
        &self,
        compiled: &CompiledUnit,
        values: &[Value],
        call_span: Option<php_ir::IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Option<VmResult> {
        if values.len() != 1 {
            return None;
        }
        let Value::Object(object) = effective_value(&values[0]) else {
            return None;
        };
        let result =
            self.serialize_object_with_magic(compiled, object, call_span, output, stack, state);
        Some(match result {
            Ok(value) => VmResult::success(OutputBuffer::new(), Some(Value::String(value))),
            Err(result) => {
                if let Some(throwable) = runtime_error_throwable(&result) {
                    if let Some(call_span) = call_span {
                        tag_throwable_location(&throwable, compiled, call_span);
                        reapply_throwable_diagnostic_overrides(&throwable, &result);
                        state.pending_trace = Some(attach_builtin_failed_call_trace(
                            &throwable,
                            compiled,
                            stack,
                            "serialize",
                            values,
                            call_span,
                        ));
                    } else {
                        state.pending_trace = Some(capture_backtrace_string(compiled, stack));
                    }
                    state.pending_throw = Some(throwable);
                }
                result
            }
        })
    }

    pub(super) fn try_execute_unserialize_with_autoload(
        &self,
        compiled: &CompiledUnit,
        values: &[Value],
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Option<VmResult> {
        if !(1..=2).contains(&values.len()) {
            return None;
        }
        let Value::String(input) = effective_value(&values[0]) else {
            return None;
        };
        if let Some(custom) = parse_legacy_serializable_payload(&input) {
            let result =
                self.unserialize_legacy_serializable(compiled, custom, output, stack, state);
            return Some(match result {
                Ok(value) => VmResult::success(OutputBuffer::new(), Some(value)),
                Err(result) => result,
            });
        }
        if let Some(message) = self.spl_unserialize_payload_error(compiled, state, &input) {
            return Some(self.throw_catchable_exception(compiled, output, stack, state, message));
        }
        let value = match unserialize_value(&input, UnserializeOptions::default()) {
            Ok(value) => value,
            Err(_) => return None,
        };
        let result = self.resolve_unserialized_classes(compiled, value, output, stack, state);
        Some(match result {
            Ok(value) => VmResult::success(OutputBuffer::new(), Some(value)),
            Err(result) => result,
        })
    }

    pub(super) fn resolve_unserialized_classes(
        &self,
        compiled: &CompiledUnit,
        value: Value,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        match value {
            Value::Object(object) => {
                let class_name = object.display_name();
                if class_like_exists_direct(
                    compiled,
                    state,
                    &class_name,
                    AutoloadClassLookupKind::Class,
                ) {
                    return Ok(Value::Object(object));
                }
                self.autoload_class(compiled, &class_name, output, stack, state, None)?;
                if class_like_exists_direct(
                    compiled,
                    state,
                    &class_name,
                    AutoloadClassLookupKind::Class,
                ) {
                    Ok(Value::Object(object))
                } else {
                    Ok(Value::Object(incomplete_class_object(class_name, object)))
                }
            }
            Value::Array(array) => {
                let mut resolved = PhpArray::new();
                for (key, element) in array.iter() {
                    let element = self.resolve_unserialized_classes(
                        compiled,
                        element.clone(),
                        output,
                        stack,
                        state,
                    )?;
                    resolved.insert(key.clone(), element);
                }
                Ok(Value::Array(resolved))
            }
            Value::Reference(cell) => {
                let resolved =
                    self.resolve_unserialized_classes(compiled, cell.get(), output, stack, state)?;
                cell.set(resolved);
                Ok(Value::Reference(cell))
            }
            other => Ok(other),
        }
    }

    pub(super) fn spl_unserialize_payload_error(
        &self,
        compiled: &CompiledUnit,
        state: &ExecutionState,
        input: &PhpString,
    ) -> Option<String> {
        let (class_name, payload) = parse_indexed_serialized_object_payload(input)?;
        let normalized = normalize_class_name(&class_name);
        if normalized == "hashcontext" {
            return validate_hash_context_unserialize_payload(&payload);
        }
        match normalized.as_str() {
            "arrayobject" | "arrayiterator" => validate_spl_array_container_unserialize_payload(
                compiled,
                state,
                &class_name,
                &payload,
            ),
            "spldoublylinkedlist" => validate_spl_doubly_linked_list_unserialize_payload(&payload),
            "splobjectstorage" => validate_spl_object_storage_unserialize_payload(&payload),
            _ => None,
        }
    }

    pub(super) fn vm_serialize_error_message(message: &str) -> String {
        if message == "Serialization of 'XMLParser' is not allowed" {
            format!("E_PHP_VM_EXCEPTION: {message}")
        } else if message == "HashContext with HASH_HMAC option cannot be serialized"
            || (message.starts_with("HashContext for algorithm \"")
                && message.ends_with("\" cannot be serialized"))
        {
            format!("E_PHP_VM_SPL_RUNTIME_EXCEPTION: {message}")
        } else {
            format!("E_PHP_VM_SERIALIZE_ERROR: {message}")
        }
    }

    pub(super) fn serialize_object_with_magic(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        call_span: Option<php_ir::IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        let Some(class) = lookup_class_in_state(compiled, state, &object.class_name()) else {
            return serialize_value(&Value::Object(object)).map_err(|error| {
                self.runtime_error(
                    output,
                    compiled,
                    stack,
                    Self::vm_serialize_error_message(error.message()),
                )
            });
        };
        if class_implements_in_state(
            compiled,
            state,
            &class.name,
            "Serializable",
            &mut Vec::new(),
        )
        .map_err(|message| self.runtime_error(output, compiled, stack, message))?
        {
            return self.serialize_legacy_serializable(
                compiled, object, &class, call_span, output, stack, state,
            );
        }
        let serialize_method =
            match lookup_method_in_hierarchy(compiled, &class, "__serialize", None) {
                Ok(method) => method,
                Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
            };
        if let Some(resolved) = serialize_method {
            if resolved.method.flags.is_static {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    format!(
                        "E_PHP_VM_SLEEP_METHOD_INACCESSIBLE: method {}::__serialize is not public instance",
                        resolved.class.name
                    ),
                ));
            }
            let owner = class_owner_in_state(compiled, state, &resolved.class.name);
            let result = self.execute_function(
                &owner,
                resolved.method.function,
                FunctionCall::new(Vec::new(), Vec::new())
                    .with_call_site_strict_types(owner.unit().strict_types)
                    .with_this(object.clone())
                    .with_class_context_handles(
                        self.class_name_handles(&resolved.class.name).normalized,
                        object_called_class_handle(&object),
                        self.class_name_handles(&resolved.class.name).normalized,
                    )
                    .with_optional_call_span(call_span),
                output,
                stack,
                state,
            );
            if !result.status.is_success() {
                return Err(result);
            }
            let Value::Array(properties) =
                effective_value(&result.return_value.unwrap_or(Value::Null))
            else {
                return Err(self.runtime_error(
                    output,
                    compiled,
                    stack,
                    format!(
                        "E_PHP_VM_SLEEP_RETURN_TYPE: {}::__serialize() must return an array",
                        class.display_name
                    ),
                ));
            };
            let runtime_class = runtime_class_entry(
                compiled,
                state,
                &class,
                &|value| self.constant_value(compiled.unit(), value),
                &|reference| class_constant_reference_value(compiled, state, reference),
                &|reference| named_constant_reference_value(compiled, state, reference),
            )
            .map_err(|error| self.runtime_error(output, compiled, stack, error.into_message()))?;
            let filtered = ObjectRef::new_with_display_name(&runtime_class, object.display_name());
            for (storage_name, _) in filtered.properties_snapshot() {
                filtered.unset_property(&storage_name);
            }
            for (key, value) in properties.iter() {
                let name = match key {
                    ArrayKey::String(name) => name.to_string_lossy(),
                    ArrayKey::Int(index) => index.to_string(),
                };
                filtered.set_property(name, effective_value(value));
            }
            return serialize_value(&Value::Object(filtered)).map_err(|error| {
                self.runtime_error(
                    output,
                    compiled,
                    stack,
                    Self::vm_serialize_error_message(error.message()),
                )
            });
        }
        let resolved = match lookup_method_in_hierarchy(compiled, &class, "__sleep", None) {
            Ok(Some(method)) => method,
            Ok(None) => {
                return serialize_value(&Value::Object(object)).map_err(|error| {
                    self.runtime_error(
                        output,
                        compiled,
                        stack,
                        Self::vm_serialize_error_message(error.message()),
                    )
                });
            }
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_SLEEP_METHOD_INACCESSIBLE: method {}::__sleep is not public instance",
                    resolved.class.name
                ),
            ));
        }
        let owner = class_owner_in_state(compiled, state, &resolved.class.name);
        let result = self.execute_function(
            &owner,
            resolved.method.function,
            FunctionCall::new(Vec::new(), Vec::new())
                .with_call_site_strict_types(owner.unit().strict_types)
                .with_this(object.clone())
                .with_class_context_handles(
                    self.class_name_handles(&resolved.class.name).normalized,
                    object_called_class_handle(&object),
                    self.class_name_handles(&resolved.class.name).normalized,
                )
                .with_optional_call_span(call_span),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            return Err(result);
        }
        let Value::Array(selected) = effective_value(&result.return_value.unwrap_or(Value::Null))
        else {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_SLEEP_RETURN_TYPE: {}::__sleep(): Return value must be of type array",
                    class.display_name
                ),
            ));
        };
        let runtime_class = runtime_class_entry(
            compiled,
            state,
            &class,
            &|value| self.constant_value(compiled.unit(), value),
            &|reference| class_constant_reference_value(compiled, state, reference),
            &|reference| named_constant_reference_value(compiled, state, reference),
        )
        .map_err(|error| self.runtime_error(output, compiled, stack, error.into_message()))?;
        let filtered = ObjectRef::new_with_display_name(&runtime_class, object.display_name());
        for (storage_name, _) in filtered.properties_snapshot() {
            filtered.unset_property(&storage_name);
        }
        let source_properties = object.properties_snapshot();
        for (_, selected_name) in selected.iter() {
            let Value::String(selected_name) = effective_value(selected_name) else {
                continue;
            };
            let selected_name = selected_name.to_string_lossy();
            let Some((storage_name, value)) =
                sleep_property_value(&source_properties, &selected_name)
            else {
                self.emit_serialize_sleep_missing_property_warning(
                    compiled,
                    output,
                    stack,
                    state,
                    &selected_name,
                    call_span,
                )?;
                continue;
            };
            filtered.set_property(storage_name, effective_value(&value));
        }
        serialize_value(&Value::Object(filtered)).map_err(|error| {
            self.runtime_error(
                output,
                compiled,
                stack,
                Self::vm_serialize_error_message(error.message()),
            )
        })
    }

    pub(super) fn call_spl_container_method_with_magic(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        method: &str,
        args: Vec<CallArgument>,
        call_span: Option<IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        let normalized_method = normalize_method_name(method);
        if normalized_method == "serialize" {
            let runtime_class = spl_runtime_marker(&object)
                .unwrap_or_else(|| normalize_class_name(&object.class_name()));
            if matches!(
                runtime_class.as_str(),
                "spldoublylinkedlist" | "splstack" | "splqueue" | "splobjectstorage"
            ) {
                validate_spl_iterator_arg_count(&object.class_name(), &args, 0, 0)
                    .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
                let bytes = self.serialize_spl_container_legacy(
                    compiled,
                    &object,
                    &runtime_class,
                    call_span,
                    output,
                    stack,
                    state,
                )?;
                return Ok(Value::String(bytes));
            }
        }
        let replaced_storage_info = if spl_runtime_marker(&object).as_deref()
            == Some("splobjectstorage")
            && normalized_method == "setinfo"
        {
            let pos = spl_position(&object);
            spl_storage_entries(&object)
                .get(pos)
                .map(|(_, _, info)| info.clone())
        } else {
            None
        };
        let value = call_spl_container_method(object, method, args)
            .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
        if let Some(info) = replaced_storage_info {
            let candidates = destructor_candidates_for_value(&info);
            if !candidates.is_empty() {
                let rooted_object_ids = php_visible_non_register_root_object_ids(stack, state);
                let mut handlers = Vec::new();
                let mut pending_control = None;
                let sweep = self.run_destructors_for_unreferenced_candidates_with_roots(
                    compiled,
                    output,
                    stack,
                    state,
                    &mut handlers,
                    &mut pending_control,
                    candidates,
                    &rooted_object_ids,
                    None,
                );
                if let Some(outcome) = sweep.outcome {
                    match outcome {
                        RaiseOutcome::Caught(_) => {}
                        RaiseOutcome::Done(result) => return Err(*result),
                    }
                }
            }
        }
        Ok(value)
    }

    pub(super) fn serialize_spl_container_legacy(
        &self,
        compiled: &CompiledUnit,
        object: &ObjectRef,
        runtime_class: &str,
        call_span: Option<IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        let mut bytes = Vec::new();
        if normalize_class_name(runtime_class) == "splobjectstorage" {
            let records = spl_storage_entries(object);
            bytes.extend_from_slice(format!("x:i:{};", records.len()).as_bytes());
            for (_, item, info) in records {
                let item = self.serialize_value_with_magic(
                    compiled,
                    Value::Object(item),
                    call_span,
                    output,
                    stack,
                    state,
                )?;
                bytes.extend_from_slice(item.as_bytes());
                bytes.push(b',');
                let info = self
                    .serialize_value_with_magic(compiled, info, call_span, output, stack, state)?;
                bytes.extend_from_slice(info.as_bytes());
                bytes.push(b';');
            }
            bytes.extend_from_slice(b"m:");
            let properties = self.serialize_value_with_magic(
                compiled,
                Value::Array(spl_object_user_properties_array(object)),
                call_span,
                output,
                stack,
                state,
            )?;
            bytes.extend_from_slice(properties.as_bytes());
            return Ok(PhpString::from(bytes));
        }

        let mut index = 0usize;
        loop {
            let entries = spl_entries(object);
            let Some((key, value)) = entries.get(index).cloned() else {
                break;
            };
            let key = self.serialize_value_with_magic(
                compiled,
                array_key_to_value(key),
                call_span,
                output,
                stack,
                state,
            )?;
            bytes.extend_from_slice(key.as_bytes());
            bytes.push(b':');
            let value =
                self.serialize_value_with_magic(compiled, value, call_span, output, stack, state)?;
            bytes.extend_from_slice(value.as_bytes());
            index += 1;
        }
        Ok(PhpString::from(bytes))
    }

    pub(super) fn serialize_value_with_magic(
        &self,
        compiled: &CompiledUnit,
        value: Value,
        call_span: Option<IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        match effective_value(&value) {
            Value::Object(object) => {
                self.serialize_object_with_magic(compiled, object, call_span, output, stack, state)
            }
            other => serialize_value(&other).map_err(|error| {
                self.runtime_error(
                    output,
                    compiled,
                    stack,
                    Self::vm_serialize_error_message(error.message()),
                )
            }),
        }
    }

    pub(super) fn call_hash_context_method(
        &self,
        object: &ObjectRef,
        method: &str,
        args: &[CallArgument],
    ) -> Result<Value, String> {
        match normalize_method_name(method).as_str() {
            "__debuginfo" => {
                if !args.is_empty() {
                    return Err(format!(
                        "E_PHP_VM_TOO_MANY_ARGS: HashContext::__debugInfo() expects exactly 0 arguments, {} given",
                        args.len()
                    ));
                }
                let Some(properties) = hash_context_debug_info_array(object) else {
                    return Err(
                        "E_PHP_VM_INVALID_HASH_CONTEXT: invalid HashContext state".to_owned()
                    );
                };
                Ok(Value::Array(properties))
            }
            "__serialize" => {
                validate_hash_context_arg_count("__serialize", args, 0)?;
                hash_context_serialize_array(object).map(Value::Array)
            }
            "__unserialize" => {
                validate_hash_context_arg_count("__unserialize", args, 1)?;
                if hash_context_object_is_initialized(object) {
                    return Err(hash_context_runtime_exception(
                        "HashContext::__unserialize called on initialized object",
                    ));
                }
                let Value::Array(payload) = effective_value(&args[0].value) else {
                    return Err(format!(
                        "E_PHP_VM_TYPE_ERROR: HashContext::__unserialize(): Argument #1 ($data) must be of type array, {} given",
                        value_type_name(&args[0].value)
                    ));
                };
                if let Some(message) = validate_hash_context_unserialize_payload(&payload) {
                    return Err(hash_context_runtime_exception(message));
                }
                Err(hash_context_runtime_exception(
                    "Incomplete or ill-formed serialization data",
                ))
            }
            _ => Err(format!(
                "E_PHP_VM_METHOD_NOT_FOUND: Call to undefined method HashContext::{method}()"
            )),
        }
    }

    pub(super) fn serialize_legacy_serializable(
        &self,
        compiled: &CompiledUnit,
        object: ObjectRef,
        class: &php_ir::module::ClassEntry,
        call_span: Option<php_ir::IrSpan>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<PhpString, VmResult> {
        let resolved = match lookup_method_in_hierarchy(compiled, class, "serialize", None) {
            Ok(Some(method)) => method,
            Ok(None) => {
                return serialize_value(&Value::Object(object)).map_err(|error| {
                    self.runtime_error(
                        output,
                        compiled,
                        stack,
                        Self::vm_serialize_error_message(error.message()),
                    )
                });
            }
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_SERIALIZABLE_METHOD_INACCESSIBLE: method {}::serialize is not public instance",
                    resolved.class.name
                ),
            ));
        }
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(Vec::new(), Vec::new())
                .with_this(object)
                .with_class_context(
                    resolved.class.name.clone(),
                    class.name.clone(),
                    resolved.class.name.clone(),
                )
                .with_optional_call_span(call_span),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            return Err(result);
        }
        match effective_value(&result.return_value.unwrap_or(Value::Null)) {
            Value::String(payload) => Ok(legacy_serializable_wire(&class.display_name, &payload)),
            Value::Null => Ok(PhpString::from_test_str("N;")),
            _ => Err(self.throw_exception_result(
                compiled,
                output,
                stack,
                state,
                call_span.unwrap_or_default(),
                format!(
                    "{}::serialize() must return a string or NULL",
                    class.display_name
                ),
            )),
        }
    }

    pub(super) fn unserialize_legacy_serializable(
        &self,
        compiled: &CompiledUnit,
        payload: LegacySerializablePayload,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<Value, VmResult> {
        self.autoload_class(compiled, &payload.class_name, output, stack, state, None)?;
        let Some(class) = lookup_class_in_state(compiled, state, &payload.class_name) else {
            let source = ObjectRef::new_with_display_name(
                &empty_runtime_class(&payload.class_name),
                payload.class_name.clone(),
            );
            return Ok(Value::Object(incomplete_class_object(
                source.display_name(),
                source,
            )));
        };
        let runtime_class = runtime_class_entry(
            compiled,
            state,
            &class,
            &|value| self.constant_value(compiled.unit(), value),
            &|reference| class_constant_reference_value(compiled, state, reference),
            &|reference| named_constant_reference_value(compiled, state, reference),
        )
        .map_err(|error| self.runtime_error(output, compiled, stack, error.into_message()))?;
        let object = ObjectRef::new_with_display_name(&runtime_class, class.display_name.clone());
        let resolved = match lookup_method_in_hierarchy(compiled, &class, "unserialize", None) {
            Ok(Some(method)) => method,
            Ok(None) => return Ok(Value::Object(object)),
            Err(message) => return Err(self.runtime_error(output, compiled, stack, message)),
        };
        if resolved.method.flags.is_static {
            return Err(self.runtime_error(
                output,
                compiled,
                stack,
                format!(
                    "E_PHP_VM_SERIALIZABLE_METHOD_INACCESSIBLE: method {}::unserialize is not public instance",
                    resolved.class.name
                ),
            ));
        }
        let result = self.execute_function(
            compiled,
            resolved.method.function,
            FunctionCall::new(
                vec![CallArgument::positional(Value::String(payload.payload))],
                Vec::new(),
            )
            .with_this(object.clone())
            .with_class_context(
                resolved.class.name.clone(),
                class.name.clone(),
                resolved.class.name.clone(),
            ),
            output,
            stack,
            state,
        );
        if !result.status.is_success() {
            return Err(result);
        }
        Ok(Value::Object(object))
    }

    pub(super) fn emit_serialize_sleep_missing_property_warning(
        &self,
        compiled: &CompiledUnit,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
        property: &str,
        call_span: Option<php_ir::IrSpan>,
    ) -> Result<(), VmResult> {
        let diagnostic = RuntimeDiagnostic::new(
            "E_PHP_VM_SERIALIZE_SLEEP_MISSING_PROPERTY",
            RuntimeSeverity::Warning,
            format!(
                "serialize(): \"{property}\" returned as member variable from __sleep() but does not exist"
            ),
            call_span
                .map(|span| runtime_source_span(compiled, span))
                .unwrap_or_default(),
            stack_trace(compiled, stack),
            Some(php_runtime::PhpReferenceClassification::Warning),
        );
        let handled = self.dispatch_error_handler(
            compiled,
            output,
            stack,
            state,
            php_runtime::PHP_E_WARNING,
            &diagnostic,
        )?;
        if !handled && error_reporting_allows(state, php_runtime::PHP_E_WARNING) {
            Self::record_last_error(state, php_runtime::PHP_E_WARNING, &diagnostic);
            emit_vm_diagnostic(
                output,
                state,
                &diagnostic,
                php_runtime::PhpDiagnosticChannel::Warning,
                php_runtime::PHP_E_WARNING,
            );
            state.diagnostics.push(diagnostic);
        }
        Ok(())
    }
}
