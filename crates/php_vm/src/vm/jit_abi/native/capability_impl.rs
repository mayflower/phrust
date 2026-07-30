//! Native-only capability behavior. Cold pointer publication lives in cold_publication.

use super::*;
use php_ir::module::{normalize_class_name, normalized_class_name};

fn native_exact_function_requires_non_reference_trampoline(
    function: &php_ir::IrFunction,
    method_scope_sensitive: bool,
) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                php_ir::InstructionKind::Yield { .. } | php_ir::InstructionKind::YieldFrom { .. }
            ) || matches!(
                &instruction.kind,
                php_ir::InstructionKind::CallFunction { name, .. }
                    if name.trim_start_matches('\\').eq_ignore_ascii_case("debug_backtrace")
            ) || method_scope_sensitive
                && matches!(
                    &instruction.kind,
                    php_ir::InstructionKind::FetchClassConstant {
                        class_name,
                        ..
                    } | php_ir::InstructionKind::CallStaticMethod {
                        class_name,
                        ..
                    } if class_name.eq_ignore_ascii_case("static")
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

fn native_exact_php_function_exists(name: &str) -> bool {
    if matches!(
        name,
        "print"
            | "mhash"
            | "mhash_count"
            | "mhash_get_block_size"
            | "mhash_get_hash_name"
            | "mhash_keygen_s2k"
    ) {
        return false;
    }
    php_std::introspection::function_exists(php_std::ExtensionRegistry::standard_library(), name)
        || php_extensions::BuiltinRegistry::new().contains(name)
}

fn native_exact_internal_class_constant_exists(name: &str) -> bool {
    let Some((class_name, constant_name)) = name.rsplit_once("::") else {
        return false;
    };
    php_std::ExtensionRegistry::standard_library()
        .enabled_class(class_name)
        .is_some()
        && php_std::generated::arginfo::constant_metadata_in_hierarchy(class_name, constant_name)
            .is_some()
}

impl NativeSymbolQueryCapability {
    #[allow(unsafe_code)]
    pub(crate) fn active_compiled(&self) -> Option<&crate::compiled_unit::CompiledUnit> {
        unsafe { self.active_compiled.as_ref() }
    }

    #[allow(unsafe_code)]
    pub(crate) fn current_dynamic_unit(&self) -> Option<usize> {
        unsafe { self.current_dynamic_unit.as_ref() }
            .copied()
            .flatten()
    }

    #[allow(unsafe_code)]
    pub(crate) fn dynamic_units(&self) -> Option<&[NativeDynamicUnit]> {
        unsafe { self.dynamic_units.as_ref() }.map(Vec::as_slice)
    }

    #[allow(unsafe_code)]
    pub(crate) fn class_is_visible(&self, normalized: &str) -> bool {
        unsafe { self.deployment_classes.as_ref() }
            .is_some_and(|classes| classes.as_ref().contains(normalized))
            || unsafe { self.dynamic_classes.as_ref() }
                .is_some_and(|classes| classes.contains(normalized))
    }

    #[allow(unsafe_code)]
    pub(crate) fn external_class_handle(
        &self,
        name: &str,
    ) -> Option<crate::compiled_unit::CompiledClass> {
        let requested = normalized_class_name(name);
        let normalized = unsafe { self.class_aliases.as_ref() }
            .and_then(|aliases| aliases.get(requested.as_ref()))
            .map_or(requested.as_ref(), String::as_str);
        let unit = unsafe { self.external_class_units.as_ref() }
            .and_then(|classes| classes.get(normalized).copied())
            .or_else(|| {
                unsafe { self.deployment_classes.as_ref() }
                    .is_some_and(|classes| classes.as_ref().contains(normalized))
                    .then_some(0)
            })?;
        if self.current_dynamic_unit() == Some(unit) {
            return None;
        }
        self.dynamic_units()?
            .get(unit)?
            .compiled
            .lookup_unit_class_handle(normalized)
    }

    pub(crate) fn class_handle(&self, name: &str) -> Option<crate::compiled_unit::CompiledClass> {
        let normalized = normalize_class_name(name);
        self.active_compiled()?
            .lookup_unit_class_handle(&normalized)
            .or_else(|| self.external_class_handle(&normalized))
    }

    pub(crate) fn caller_class(&self, function: u32) -> Option<String> {
        self.active_compiled()?
            .unit()
            .classes
            .iter()
            .find(|class| {
                class
                    .methods
                    .iter()
                    .any(|method| method.function.raw() == function)
            })
            .map(|class| class.name.clone())
    }

    pub(crate) fn class_lineage_any(
        &self,
        name: &str,
        predicate: &mut impl FnMut(&crate::compiled_unit::CompiledClass) -> bool,
    ) -> bool {
        fn visit(
            symbols: &NativeSymbolQueryCapability,
            name: &str,
            depth: usize,
            predicate: &mut impl FnMut(&crate::compiled_unit::CompiledClass) -> bool,
        ) -> bool {
            if depth >= php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
                return false;
            }
            let Some(class) = symbols.class_handle(name) else {
                return false;
            };
            if predicate(&class) {
                return true;
            }
            class
                .parent
                .as_deref()
                .is_some_and(|parent| visit(symbols, parent, depth + 1, predicate))
        }
        visit(self, name, 0, predicate)
    }

    /// Resolves an exact class/interface ancestry query from the published
    /// unit, deployment, and internal-class metadata. `None` means some
    /// ancestry node is not represented by this capability and must take the
    /// instruction's single baseline continuation.
    #[allow(unsafe_code)]
    pub(crate) fn class_is_a(&self, class_name: &str, target: &str) -> Option<bool> {
        fn visit(
            symbols: &NativeSymbolQueryCapability,
            candidate: &str,
            target: &str,
            depth: usize,
        ) -> Option<bool> {
            if depth >= php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
                return None;
            }
            let candidate = normalize_class_name(candidate);
            if candidate == target {
                return Some(true);
            }
            if candidate == "arrayiterator" && matches!(target, "iterator" | "traversable") {
                return Some(true);
            }
            if let Some(class) = symbols.class_handle(&candidate) {
                if let Some(parent) = class.parent.as_deref()
                    && visit(symbols, parent, target, depth + 1)?
                {
                    return Some(true);
                }
                for interface in &class.interfaces {
                    if visit(symbols, interface, target, depth + 1)? {
                        return Some(true);
                    }
                }
                return Some(false);
            }
            if let Some(class) =
                php_std::ExtensionRegistry::standard_library().enabled_class(&candidate)
                && let Some(metadata) = class.source_metadata()
            {
                if let Some(parent) = metadata.parent
                    && visit(symbols, parent, target, depth + 1)?
                {
                    return Some(true);
                }
                for interface in metadata.interfaces {
                    if visit(symbols, interface, target, depth + 1)? {
                        return Some(true);
                    }
                }
                return Some(false);
            }
            None
        }

        let target = normalize_class_name(target);
        let target = unsafe { self.class_aliases.as_ref() }
            .and_then(|aliases| aliases.get(&target))
            .map_or(target.as_str(), String::as_str)
            .to_owned();
        visit(self, class_name, &target, 0)
    }

    #[allow(unsafe_code)]
    pub(crate) fn constant_exists(&self, name: &str) -> bool {
        unsafe { self.native_dynamic_constants.as_ref() }
            .is_some_and(|values| values.contains_key(name))
            || self.active_compiled().is_some_and(|compiled| {
                compiled
                    .unit()
                    .constant_table
                    .iter()
                    .any(|constant| constant.name == name)
            })
            || native_exact_internal_class_constant_exists(name)
            || php_std::ExtensionRegistry::standard_library()
                .enabled_constant(name)
                .and_then(php_std::ConstantDescriptor::value)
                .is_some()
    }

    #[allow(unsafe_code)]
    pub(crate) fn native_constants(&self) -> Option<&std::collections::BTreeMap<String, i64>> {
        unsafe { self.native_dynamic_constants.as_ref() }
    }

    #[allow(unsafe_code)]
    pub(crate) fn dynamic_constant_sites(&self, name: &str) -> (*const usize, usize) {
        let sites: &[usize] = unsafe { self.trusted_dynamic_constant_sites.as_ref() }
            .and_then(|sites| sites.get(name))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        (sites.as_ptr(), sites.len())
    }

    #[allow(unsafe_code)]
    pub(crate) fn function_exists(&self, name: &str) -> bool {
        let normalized = name.to_ascii_lowercase();
        let active = self.active_compiled().is_some_and(|compiled| {
            compiled
                .unit()
                .function_table
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(name))
        });
        let dynamic = unsafe { self.dynamic_functions.as_ref() }.is_some_and(|functions| {
            functions.contains_key(name) || functions.contains_key(&normalized)
        });
        let external = unsafe { self.external_functions.as_ref() }.is_some_and(|functions| {
            functions.contains_key(name) || functions.contains_key(&normalized)
        });
        let deployment = unsafe { self.deployment_functions.as_ref() }
            .is_some_and(|functions| functions.as_ref().contains_key(normalized.as_str()));
        let visible = unsafe { self.visible_function_names.as_ref() }
            .is_some_and(|functions| functions.contains(&normalized));
        active
            || dynamic
            || external
            || deployment
            || visible
            || native_exact_php_function_exists(&normalized)
    }

    pub(crate) fn same_unit_callable_plan(&self, name: &str) -> Option<NativeFixedCallablePlan> {
        let compiled = self.active_compiled()?;
        let normalized = name.trim_start_matches('\\').to_ascii_lowercase();
        let function = compiled.lookup_function(&normalized).or_else(|| {
            normalized
                .rsplit_once('\\')
                .and_then(|(_, basename)| compiled.lookup_function(basename))
        })?;
        native_fixed_callable_plan(compiled, function, false)
    }

    /// Resolve one public method against the immutable same-unit hierarchy.
    ///
    /// Callable publication is the semantic boundary: the exact method
    /// identity, staticness and fixed by-value signature are recorded once.
    /// Dynamic classes, inaccessible methods, magic dispatch, and
    /// late-static-scope-sensitive bodies remain on the single baseline
    /// continuation.
    pub(crate) fn same_unit_method_callable_plan(
        &self,
        class_name: &str,
        method_name: &str,
        object_target: bool,
    ) -> Option<NativeFixedCallablePlan> {
        let compiled = self.active_compiled()?;
        let mut candidate = normalize_class_name(class_name);
        loop {
            let class = compiled
                .unit()
                .classes
                .iter()
                .find(|class| class.name == candidate)?;
            if let Some(method) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(method_name))
            {
                if method.flags.is_abstract
                    || method.flags.is_private
                    || method.flags.is_protected
                    || (!object_target && !method.flags.is_static)
                {
                    return None;
                }
                let function = compiled.unit().functions.get(method.function.index())?;
                if native_exact_function_requires_non_reference_trampoline(function, true) {
                    return None;
                }
                let has_receiver = !method.flags.is_static;
                let plan = native_fixed_callable_plan(compiled, method.function, has_receiver)?;
                if usize::from(has_receiver).saturating_add(plan.visible_arity as usize)
                    > u8::MAX as usize
                {
                    return None;
                }
                return Some(plan);
            }
            candidate = normalize_class_name(class.parent.as_ref()?);
        }
    }

    /// Decides callable visibility from published immutable class metadata.
    ///
    /// Public concrete methods and public magic dispatch are representation
    /// complete here. Visibility-sensitive, abstract, or unpublished class
    /// shapes return `None` so the callsite takes its single baseline
    /// continuation before producing an observable result.
    pub(crate) fn method_is_callable(
        &self,
        class_name: &str,
        method_name: &str,
        object_target: bool,
    ) -> Option<bool> {
        let mut candidate = normalize_class_name(class_name);
        for _ in 0..php_jit::JIT_NATIVE_DIRECT_VALUE_CAPACITY {
            let class = self.class_handle(&candidate)?;
            if let Some(method) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(method_name))
            {
                if method.flags.is_abstract || method.flags.is_private || method.flags.is_protected
                {
                    return None;
                }
                if !object_target && !method.flags.is_static {
                    return None;
                }
                return Some(true);
            }
            let magic_name = if object_target {
                "__call"
            } else {
                "__callStatic"
            };
            if let Some(magic) = class
                .methods
                .iter()
                .find(|method| method.name.eq_ignore_ascii_case(magic_name))
            {
                if magic.flags.is_abstract
                    || magic.flags.is_private
                    || magic.flags.is_protected
                    || (!object_target && !magic.flags.is_static)
                {
                    return None;
                }
                return Some(true);
            }
            let Some(parent) = class.parent.as_deref() else {
                return Some(false);
            };
            candidate = normalize_class_name(parent);
        }
        None
    }
}

