use super::prelude::*;

pub(super) fn is_fiber_runtime_class(class_name: &str) -> bool {
    normalize_class_name(class_name) == "fiber"
}

pub(super) fn is_closure_runtime_class(class_name: &str) -> bool {
    normalize_class_name(class_name) == "closure"
}

/// Returns true when a statically named `new` expression can lower to the
/// dense `NewObject` opcode. Builtin runtime classes keep their dedicated
/// rich-interpreter construction paths; everything else resolves through
/// the shared userland instantiation helpers at execution time (including
/// autoload, abstract/interface/enum guards, and constructor dispatch).
pub(crate) fn dense_new_object_lowering_supported(class_name: &str) -> bool {
    !(is_special_static_class_name(class_name)
        || is_closure_runtime_class(class_name)
        || is_fiber_runtime_class(class_name)
        || is_reflection_runtime_class(class_name)
        || is_phar_runtime_class(class_name)
        || is_zip_runtime_class(class_name)
        || is_xml_runtime_class(class_name)
        || is_pdo_runtime_class(class_name)
        || is_sqlite_runtime_class(class_name)
        || is_spl_iterator_runtime_class(class_name)
        || is_spl_container_runtime_class(class_name)
        || is_spl_heap_runtime_class(class_name)
        || is_spl_file_runtime_class(class_name)
        || is_std_class_runtime_class(class_name)
        || is_php_token_runtime_class(class_name)
        || is_fileinfo_runtime_class(class_name)
        || is_imagick_runtime_class(class_name)
        || is_xsl_runtime_class(class_name)
        || is_soap_runtime_class(class_name)
        || is_date_time_runtime_class(class_name))
}

pub(super) fn runtime_class_entry(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
    constant_value: &impl Fn(ConstId) -> Result<Value, String>,
    _class_constant_reference_value: &impl Fn(&ClassConstantReference) -> Result<Value, String>,
    _named_constant_reference_value: &impl Fn(&NamedConstantReference) -> Result<Value, String>,
) -> Result<RuntimeClassEntry, RuntimeClassEntryError> {
    let mut lineage = Vec::new();
    collect_class_lineage(compiled, state, class, &mut lineage)
        .map_err(RuntimeClassEntryError::new)?;
    let mut properties = Vec::new();
    let mut constants = Vec::new();
    for lineage_class in &lineage {
        let owner = class_owner_in_state(compiled, state, &lineage_class.name);
        push_runtime_properties(&owner, state, lineage_class, &mut properties)?;
        push_runtime_constants(&owner, state, lineage_class, &mut constants)?;
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
                    attributes: runtime_attributes(&method.attributes, constant_value)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        properties,
        constants,
        enum_cases: push_runtime_enum_cases(class, constant_value)?,
        attributes: runtime_attributes(&class.attributes, constant_value)?,
        enum_backing_type: class.enum_backing_type.map(|backing| match backing {
            php_ir::module::ClassEnumBackingType::Int => RuntimeClassEnumBackingType::Int,
            php_ir::module::ClassEnumBackingType::String => RuntimeClassEnumBackingType::String,
        }),
        constructor_id: class.constructor.map(|function| function.raw()),
        flags: RuntimeClassFlags {
            is_abstract: class.flags.is_abstract || class.flags.is_trait,
            is_final: class.flags.is_final,
            is_readonly: class.flags.is_readonly,
            is_interface: class.flags.is_interface,
            is_enum: class.flags.is_enum,
        },
    })
}

pub(super) fn collect_class_lineage(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
    lineage: &mut Vec<php_ir::module::ClassEntry>,
) -> Result<(), String> {
    collect_class_lineage_inner(compiled, state, class, lineage, &mut Vec::new())
}

pub(super) fn collect_class_lineage_inner(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
    lineage: &mut Vec<php_ir::module::ClassEntry>,
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
        let Some(parent) = lookup_class_in_state(compiled, state, parent_name) else {
            return Err(format!(
                "E_PHP_VM_UNKNOWN_PARENT_CLASS: class {} extends missing class {}",
                class.name, parent_name
            ));
        };
        collect_class_lineage_inner(compiled, state, &parent, lineage, seen)?;
    }
    lineage.push(class.clone());
    seen.pop();
    Ok(())
}

