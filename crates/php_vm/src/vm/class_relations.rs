use super::prelude::*;

pub(super) fn class_is_or_extends(
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
        if let Some(parent) = class.parent.as_deref() {
            let parent = normalize_class_name(parent);
            if internal_runtime_class_entry(&parent).is_some() {
                return Ok(internal_runtime_class_is_or_extends(
                    &parent,
                    &ancestor_name,
                ));
            }
        }
        let Some(parent) = parent_class(compiled, class)? else {
            return Ok(false);
        };
        class = parent;
    }
}

pub(super) fn class_is_or_implements(
    compiled: &CompiledUnit,
    class_name: &str,
    target_name: &str,
) -> Result<bool, String> {
    if class_is_or_extends(compiled, class_name, target_name)? {
        return Ok(true);
    }
    class_implements_interface(compiled, class_name, target_name, &mut Vec::new())
}

pub(super) fn class_extends_php_token(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class: &php_ir::module::ClassEntry,
) -> bool {
    let mut parent = class.parent.clone();
    let mut seen = HashSet::new();
    while let Some(parent_name) = parent {
        let normalized = normalize_class_name(&parent_name);
        if is_php_token_runtime_class(&normalized) {
            return true;
        }
        if !seen.insert(normalized.clone()) {
            return false;
        }
        parent = lookup_class_in_state(compiled, state, &normalized)
            .and_then(|entry| entry.parent.clone());
    }
    false
}

pub(super) fn internal_runtime_parent_name(class_name: &str) -> Option<String> {
    let class_name = normalize_class_name(class_name);
    if is_spl_iterator_runtime_class(&class_name) {
        match class_name.as_str() {
            "recursivearrayiterator" => Some(normalize_class_name("ArrayIterator")),
            _ => spl_iterator_class(&class_name).parent,
        }
    } else if is_spl_container_runtime_class(&class_name) {
        spl_container_class(&class_name).parent
    } else if is_spl_heap_runtime_class(&class_name) {
        spl_heap_class(&class_name).parent
    } else if is_spl_file_runtime_class(&class_name) {
        spl_file_class(&class_name).parent
    } else if internal_throwable_instanceof(&class_name, "throwable").is_some() {
        internal_throwable_parent(&class_name).map(normalize_class_name)
    } else {
        None
    }
}

pub(super) fn internal_runtime_class_is_or_extends(class_name: &str, ancestor_name: &str) -> bool {
    let mut class_name = normalize_class_name(class_name);
    let ancestor_name = normalize_class_name(ancestor_name);
    let mut seen = Vec::new();
    loop {
        if class_name == ancestor_name {
            return true;
        }
        if seen.iter().any(|name| name == &class_name) {
            return false;
        }
        seen.push(class_name.clone());
        let parent = internal_runtime_parent_name(&class_name);
        let Some(parent) = parent else {
            return false;
        };
        class_name = parent;
    }
}