pub(crate) fn native_fixed_callable_plan(
    compiled: &crate::compiled_unit::CompiledUnit,
    function_id: php_ir::FunctionId,
    has_receiver: bool,
) -> Option<NativeFixedCallablePlan> {
    let function = compiled.unit().functions.get(function_id.index())?;
    let requires_argument_trace = function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                &instruction.kind,
                php_ir::InstructionKind::CallFunction { name, .. }
                    if matches!(
                        name.trim_start_matches('\\').to_ascii_lowercase().as_str(),
                        "func_get_arg" | "func_get_args" | "func_num_args"
                    )
            )
        })
    });
    let first_parameter_by_reference = function
        .params
        .first()
        .is_some_and(|parameter| parameter.by_ref);
    let supported_parameters = function
        .params
        .iter()
        .enumerate()
        .all(|(index, parameter)| !parameter.variadic && (!parameter.by_ref || index == 0));
    let admitted = !function.flags.is_generator
        && !function.returns_by_ref
        && !requires_argument_trace
        && function.params.len() <= u8::MAX as usize
        && supported_parameters;
    let visible_arity = u32::try_from(function.params.len()).ok()?;
    admitted.then(|| NativeFixedCallablePlan {
        function: function_id,
        visible_arity,
        has_receiver,
        first_parameter_by_reference,
        returns_int: matches!(
            function.return_type.as_ref(),
            Some(php_ir::IrReturnType::Int)
        ),
        returns_string: matches!(
            function.return_type.as_ref(),
            Some(php_ir::IrReturnType::String)
        ),
        returns_releasable_scalar: function
            .return_type
            .as_ref()
            .is_some_and(native_callback_return_type_is_releasable_scalar),
    })
}

