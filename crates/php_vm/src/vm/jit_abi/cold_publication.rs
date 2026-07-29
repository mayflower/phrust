//! Cold request/publication-time native metadata construction.
//!
//! Class layouts and immutable constant relocations are resolved once
//! before generated code executes. Per-invocation validation is forbidden.

use super::*;
use php_runtime::api::PhpString;
use php_runtime::api::Value;

impl<'a> NativeRequestColdState<'a> {
    /// Resolve immutable local class layouts once for the active source unit.
    /// Plans with request-dependent defaults or unresolved external parents
    /// remain empty and retain their single baseline continuation.
    pub(super) fn prepare_trusted_class_plans(&mut self) {
        if self.trusted_class_plans.len() == self.unit.classes.len()
            && self.unit.classes.iter().enumerate().all(|(index, class)| {
                !native_class_is_publication_allocatable(self, self.current_dynamic_unit, class)
                    || self.trusted_class_plans[index].state
                        == php_jit::JIT_NATIVE_PREPARED_CLASS_ALLOCATABLE
            })
        {
            return;
        }
        let owner = self.current_dynamic_unit;
        let classes = self.unit.classes.clone();
        if self.trusted_class_plans.len() != classes.len() {
            self.trusted_class_plans.resize(
                classes.len(),
                php_jit::JitNativePreparedClassPlan::default(),
            );
        }
        for (index, class) in classes.iter().enumerate() {
            if self.trusted_class_plans[index].state
                == php_jit::JIT_NATIVE_PREPARED_CLASS_ALLOCATABLE
            {
                continue;
            }
            if !native_class_is_publication_allocatable(self, owner, class) {
                continue;
            }
            let key = (owner, class.name.clone());
            let cached = { self.runtime_class_cache.borrow().get(&key).cloned() };
            let prepared = if let Some(cached) = cached {
                Some(cached)
            } else {
                let Ok(entry) = native_runtime_class_with_owner(self, owner, class) else {
                    continue;
                };
                let default_declared_slots = php_runtime::api::ObjectRef::default_declared_slots(
                    &entry,
                    &class.display_name,
                );
                let mut owned_defaults = Vec::new();
                let mut default_native_slots = Vec::with_capacity(default_declared_slots.len());
                let mut failed = false;
                for default in default_declared_slots {
                    let encoded = match default {
                        None => {
                            default_native_slots
                                .push(php_runtime::api::NativeDeclaredPropertySlot::default());
                            continue;
                        }
                        Some(Value::Uninitialized) => {
                            php_jit::jit_encode_constant(php_jit::JIT_VALUE_UNINITIALIZED)
                        }
                        Some(value) => match self.encode_baseline_value(value) {
                            Ok(encoded) => encoded,
                            Err(_) => {
                                failed = true;
                                break;
                            }
                        },
                    };
                    if let Some(runtime_index) = php_jit::jit_decode_runtime_value(encoded) {
                        if runtime_index < php_jit::JIT_NATIVE_DIRECT_VALUE_INDEX_BASE {
                            let _ = self.release(encoded);
                            failed = true;
                            break;
                        }
                        owned_defaults.push(encoded);
                    }
                    default_native_slots.push(php_runtime::api::NativeDeclaredPropertySlot {
                        initialized: 1,
                        reserved: 0,
                        value: encoded,
                    });
                }
                if failed {
                    for encoded in owned_defaults {
                        let _ = self.release(encoded);
                    }
                    continue;
                }
                let layout_id =
                    php_runtime::api::ObjectRef::prepared_layout_id(&entry, &class.display_name);
                self.runtime_class_layout_cache
                    .borrow_mut()
                    .insert(key.clone(), layout_id);
                let prepared = Rc::new(PreparedNativeRuntimeClass {
                    entry,
                    display_name: class.display_name.clone(),
                    layout_id,
                    default_native_slots: default_native_slots.into_boxed_slice(),
                });
                self.runtime_class_cache
                    .borrow_mut()
                    .insert(key.clone(), Rc::clone(&prepared));
                Some(prepared)
            };
            let Some(prepared) = prepared else {
                continue;
            };
            self.trusted_class_plans[index] = php_jit::JitNativePreparedClassPlan {
                prepared: Rc::as_ptr(&prepared) as usize as u64,
                display_name_bytes: prepared.display_name.as_ptr() as usize as u64,
                display_name_length: prepared.display_name.len() as u64,
                state: php_jit::JIT_NATIVE_PREPARED_CLASS_ALLOCATABLE,
                reserved: 0,
            };
        }
    }