pub(super) fn class_implements_interface(
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
    if let Some(parent) = class.parent.as_deref() {
        let parent = normalize_class_name(parent);
        if internal_runtime_class_entry(&parent).is_some() {
            if interface_or_extends(compiled, &parent, &interface_name, &mut Vec::new())? {
                seen.pop();
                return Ok(true);
            }
            for interface in internal_class_interfaces(&parent) {
                if interface_or_extends(compiled, &interface, &interface_name, &mut Vec::new())? {
                    seen.pop();
                    return Ok(true);
                }
            }
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

pub(super) fn collect_class_interface_display_names(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    class_name: &str,
    seen: &mut Vec<(String, String)>,
) {
    let normalized = normalize_class_name(class_name);
    let Some(class) = lookup_class_in_state(compiled, state, &normalized) else {
        for interface in internal_class_interfaces(&normalized) {
            collect_interface_display_names(compiled, &interface, seen);
        }
        if let Some(parent) = internal_runtime_parent_name(&normalized) {
            collect_class_interface_display_names(compiled, state, &parent, seen);
        }
        return;
    };
    for interface in &class.interfaces {
        collect_interface_display_names(compiled, interface, seen);
    }
    if let Some(parent) = class.parent.as_deref() {
        collect_class_interface_display_names(compiled, state, parent, seen);
    }
}

pub(super) fn collect_interface_display_names(
    compiled: &CompiledUnit,
    interface_name: &str,
    seen: &mut Vec<(String, String)>,
) {
    let normalized = normalize_class_name(interface_name);
    if seen.iter().any(|(name, _)| name == &normalized) {
        return;
    }
    let display = compiled
        .lookup_class(&normalized)
        .map(|class| class.display_name.clone())
        .unwrap_or_else(|| interface_name.to_owned());
    seen.push((normalized.clone(), display));
    if let Some(interface) = compiled.lookup_class(&normalized) {
        for parent in &interface.interfaces {
            collect_interface_display_names(compiled, parent, seen);
        }
    } else {
        for parent in internal_class_interfaces(&normalized) {
            collect_interface_display_names(compiled, &parent, seen);
        }
    }
}

pub(super) fn interface_or_extends(
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
        for parent in internal_class_interfaces(&interface_name) {
            if interface_or_extends(compiled, &parent, &target_name, seen)? {
                return Ok(true);
            }
        }
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

pub(super) fn class_relation_subject_name(value: &Value) -> Option<String> {
    match value {
        Value::Reference(cell) => class_relation_subject_name(&cell.get()),
        Value::Object(object) => Some(normalize_class_name(&object.class_name())),
        Value::Fiber(_) => Some("fiber".to_owned()),
        Value::Callable(_) => Some("closure".to_owned()),
        _ => None,
    }
}

pub(super) fn class_relation_config_fingerprint(compiled: &CompiledUnit) -> String {
    format!(
        "unit:{}:strict:{}",
        compiled.unit().id.raw(),
        compiled.unit().strict_types
    )
}

pub(super) fn object_instanceof(
    compiled: &CompiledUnit,
    value: &Value,
    class_name: &str,
) -> Result<bool, String> {
    match value {
        Value::Reference(cell) => object_instanceof(compiled, &cell.get(), class_name),
        Value::Fiber(_) => Ok(normalize_class_name(class_name) == "fiber"),
        Value::Callable(_) => Ok(is_closure_runtime_class(class_name)),
        Value::Object(object) => {
            if is_std_class_runtime_class(&object.class_name())
                && is_std_class_runtime_class(class_name)
            {
                return Ok(true);
            }
            if let Some(result) = internal_hash_context_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) = internal_php_token_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) =
                internal_throwable_instanceof(&object.class_name_handle(), class_name)
            {
                return Ok(result);
            }
            if class_is_or_implements(compiled, &object.class_name(), class_name)? {
                return Ok(true);
            }
            if let Some(spl_class) = spl_runtime_marker(object) {
                if let Some(result) = internal_spl_iterator_instanceof(&spl_class, class_name) {
                    return Ok(result);
                }
                if let Some(result) = internal_spl_container_instanceof(&spl_class, class_name) {
                    return Ok(result);
                }
                if let Some(result) = internal_spl_heap_instanceof(&spl_class, class_name) {
                    return Ok(result);
                }
                if let Some(result) = internal_spl_file_instanceof(&spl_class, class_name) {
                    return Ok(result);
                }
            }
            if let Some(result) = internal_spl_iterator_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) =
                internal_spl_container_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) = internal_spl_heap_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_spl_file_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_date_time_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_sqlite_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_pdo_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_mysqli_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_redis_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_memcached_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_soap_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_fileinfo_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_phar_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_zip_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_gd_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) =
                internal_extension_resource_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            class_is_or_implements(compiled, &object.class_name(), class_name)
        }
        _ => Ok(false),
    }
}

pub(super) fn object_instanceof_in_state(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    value: &Value,
    class_name: &str,
) -> Result<bool, String> {
    match value {
        Value::Reference(cell) => {
            object_instanceof_in_state(compiled, state, &cell.get(), class_name)
        }
        Value::Fiber(_) => Ok(normalize_class_name(class_name) == "fiber"),
        Value::Callable(_) => Ok(is_closure_runtime_class(class_name)),
        Value::Object(object) => {
            if let Some(result) = internal_hash_context_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) = internal_php_token_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) =
                internal_throwable_instanceof(&object.class_name_handle(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) = internal_spl_iterator_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) =
                internal_spl_container_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            if let Some(result) = internal_spl_file_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_date_time_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_sqlite_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_pdo_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_mysqli_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_redis_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_memcached_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_soap_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_fileinfo_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_phar_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_zip_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) = internal_gd_instanceof(&object.class_name(), class_name) {
                return Ok(result);
            }
            if let Some(result) =
                internal_extension_resource_instanceof(&object.class_name(), class_name)
            {
                return Ok(result);
            }
            class_is_a_in_state(compiled, state, &object.class_name(), class_name)
        }
        _ => Ok(false),
    }
}

pub(super) fn iterator_function_accepts_source(
    compiled: &CompiledUnit,
    state: &ExecutionState,
    value: &Value,
) -> Result<bool, String> {
    match effective_value(value) {
        Value::Array(_) | Value::Generator(_) => Ok(true),
        Value::Object(object) => {
            if let Some(spl_class) = spl_runtime_marker(&object) {
                return Ok(internal_spl_iterator_instanceof(&spl_class, "Traversable")
                    .or_else(|| internal_spl_container_instanceof(&spl_class, "Traversable"))
                    .or_else(|| internal_spl_heap_instanceof(&spl_class, "Traversable"))
                    .or_else(|| internal_spl_file_instanceof(&spl_class, "Traversable"))
                    .unwrap_or(false));
            }
            object_instanceof_in_state(compiled, state, &Value::Object(object), "Traversable")
        }
        _ => Ok(false),
    }
}