fn native_callback_return_type_is_releasable_scalar(type_: &php_ir::IrReturnType) -> bool {
    use php_ir::IrReturnType as Type;
    match type_ {
        Type::Int
        | Type::Float
        | Type::String
        | Type::Bool
        | Type::Null
        | Type::False
        | Type::True
        | Type::Void
        | Type::Never => true,
        Type::Nullable { inner } => native_callback_return_type_is_releasable_scalar(inner),
        Type::Union { members } => {
            !members.is_empty()
                && members
                    .iter()
                    .all(native_callback_return_type_is_releasable_scalar)
        }
        Type::Array
        | Type::Callable
        | Type::Iterable
        | Type::Object
        | Type::Mixed
        | Type::Class { .. }
        | Type::Intersection { .. }
        | Type::Dnf { .. } => false,
    }
}

impl NativeRequestQueryCapability {
    #[allow(unsafe_code)]
    pub(crate) fn environment(&self) -> Option<&[(String, String)]> {
        unsafe { self.environment.as_ref() }.map(|environment| environment.as_ref().as_slice())
    }

    #[allow(unsafe_code)]
    pub(crate) fn included_files(&self) -> Option<&std::collections::BTreeSet<std::path::PathBuf>> {
        unsafe { self.included_files.as_ref() }
    }