    /// Resolve immutable constant sites at the request/publication boundary.
    /// A namespace fallback is deliberately not cached: defining the primary
    /// name later in the request must change subsequent lookup. Class
    /// constants are published only when resolution is effect-free and
    /// independent of the late-static calling class.
    pub(super) fn prepare_trusted_constant_fetches(&mut self) {
        // Build exact publication sites in one pass. The former name set fed
        // every distinct name back into `publish_trusted_constant_name`,
        // which rescanned every function and continuation for each name.
        let mut sites = std::collections::BTreeMap::<String, Vec<(u32, u32)>>::new();
        let mut class_sites = Vec::<(u32, u32, String, String)>::new();
        for function in self.published_native_functions() {
            let Some(instructions) = self.prepared_continuation_instructions(function) else {
                continue;
            };
            let function = function.raw();
            for (continuation, instruction) in instructions.iter().enumerate() {
                let Some(instruction) = instruction.as_ref() else {
                    continue;
                };
                let Ok(continuation) = u32::try_from(continuation) else {
                    continue;
                };
                match &instruction.kind {
                    php_ir::InstructionKind::FetchConst { name, .. } => {
                        sites
                            .entry(name.clone())
                            .or_default()
                            .push((function, continuation));
                        if let Some(index) = self
                            .trusted_property_function_offsets
                            .get(function as usize)
                            .and_then(|base| usize::try_from(*base).ok())
                            .and_then(|base| base.checked_add(continuation as usize))
                        {
                            let published = self
                                .trusted_dynamic_constant_sites
                                .entry(name.clone())
                                .or_default();
                            if !published.contains(&index) {
                                published.push(index);
                            }
                        }
                    }
                    php_ir::InstructionKind::FetchClassConstant {
                        class_name,
                        constant,
                        ..
                    } => class_sites.push((
                        function,
                        continuation,
                        class_name.clone(),
                        constant.clone(),
                    )),
                    _ => {}
                }
            }
        }
        for (name, sites) in sites {
            let Ok(encoded) = self.encode_named_runtime_constant_owned(&name, 0) else {
                continue;
            };
            for (function, continuation) in sites {
                let _ = self.publish_trusted_constant_fetch(function, continuation, encoded);
            }
            let _ = self.release(encoded);
        }
        for (function, continuation, class_name, constant) in class_sites {
            let Some(encoded) =
                self.prepare_effect_free_class_constant_owned(function, &class_name, &constant)
            else {
                continue;
            };
            let _ = self.publish_trusted_constant_fetch(function, continuation, encoded);
            let _ = self.release(encoded);
        }
    }