pub(super) fn collect_class_lineage_compiled<'a>(
    compiled: &'a CompiledUnit,
    class: &'a php_ir::module::ClassEntry,
    lineage: &mut Vec<&'a php_ir::module::ClassEntry>,
) -> Result<(), String> {
    collect_class_lineage_compiled_inner(compiled, class, lineage, &mut Vec::new())
}

pub(super) fn collect_class_lineage_compiled_inner<'a>(
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
            if internal_runtime_class_entry(&normalize_class_name(parent_name)).is_some() {
                lineage.push(class);
                seen.pop();
                return Ok(());
            }
            return Err(format!(
                "E_PHP_VM_UNKNOWN_PARENT_CLASS: class {} extends missing class {}",
                class.name, parent_name
            ));
        };
        collect_class_lineage_compiled_inner(compiled, parent, lineage, seen)?;
    }
    lineage.push(class);
    seen.pop();
    Ok(())
}

pub(super) fn push_runtime_properties(
    owner: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
    properties: &mut Vec<RuntimeClassPropertyEntry>,
) -> Result<(), RuntimeClassEntryError> {
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
                attributes: runtime_attributes(&property.attributes, &|value| {
                    constant_value(owner.unit(), value)
                })
                .map_err(RuntimeClassEntryError::new)?,
            });
            continue;
        }
        let default = if let Some(default) = property.default {
            constant_value(owner.unit(), default).map_err(RuntimeClassEntryError::new)?
        } else if let Some(reference) = &property.default_class_constant {
            class_constant_reference_value(owner, state, reference)
                .map_err(RuntimeClassEntryError::new)?
        } else if let Some(reference) = &property.default_named_constant {
            named_constant_reference_value(owner, state, reference)
                .map_err(RuntimeClassEntryError::new)?
        } else if let Some(expr) = &property.default_expr {
            deferred_const_expr_value(owner, state, expr).map_err(RuntimeClassEntryError::new)?
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
            attributes: runtime_attributes(&property.attributes, &|value| {
                constant_value(owner.unit(), value)
            })
            .map_err(RuntimeClassEntryError::new)?,
        });
    }
    Ok(())
}

pub(super) fn push_runtime_constants(
    owner: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
    constants: &mut Vec<RuntimeClassConstantEntry>,
) -> Result<(), RuntimeClassEntryError> {
    for constant in &class.constants {
        let value = if let Some(value) = constant.value {
            constant_value(owner.unit(), value).map_err(|message| {
                RuntimeClassEntryError::with_constant_initializer_span(message, constant.span)
            })?
        } else if let Some(reference) = &constant.value_class_constant {
            class_constant_reference_value(owner, state, reference).map_err(|message| {
                RuntimeClassEntryError::with_constant_initializer_span(message, constant.span)
            })?
        } else if let Some(reference) = &constant.value_named_constant {
            named_constant_reference_value(owner, state, reference).map_err(|message| {
                RuntimeClassEntryError::with_constant_initializer_span(message, constant.span)
            })?
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
            attributes: runtime_attributes(&constant.attributes, &|value| {
                constant_value(owner.unit(), value)
            })
            .map_err(RuntimeClassEntryError::new)?,
        });
    }
    Ok(())
}

pub(super) fn push_runtime_enum_cases(
    class: &php_ir::module::ClassEntry,
    constant_value: &impl Fn(ConstId) -> Result<Value, String>,
) -> Result<Vec<RuntimeClassEnumCaseEntry>, String> {
    class
        .enum_cases
        .iter()
        .map(|case| {
            Ok(RuntimeClassEnumCaseEntry {
                name: case.name.clone(),
                value: case.value.map(constant_value).transpose()?,
                attributes: runtime_attributes(&case.attributes, constant_value)?,
            })
        })
        .collect()
}

pub(super) fn runtime_attributes(
    attributes: &[php_ir::module::AttributeEntry],
    constant_value: &impl Fn(ConstId) -> Result<Value, String>,
) -> Result<Vec<RuntimeAttributeEntry>, String> {
    attributes
        .iter()
        .map(|attribute| {
            let arguments = attribute
                .arguments
                .iter()
                .map(|argument| constant_value(*argument))
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