    #[allow(unsafe_code)]
    pub(crate) fn sapi_name(&self) -> Option<&str> {
        unsafe { self.sapi_name.as_ref() }.map(String::as_str)
    }
}

impl NativeConfigurationCapability {
    /// Returns the request registry guaranteed by capability publication.
    ///
    /// Exact handlers never validate this engine invariant per invocation:
    /// `NativeRequestOwner` publishes the stable non-null owner before native
    /// execution can observe the fast state.
    #[allow(unsafe_code)]
    pub(crate) fn ini_registry(&self) -> &php_runtime::api::IniRegistry {
        unsafe { &*self.ini_registry }
    }

    #[allow(unsafe_code)]
    pub(crate) fn ini_registry_mut(&mut self) -> &mut php_runtime::api::IniRegistry {
        unsafe { &mut *self.ini_registry }
    }

    #[allow(unsafe_code)]
    pub(crate) fn include_path_mut(&mut self) -> &mut Arc<Vec<std::path::PathBuf>> {
        unsafe { &mut *self.include_path }
    }

    #[allow(unsafe_code)]
    pub(crate) fn include_path(&self) -> &Arc<Vec<std::path::PathBuf>> {
        unsafe { &*self.include_path }
    }

    #[allow(unsafe_code)]
    pub(crate) fn display_errors_mut(&mut self) -> &mut bool {
        unsafe { &mut *self.display_errors }
    }