    /// Publishes only class constants whose lookup cannot autoload, diagnose,
    /// depend on visibility, or vary with late-static binding. More dynamic
    /// sites are populated by the completed baseline continuation after its
    /// PHP-visible effects have run.
    pub(super) fn prepare_effect_free_class_constant_owned(
        &mut self,
        caller_function: u32,
        class_name: &str,
        constant_name: &str,
    ) -> Option<i64> {
        fn is_direct_literal(constant: &php_ir::IrConstant) -> bool {
            match constant {
                php_ir::IrConstant::NamedConstant(_) | php_ir::IrConstant::ClassConstant { .. } => {
                    false
                }
                php_ir::IrConstant::Array(entries) => entries.iter().all(|entry| {
                    entry.key.as_ref().is_none_or(is_direct_literal)
                        && is_direct_literal(&entry.value)
                }),
                _ => true,
            }
        }

        let mut resolved_class = match class_name.to_ascii_lowercase().as_str() {
            "static" => return None,
            "self" => native_effective_calling_class(self, caller_function)?
                .name
                .clone(),
            "parent" => native_effective_calling_class(self, caller_function)?
                .parent
                .clone()?,
            _ => normalize_class_name(class_name),
        };
        if let Some(original) = self
            .class_aliases
            .get(&normalize_class_name(&resolved_class))
        {
            resolved_class = original.clone();
        }
        if constant_name.eq_ignore_ascii_case("class") {
            let display = native_active_class_handle(self, &resolved_class)
                .map(|class| class.display_name.clone())
                .or_else(|| {
                    native_external_class_handle(self, &resolved_class)
                        .map(|(_, class)| class.display_name.clone())
                })
                .unwrap_or(resolved_class);
            return self
                .encode_native_string_owner(PhpString::from_bytes(display.into_bytes()))
                .ok();
        }

        resolved_class = normalize_class_name(&resolved_class);
        if class_name.eq_ignore_ascii_case("ArrayObject")
            && constant_name.eq_ignore_ascii_case("ARRAY_AS_PROPS")
        {
            return Some(2);
        }
        if pdo_mysql_deprecated_constant(&resolved_class, constant_name).is_some() {
            return None;
        }
        if let Some(value) = native_internal_class_constant(&resolved_class, constant_name) {
            return self.encode_baseline_value(value).ok();
        }

        let mut candidate = resolved_class;
        loop {
            let (owner_unit, class) =
                if let Some(class) = native_active_class_handle(self, &candidate) {
                    (None, class)
                } else if let Some((unit, class)) = native_external_class_handle(self, &candidate) {
                    (Some(unit), class)
                } else {
                    return None;
                };
            if let Some(entry) = class
                .constants
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(constant_name))
            {
                if entry.flags.is_private || entry.flags.is_protected {
                    return None;
                }
                let constant = entry.value.and_then(|value| {
                    owner_unit.map_or_else(
                        || self.unit.constants.get(value.index()),
                        |unit| {
                            self.dynamic_units.get(unit).and_then(|package| {
                                package.compiled.unit().constants.get(value.index())
                            })
                        },
                    )
                })?;
                if !is_direct_literal(constant) {
                    return None;
                }
                let constant = constant.clone();
                return self.encode_native_ir_constant_owned(&constant).ok();
            }
            if class
                .enum_cases
                .iter()
                .any(|case| case.name.eq_ignore_ascii_case(constant_name))
            {
                return None;
            }
            candidate = normalize_class_name(class.parent.as_deref()?);
        }
    }
    /// Resolves a named constant into an independently owned native encoding.
    /// Compiled declarations and native `define()` values remain in their
    /// authoritative representation. Extension constants cross their one
    /// explicit cold boundary only when no compiled/native declaration
    /// exists.
    pub(super) fn encode_named_runtime_constant_owned(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<i64, String> {
        if depth > 32 {
            return Err("native constant resolution exceeded its recursion limit".to_owned());
        }
        if let Some(encoded) = self.native_dynamic_constants.get(name).copied() {
            return self
                .duplicate_authoritative_native_value(encoded)?
                .ok_or_else(|| {
                    format!("native dynamic constant {name} is not authoritative native data")
                });
        }
        if let Some(constant) = self
            .unit
            .constant_table
            .iter()
            .find(|constant| constant.name == name)
            .and_then(|constant| self.unit.constants.get(constant.value.index()))
            .cloned()
        {
            return self.encode_native_ir_constant_owned_at_depth(&constant, depth + 1);
        }
        php_std::ExtensionRegistry::standard_library()
            .enabled_constant(name)
            .and_then(php_std::ConstantDescriptor::value)
            .map(php_std::constants::constant_to_value)
            .ok_or_else(|| format!("Undefined constant \"{name}\""))
            .and_then(|value| self.encode_baseline_value(value))
    }
}