    #[allow(unsafe_code)]
    pub(crate) fn default_timezone(&self) -> &str {
        unsafe { &*self.default_timezone }.as_str()
    }

    #[allow(unsafe_code)]
    pub(crate) fn default_timezone_mut(&mut self) -> &mut String {
        unsafe { &mut *self.default_timezone }
    }
}

impl NativeHttpResponseCapability {
    /// Publication guarantees the stable non-null owner; exact invocation
    /// therefore performs no repeated engine-integrity validation.
    #[allow(unsafe_code)]
    pub(crate) fn response(&self) -> &php_runtime::api::RuntimeHttpResponseState {
        unsafe { &*self.response }
    }

    #[allow(unsafe_code)]
    pub(crate) fn response_mut(&mut self) -> &mut php_runtime::api::RuntimeHttpResponseState {
        unsafe { &mut *self.response }
    }
}

impl NativeSessionCapability {
    #[allow(unsafe_code)]
    pub(crate) fn control(&self) -> &php_runtime::api::NativeSessionControlState {
        unsafe { &*self.control }
    }

    #[allow(unsafe_code)]
    pub(crate) fn control_mut(&mut self) -> &mut php_runtime::api::NativeSessionControlState {
        unsafe { &mut *self.control }
    }

    pub(crate) const fn has_loader(&self) -> bool {
        self.has_loader != 0
    }

    pub(crate) const fn has_id_generator(&self) -> bool {
        self.has_id_generator != 0
    }
}

impl NativeExecutionDeadlineCapability {
    /// Checks and publishes only the deadline diagnostic owned by this
    /// capability. No value plane, call frame, unit, or compatibility state
    /// is reachable from the exact poll.
    #[allow(unsafe_code)]
    pub(crate) fn poll(&mut self) -> i32 {
        let Some(deadline) = (unsafe { self.deadline.as_ref() }) else {
            return php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32;
        };
        if deadline.is_none_or(|deadline| std::time::Instant::now() < deadline) {
            return 0;
        }
        let Some(diagnostic) = (unsafe { self.diagnostic.as_mut() }) else {
            return php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32;
        };
        *diagnostic = Some(php_runtime::api::RuntimeDiagnostic::new(
            "E_PHP_VM_EXECUTION_TIMEOUT",
            php_runtime::api::RuntimeSeverity::RecoverableError,
            "maximum execution time exceeded",
            php_runtime::api::RuntimeSourceSpan::default(),
            Vec::new(),
            None,
        ));
        php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32
    }
}

impl NativeFrameArenaCapability {
    /// Allocates one generated frame from the authoritative native arena.
    ///
    /// Publication guarantees both pointers are valid for the synchronous
    /// request lifetime, so the compiled boundary performs no cold-context
    /// recovery or repeated engine-integrity validation.
    #[allow(unsafe_code)]
    pub(crate) fn allocate(&mut self, bytes: u64, alignment: u64) -> u64 {
        let result = usize::try_from(bytes)
            .map_err(|_| "E_PHP_VM_NATIVE_FRAME_LIMIT: frame size does not fit usize".to_owned())
            .and_then(|bytes| {
                usize::try_from(alignment)
                    .map_err(|_| {
                        "E_PHP_VM_NATIVE_FRAME_ALIGNMENT: alignment does not fit usize".to_owned()
                    })
                    .and_then(|alignment| unsafe { &mut *self.arena }.allocate(bytes, alignment))
            });
        match result {
            Ok(address) => address as u64,
            Err(message) => {
                unsafe {
                    *self.diagnostic = Some(php_runtime::api::RuntimeDiagnostic::new(
                        "E_PHP_VM_NATIVE_FRAME_LIMIT",
                        php_runtime::api::RuntimeSeverity::FatalError,
                        message,
                        php_runtime::api::RuntimeSourceSpan::default(),
                        Vec::new(),
                        None,
                    ));
                }
                0
            }
        }
    }

    #[allow(unsafe_code)]
    pub(crate) fn release(&mut self, address: u64) -> i32 {
        if unsafe { &mut *self.arena }
            .release(address as usize)
            .is_ok()
        {
            0
        } else {
            php_jit::JitCallStatus::RUNTIME_ERROR.0 as i32
        }
    }
}
