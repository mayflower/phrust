//! Optional VM/runtime counters for Phase 7 performance instrumentation.

use std::collections::BTreeMap;

use php_ir::instruction::{BinaryOp, InstructionKind};
use php_runtime::OutputStats;

use crate::{InlineCacheKind, InlineCacheObservation};

/// Lightweight counters collected only when explicitly enabled.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VmCounters {
    pub instructions_executed: u64,
    pub opcodes: BTreeMap<String, u64>,
    pub function_calls: u64,
    pub method_calls: u64,
    pub frame_allocations: u64,
    pub frame_reuses: u64,
    pub array_dim_fetches: u64,
    pub packed_dim_fast_path_hits: u64,
    pub packed_dim_fast_path_misses: u64,
    pub array_packed_append_fast_path_hits: u64,
    pub array_packed_read_fast_path_hits: u64,
    pub array_sequential_foreach_fast_path_hits: u64,
    pub array_count_fast_path_hits: u64,
    pub array_packed_to_mixed_transitions: u64,
    pub numeric_string_cache_hits: u64,
    pub numeric_string_cache_misses: u64,
    pub typecheck_fast_path_hits: u64,
    pub typecheck_fast_path_misses: u64,
    pub output_bytes: u64,
    pub output_buffer_appends: u64,
    pub output_buffer_batch_writes: u64,
    pub output_buffer_flushes: u64,
    pub internal_function_dispatches: u64,
    pub internal_function_dispatch_cache_hits: u64,
    pub internal_function_dispatch_cache_misses: u64,
    pub internal_count_array_direct_fast_path_hits: u64,
    pub local_slot_fast_path_hits: u64,
    pub local_slot_fast_path_misses: u64,
    pub property_fetches: u64,
    pub property_accesses: u64,
    pub type_checks: u64,
    pub includes: u64,
    pub autoloads: u64,
    pub string_concats: u64,
    pub string_concat_fast_path_hits: u64,
    pub string_concat_fast_path_misses: u64,
    pub guard_failures: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub literal_intern_hits: u64,
    pub literal_intern_misses: u64,
    pub quickening_attempts: u64,
    pub quickening_specialized: u64,
    pub quickening_guard_hits: u64,
    pub quickening_guard_misses: u64,
    pub quickening_guard_failures: u64,
    pub quickening_fallback_calls: u64,
    pub quickening_dequickens: u64,
    pub quickening_megamorphic: u64,
    pub quickening_disabled: u64,
    pub jit_compile_attempts: u64,
    pub jit_compiled: u64,
    pub jit_executed: u64,
    pub jit_bailouts: u64,
    pub inline_cache_observations: u64,
    pub inline_cache_slots: u64,
    pub inline_cache_function_slots: u64,
    pub inline_cache_method_slots: u64,
    pub inline_cache_property_slots: u64,
    pub inline_cache_dim_slots: u64,
    pub inline_cache_class_constant_static_property_slots: u64,
    pub inline_cache_include_path_slots: u64,
    pub inline_cache_autoload_class_lookup_slots: u64,
    pub inline_cache_hits: u64,
    pub inline_cache_misses: u64,
    pub inline_cache_invalidations: u64,
    pub inline_cache_guard_failures: u64,
    pub inline_cache_fallback_calls: u64,
    pub inline_cache_monomorphic: u64,
    pub inline_cache_polymorphic: u64,
    pub inline_cache_megamorphic: u64,
    pub inline_cache_disabled: u64,
    pub method_ic_hits: u64,
    pub method_ic_misses: u64,
    pub method_ic_guard_failures: u64,
    pub property_ic_hits: u64,
    pub property_ic_misses: u64,
    pub property_ic_guard_failures: u64,
    pub class_static_ic_hits: u64,
    pub class_static_ic_misses: u64,
    pub class_static_ic_guard_failures: u64,
    pub include_path_ic_hits: u64,
    pub include_path_ic_misses: u64,
    pub include_path_ic_invalidations: u64,
    pub include_path_ic_guard_failures: u64,
    pub autoload_class_lookup_ic_hits: u64,
    pub autoload_class_lookup_ic_misses: u64,
    pub autoload_class_lookup_ic_invalidations: u64,
    pub autoload_class_lookup_ic_guard_failures: u64,
}

impl VmCounters {
    pub(crate) fn record_instruction(&mut self, kind: &InstructionKind) {
        self.instructions_executed += 1;
        *self
            .opcodes
            .entry(opcode_name(kind).to_owned())
            .or_default() += 1;
        match kind {
            InstructionKind::BindReferenceFromCall { .. }
            | InstructionKind::CallFunction { .. }
            | InstructionKind::CallClosure { .. }
            | InstructionKind::CallCallable { .. }
            | InstructionKind::Pipe { .. } => self.function_calls += 1,
            InstructionKind::CallMethod { .. } | InstructionKind::CallStaticMethod { .. } => {
                self.method_calls += 1;
            }
            InstructionKind::BindReferenceDim { .. }
            | InstructionKind::BindReferenceFromDim { .. }
            | InstructionKind::FetchDim { .. }
            | InstructionKind::ArrayGet { .. }
            | InstructionKind::IssetDim { .. }
            | InstructionKind::EmptyDim { .. }
            | InstructionKind::UnsetDim { .. } => self.array_dim_fetches += 1,
            InstructionKind::FetchProperty { .. } | InstructionKind::FetchStaticProperty { .. } => {
                self.property_fetches += 1;
                self.property_accesses += 1;
            }
            InstructionKind::IssetProperty { .. }
            | InstructionKind::EmptyProperty { .. }
            | InstructionKind::UnsetProperty { .. }
            | InstructionKind::AssignProperty { .. }
            | InstructionKind::AssignStaticProperty { .. } => self.property_accesses += 1,
            InstructionKind::InstanceOf { .. } => self.type_checks += 1,
            InstructionKind::Include { .. } => self.includes += 1,
            InstructionKind::Binary {
                op: BinaryOp::Concat,
                ..
            } => self.string_concats += 1,
            _ => {}
        }
    }

    pub(crate) fn record_autoload(&mut self) {
        self.autoloads += 1;
    }

    pub(crate) fn record_frame_activation(&mut self, reused: bool) {
        if reused {
            self.frame_reuses += 1;
        } else {
            self.frame_allocations += 1;
        }
    }

    pub(crate) fn record_literal_intern(&mut self, hit: bool) {
        if hit {
            self.literal_intern_hits += 1;
        } else {
            self.literal_intern_misses += 1;
        }
    }

    pub(crate) fn record_string_concat_fast_path(&mut self, hit: bool) {
        if hit {
            self.string_concat_fast_path_hits += 1;
        } else {
            self.string_concat_fast_path_misses += 1;
        }
    }

    pub(crate) fn record_packed_dim_fast_path(&mut self, hit: bool) {
        if hit {
            self.packed_dim_fast_path_hits += 1;
        } else {
            self.packed_dim_fast_path_misses += 1;
        }
    }

    pub(crate) fn record_array_packed_append_fast_path_hit(&mut self) {
        self.array_packed_append_fast_path_hits += 1;
    }

    pub(crate) fn record_array_packed_read_fast_path_hit(&mut self) {
        self.array_packed_read_fast_path_hits += 1;
    }

    pub(crate) fn record_array_sequential_foreach_fast_path_hit(&mut self) {
        self.array_sequential_foreach_fast_path_hits += 1;
    }

    pub(crate) fn record_array_count_fast_path_hit(&mut self) {
        self.array_count_fast_path_hits += 1;
    }

    pub(crate) fn record_array_packed_to_mixed_transition(&mut self) {
        self.array_packed_to_mixed_transitions += 1;
    }

    pub(crate) fn record_numeric_string_cache_stats(
        &mut self,
        stats: php_runtime::numeric_string::NumericStringCacheStats,
    ) {
        self.numeric_string_cache_hits += stats.hits;
        self.numeric_string_cache_misses += stats.misses;
    }

    pub(crate) fn record_typecheck_fast_path(&mut self, hit: bool) {
        if hit {
            self.typecheck_fast_path_hits += 1;
        } else {
            self.typecheck_fast_path_misses += 1;
        }
    }

    pub(crate) fn record_output_stats(&mut self, final_output_bytes: usize, stats: OutputStats) {
        self.output_bytes = final_output_bytes as u64;
        self.output_buffer_appends = stats.appends;
        self.output_buffer_batch_writes = stats.batch_writes;
        self.output_buffer_flushes = stats.flushes;
    }

    pub(crate) fn record_internal_function_dispatch(&mut self) {
        self.internal_function_dispatches += 1;
    }

    pub(crate) fn record_internal_function_dispatch_cache(&mut self, hit: bool) {
        if hit {
            self.internal_function_dispatch_cache_hits += 1;
        } else {
            self.internal_function_dispatch_cache_misses += 1;
        }
    }

    pub(crate) fn record_internal_count_array_direct_fast_path_hit(&mut self) {
        self.internal_count_array_direct_fast_path_hits += 1;
    }

    pub(crate) fn record_local_slot_fast_path(&mut self, hit: bool) {
        if hit {
            self.local_slot_fast_path_hits += 1;
        } else {
            self.local_slot_fast_path_misses += 1;
        }
    }

    pub(crate) fn record_quickening(&mut self, observation: crate::QuickeningObservation) {
        if observation.attempt {
            self.quickening_attempts += 1;
        }
        if observation.specialized {
            self.quickening_specialized += 1;
        }
        if observation.guard_hit {
            self.quickening_guard_hits += 1;
        }
        if observation.guard_miss {
            self.quickening_guard_misses += 1;
        }
        if observation.guard_failure {
            self.quickening_guard_failures += 1;
        }
        if observation.fallback_call {
            self.quickening_fallback_calls += 1;
        }
        if observation.dequickened {
            self.quickening_dequickens += 1;
        }
        if observation.megamorphic {
            self.quickening_megamorphic += 1;
        }
        if observation.disabled {
            self.quickening_disabled += 1;
        }
    }

    #[cfg_attr(not(feature = "jit-cranelift"), allow(dead_code))]
    pub(crate) fn record_jit_compile_attempt(&mut self) {
        self.jit_compile_attempts += 1;
    }

    #[cfg_attr(not(feature = "jit-cranelift"), allow(dead_code))]
    pub(crate) fn record_jit_compiled(&mut self) {
        self.jit_compiled += 1;
    }

    #[cfg_attr(not(feature = "jit-cranelift"), allow(dead_code))]
    pub(crate) fn record_jit_executed(&mut self) {
        self.jit_executed += 1;
    }

    #[cfg_attr(not(feature = "jit-cranelift"), allow(dead_code))]
    pub(crate) fn record_jit_bailout(&mut self) {
        self.jit_bailouts += 1;
    }

    pub(crate) fn record_inline_cache(&mut self, observation: InlineCacheObservation) {
        if observation.candidate {
            self.inline_cache_observations += 1;
        }
        if observation.hit {
            self.inline_cache_hits += 1;
            if observation.kind == Some(InlineCacheKind::MethodCall) {
                self.method_ic_hits += 1;
            }
            if observation.kind == Some(InlineCacheKind::PropertyFetch) {
                self.property_ic_hits += 1;
            }
            if observation.kind == Some(InlineCacheKind::ClassConstantStaticProperty) {
                self.class_static_ic_hits += 1;
            }
            if observation.kind == Some(InlineCacheKind::IncludePath) {
                self.include_path_ic_hits += 1;
            }
            if observation.kind == Some(InlineCacheKind::AutoloadClassLookup) {
                self.autoload_class_lookup_ic_hits += 1;
            }
        }
        if observation.miss {
            self.inline_cache_misses += 1;
            if observation.kind == Some(InlineCacheKind::MethodCall) {
                self.method_ic_misses += 1;
            }
            if observation.kind == Some(InlineCacheKind::PropertyFetch) {
                self.property_ic_misses += 1;
            }
            if observation.kind == Some(InlineCacheKind::ClassConstantStaticProperty) {
                self.class_static_ic_misses += 1;
            }
            if observation.kind == Some(InlineCacheKind::IncludePath) {
                self.include_path_ic_misses += 1;
            }
            if observation.kind == Some(InlineCacheKind::AutoloadClassLookup) {
                self.autoload_class_lookup_ic_misses += 1;
            }
        }
        if observation.invalidation {
            self.inline_cache_invalidations += 1;
            if observation.kind == Some(InlineCacheKind::IncludePath) {
                self.include_path_ic_invalidations += 1;
            }
            if observation.kind == Some(InlineCacheKind::AutoloadClassLookup) {
                self.autoload_class_lookup_ic_invalidations += 1;
            }
        }
        if observation.guard_failure {
            self.inline_cache_guard_failures += 1;
            if observation.kind == Some(InlineCacheKind::MethodCall) {
                self.method_ic_guard_failures += 1;
            }
            if observation.kind == Some(InlineCacheKind::PropertyFetch) {
                self.property_ic_guard_failures += 1;
            }
            if observation.kind == Some(InlineCacheKind::ClassConstantStaticProperty) {
                self.class_static_ic_guard_failures += 1;
            }
            if observation.kind == Some(InlineCacheKind::IncludePath) {
                self.include_path_ic_guard_failures += 1;
            }
            if observation.kind == Some(InlineCacheKind::AutoloadClassLookup) {
                self.autoload_class_lookup_ic_guard_failures += 1;
            }
        }
        if observation.fallback_call {
            self.inline_cache_fallback_calls += 1;
        }
        if observation.monomorphic {
            self.inline_cache_monomorphic += 1;
        }
        if observation.polymorphic {
            self.inline_cache_polymorphic += 1;
        }
        if observation.megamorphic {
            self.inline_cache_megamorphic += 1;
        }
        if observation.disabled {
            self.inline_cache_disabled += 1;
        }
        if !observation.slot_allocated {
            return;
        }
        self.inline_cache_slots += 1;
        match observation.kind {
            Some(InlineCacheKind::FunctionCall) => self.inline_cache_function_slots += 1,
            Some(InlineCacheKind::MethodCall) => self.inline_cache_method_slots += 1,
            Some(InlineCacheKind::PropertyFetch) => self.inline_cache_property_slots += 1,
            Some(InlineCacheKind::DimFetch) => self.inline_cache_dim_slots += 1,
            Some(InlineCacheKind::ClassConstantStaticProperty) => {
                self.inline_cache_class_constant_static_property_slots += 1;
            }
            Some(InlineCacheKind::IncludePath) => self.inline_cache_include_path_slots += 1,
            Some(InlineCacheKind::AutoloadClassLookup) => {
                self.inline_cache_autoload_class_lookup_slots += 1;
            }
            None => {}
        }
    }

    /// Serializes counters as stable JSON without adding serde to the VM crate.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        push_field(&mut json, "schema_version", 1, true);
        push_field(
            &mut json,
            "instructions_executed",
            self.instructions_executed,
            true,
        );
        json.push_str("  \"opcodes\": {");
        if self.opcodes.is_empty() {
            json.push('}');
        } else {
            json.push('\n');
            for (index, (name, count)) in self.opcodes.iter().enumerate() {
                json.push_str("    ");
                json.push('"');
                json.push_str(&escape_json(name));
                json.push_str("\": ");
                json.push_str(&count.to_string());
                if index + 1 != self.opcodes.len() {
                    json.push(',');
                }
                json.push('\n');
            }
            json.push_str("  }");
        }
        json.push_str(",\n");
        push_field(&mut json, "function_calls", self.function_calls, true);
        push_field(&mut json, "method_calls", self.method_calls, true);
        push_field(&mut json, "frame_allocations", self.frame_allocations, true);
        push_field(&mut json, "frame_reuses", self.frame_reuses, true);
        push_field(&mut json, "array_dim_fetches", self.array_dim_fetches, true);
        push_field(
            &mut json,
            "packed_dim_fast_path_hits",
            self.packed_dim_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "packed_dim_fast_path_misses",
            self.packed_dim_fast_path_misses,
            true,
        );
        push_field(
            &mut json,
            "array_packed_append_fast_path_hits",
            self.array_packed_append_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "array_packed_read_fast_path_hits",
            self.array_packed_read_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "array_sequential_foreach_fast_path_hits",
            self.array_sequential_foreach_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "array_count_fast_path_hits",
            self.array_count_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "array_packed_to_mixed_transitions",
            self.array_packed_to_mixed_transitions,
            true,
        );
        push_field(
            &mut json,
            "numeric_string_cache_hits",
            self.numeric_string_cache_hits,
            true,
        );
        push_field(
            &mut json,
            "numeric_string_cache_misses",
            self.numeric_string_cache_misses,
            true,
        );
        push_field(
            &mut json,
            "typecheck_fast_path_hits",
            self.typecheck_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "typecheck_fast_path_misses",
            self.typecheck_fast_path_misses,
            true,
        );
        push_field(&mut json, "output_bytes", self.output_bytes, true);
        push_field(
            &mut json,
            "output_buffer_appends",
            self.output_buffer_appends,
            true,
        );
        push_field(
            &mut json,
            "output_buffer_batch_writes",
            self.output_buffer_batch_writes,
            true,
        );
        push_field(
            &mut json,
            "output_buffer_flushes",
            self.output_buffer_flushes,
            true,
        );
        push_field(
            &mut json,
            "internal_function_dispatches",
            self.internal_function_dispatches,
            true,
        );
        push_field(
            &mut json,
            "internal_function_dispatch_cache_hits",
            self.internal_function_dispatch_cache_hits,
            true,
        );
        push_field(
            &mut json,
            "internal_function_dispatch_cache_misses",
            self.internal_function_dispatch_cache_misses,
            true,
        );
        push_field(
            &mut json,
            "internal_count_array_direct_fast_path_hits",
            self.internal_count_array_direct_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "local_slot_fast_path_hits",
            self.local_slot_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "local_slot_fast_path_misses",
            self.local_slot_fast_path_misses,
            true,
        );
        push_field(&mut json, "property_fetches", self.property_fetches, true);
        push_field(&mut json, "property_accesses", self.property_accesses, true);
        push_field(&mut json, "type_checks", self.type_checks, true);
        push_field(&mut json, "includes", self.includes, true);
        push_field(&mut json, "autoloads", self.autoloads, true);
        push_field(&mut json, "string_concats", self.string_concats, true);
        push_field(
            &mut json,
            "string_concat_fast_path_hits",
            self.string_concat_fast_path_hits,
            true,
        );
        push_field(
            &mut json,
            "string_concat_fast_path_misses",
            self.string_concat_fast_path_misses,
            true,
        );
        push_field(&mut json, "guard_failures", self.guard_failures, true);
        push_field(&mut json, "cache_hits", self.cache_hits, true);
        push_field(&mut json, "cache_misses", self.cache_misses, true);
        push_field(
            &mut json,
            "literal_intern_hits",
            self.literal_intern_hits,
            true,
        );
        push_field(
            &mut json,
            "literal_intern_misses",
            self.literal_intern_misses,
            true,
        );
        push_field(
            &mut json,
            "quickening_attempts",
            self.quickening_attempts,
            true,
        );
        push_field(
            &mut json,
            "quickening_specialized",
            self.quickening_specialized,
            true,
        );
        push_field(
            &mut json,
            "quickening_guard_hits",
            self.quickening_guard_hits,
            true,
        );
        push_field(
            &mut json,
            "quickening_guard_misses",
            self.quickening_guard_misses,
            true,
        );
        push_field(
            &mut json,
            "quickening_guard_failures",
            self.quickening_guard_failures,
            true,
        );
        push_field(
            &mut json,
            "quickening_fallback_calls",
            self.quickening_fallback_calls,
            true,
        );
        push_field(
            &mut json,
            "quickening_dequickens",
            self.quickening_dequickens,
            true,
        );
        push_field(
            &mut json,
            "quickening_megamorphic",
            self.quickening_megamorphic,
            true,
        );
        push_field(
            &mut json,
            "quickening_disabled",
            self.quickening_disabled,
            true,
        );
        push_field(
            &mut json,
            "jit_compile_attempts",
            self.jit_compile_attempts,
            true,
        );
        push_field(&mut json, "jit_compiled", self.jit_compiled, true);
        push_field(&mut json, "jit_executed", self.jit_executed, true);
        push_field(&mut json, "jit_bailouts", self.jit_bailouts, true);
        push_field(
            &mut json,
            "inline_cache_observations",
            self.inline_cache_observations,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_slots",
            self.inline_cache_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_function_slots",
            self.inline_cache_function_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_method_slots",
            self.inline_cache_method_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_property_slots",
            self.inline_cache_property_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_dim_slots",
            self.inline_cache_dim_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_class_constant_static_property_slots",
            self.inline_cache_class_constant_static_property_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_include_path_slots",
            self.inline_cache_include_path_slots,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_autoload_class_lookup_slots",
            self.inline_cache_autoload_class_lookup_slots,
            true,
        );
        push_field(&mut json, "inline_cache_hits", self.inline_cache_hits, true);
        push_field(
            &mut json,
            "inline_cache_misses",
            self.inline_cache_misses,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_invalidations",
            self.inline_cache_invalidations,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_guard_failures",
            self.inline_cache_guard_failures,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_fallback_calls",
            self.inline_cache_fallback_calls,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_monomorphic",
            self.inline_cache_monomorphic,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_polymorphic",
            self.inline_cache_polymorphic,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_megamorphic",
            self.inline_cache_megamorphic,
            true,
        );
        push_field(
            &mut json,
            "inline_cache_disabled",
            self.inline_cache_disabled,
            true,
        );
        push_field(&mut json, "method_ic_hits", self.method_ic_hits, true);
        push_field(&mut json, "method_ic_misses", self.method_ic_misses, true);
        push_field(
            &mut json,
            "method_ic_guard_failures",
            self.method_ic_guard_failures,
            true,
        );
        push_field(&mut json, "property_ic_hits", self.property_ic_hits, true);
        push_field(
            &mut json,
            "property_ic_misses",
            self.property_ic_misses,
            true,
        );
        push_field(
            &mut json,
            "property_ic_guard_failures",
            self.property_ic_guard_failures,
            true,
        );
        push_field(
            &mut json,
            "class_static_ic_hits",
            self.class_static_ic_hits,
            true,
        );
        push_field(
            &mut json,
            "class_static_ic_misses",
            self.class_static_ic_misses,
            true,
        );
        push_field(
            &mut json,
            "class_static_ic_guard_failures",
            self.class_static_ic_guard_failures,
            true,
        );
        push_field(
            &mut json,
            "include_path_ic_hits",
            self.include_path_ic_hits,
            true,
        );
        push_field(
            &mut json,
            "include_path_ic_misses",
            self.include_path_ic_misses,
            true,
        );
        push_field(
            &mut json,
            "include_path_ic_invalidations",
            self.include_path_ic_invalidations,
            true,
        );
        push_field(
            &mut json,
            "include_path_ic_guard_failures",
            self.include_path_ic_guard_failures,
            true,
        );
        push_field(
            &mut json,
            "autoload_class_lookup_ic_hits",
            self.autoload_class_lookup_ic_hits,
            true,
        );
        push_field(
            &mut json,
            "autoload_class_lookup_ic_misses",
            self.autoload_class_lookup_ic_misses,
            true,
        );
        push_field(
            &mut json,
            "autoload_class_lookup_ic_invalidations",
            self.autoload_class_lookup_ic_invalidations,
            true,
        );
        push_field(
            &mut json,
            "autoload_class_lookup_ic_guard_failures",
            self.autoload_class_lookup_ic_guard_failures,
            false,
        );
        json.push_str("}\n");
        json
    }
}

fn push_field(json: &mut String, name: &str, value: u64, comma: bool) {
    json.push_str("  \"");
    json.push_str(name);
    json.push_str("\": ");
    json.push_str(&value.to_string());
    if comma {
        json.push(',');
    }
    json.push('\n');
}

fn opcode_name(kind: &InstructionKind) -> &'static str {
    match kind {
        InstructionKind::Nop => "nop",
        InstructionKind::LoadConst { .. } => "load_const",
        InstructionKind::FetchConst { .. } => "fetch_const",
        InstructionKind::Move { .. } => "move",
        InstructionKind::LoadLocal { .. } => "load_local",
        InstructionKind::LoadLocalQuiet { .. } => "load_local_quiet",
        InstructionKind::StoreLocal { .. } => "store_local",
        InstructionKind::BindReference { .. } => "bind_reference",
        InstructionKind::BindGlobal { .. } => "bind_global",
        InstructionKind::BindReferenceDim { .. } => "bind_reference_dim",
        InstructionKind::BindReferenceFromDim { .. } => "bind_reference_from_dim",
        InstructionKind::BindReferenceFromCall { .. } => "bind_reference_from_call",
        InstructionKind::InitStaticLocal { .. } => "init_static_local",
        InstructionKind::Binary {
            op: BinaryOp::Concat,
            ..
        } => "binary_concat",
        InstructionKind::Binary { .. } => "binary",
        InstructionKind::Compare { .. } => "compare",
        InstructionKind::InstanceOf { .. } => "instanceof",
        InstructionKind::Unary { .. } => "unary",
        InstructionKind::Cast { .. } => "cast",
        InstructionKind::Discard { .. } => "discard",
        InstructionKind::Echo { .. } => "echo",
        InstructionKind::Yield { .. } => "yield",
        InstructionKind::YieldFrom { .. } => "yield_from",
        InstructionKind::CallFunction { .. } => "call_function",
        InstructionKind::CallMethod { .. } => "call_method",
        InstructionKind::CallStaticMethod { .. } => "call_static_method",
        InstructionKind::CloneObject { .. } => "clone_object",
        InstructionKind::CloneWith { .. } => "clone_with",
        InstructionKind::EnterTry { .. } => "enter_try",
        InstructionKind::LeaveTry => "leave_try",
        InstructionKind::EndFinally { .. } => "end_finally",
        InstructionKind::Throw { .. } => "throw",
        InstructionKind::MakeException { .. } => "make_exception",
        InstructionKind::MakeClosure { .. } => "make_closure",
        InstructionKind::CallClosure { .. } => "call_closure",
        InstructionKind::ResolveCallable { .. } => "resolve_callable",
        InstructionKind::CallCallable { .. } => "call_callable",
        InstructionKind::Pipe { .. } => "pipe",
        InstructionKind::Include { .. } => "include",
        InstructionKind::Eval { .. } => "eval",
        InstructionKind::NewObject { .. } => "new_object",
        InstructionKind::FetchProperty { .. } => "fetch_property",
        InstructionKind::IssetProperty { .. } => "isset_property",
        InstructionKind::EmptyProperty { .. } => "empty_property",
        InstructionKind::UnsetProperty { .. } => "unset_property",
        InstructionKind::FetchStaticProperty { .. } => "fetch_static_property",
        InstructionKind::FetchClassConstant { .. } => "fetch_class_constant",
        InstructionKind::AssignProperty { .. } => "assign_property",
        InstructionKind::AssignStaticProperty { .. } => "assign_static_property",
        InstructionKind::NewArray { .. } => "new_array",
        InstructionKind::ArrayInsert { .. } => "array_insert",
        InstructionKind::FetchDim { .. } => "fetch_dim",
        InstructionKind::AssignDim { .. } => "assign_dim",
        InstructionKind::AppendDim { .. } => "append_dim",
        InstructionKind::IssetLocal { .. } => "isset_local",
        InstructionKind::EmptyLocal { .. } => "empty_local",
        InstructionKind::UnsetLocal { .. } => "unset_local",
        InstructionKind::IssetDim { .. } => "isset_dim",
        InstructionKind::EmptyDim { .. } => "empty_dim",
        InstructionKind::UnsetDim { .. } => "unset_dim",
        InstructionKind::ForeachInit { .. } => "foreach_init",
        InstructionKind::ForeachNext { .. } => "foreach_next",
        InstructionKind::ForeachInitRef { .. } => "foreach_init_ref",
        InstructionKind::ForeachNextRef { .. } => "foreach_next_ref",
        InstructionKind::ArrayGet { .. } => "array_get",
        InstructionKind::Unsupported { .. } => "unsupported",
        InstructionKind::RuntimeError { .. } => "runtime_error",
    }
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use crate::{InlineCacheKind, InlineCacheObservation, QuickeningObservation};
    use php_ir::ids::RegId;
    use php_ir::instruction::{BinaryOp, InstructionKind};

    use super::{OutputStats, VmCounters};

    #[test]
    fn counters_classify_required_phase7_families() {
        let mut counters = VmCounters::default();
        counters.record_instruction(&InstructionKind::Binary {
            dst: RegId::new(0),
            op: BinaryOp::Concat,
            lhs: php_ir::operand::Operand::Register(RegId::new(1)),
            rhs: php_ir::operand::Operand::Register(RegId::new(2)),
        });
        counters.record_instruction(&InstructionKind::CallFunction {
            dst: RegId::new(1),
            name: "f".to_owned(),
            args: Vec::new(),
        });
        counters.record_frame_activation(false);
        counters.record_frame_activation(true);
        counters.record_autoload();
        counters.record_literal_intern(false);
        counters.record_literal_intern(true);
        counters.record_string_concat_fast_path(false);
        counters.record_string_concat_fast_path(true);
        counters.record_packed_dim_fast_path(false);
        counters.record_packed_dim_fast_path(true);
        counters.record_array_packed_append_fast_path_hit();
        counters.record_array_packed_read_fast_path_hit();
        counters.record_array_sequential_foreach_fast_path_hit();
        counters.record_array_count_fast_path_hit();
        counters.record_array_packed_to_mixed_transition();
        counters.record_numeric_string_cache_stats(
            php_runtime::numeric_string::NumericStringCacheStats { hits: 2, misses: 3 },
        );
        counters.record_typecheck_fast_path(false);
        counters.record_typecheck_fast_path(true);
        counters.record_output_stats(
            12,
            OutputStats {
                appends: 3,
                batch_writes: 1,
                flushes: 2,
            },
        );
        counters.record_internal_function_dispatch();
        counters.record_internal_function_dispatch_cache(false);
        counters.record_internal_function_dispatch_cache(true);
        counters.record_internal_count_array_direct_fast_path_hit();
        counters.record_local_slot_fast_path(false);
        counters.record_local_slot_fast_path(true);
        counters.record_quickening(QuickeningObservation {
            attempt: true,
            specialized: true,
            guard_hit: true,
            guard_miss: true,
            guard_failure: true,
            fallback_call: true,
            dequickened: true,
            megamorphic: true,
            disabled: true,
        });
        counters.record_jit_compile_attempt();
        counters.record_jit_compiled();
        counters.record_jit_executed();
        counters.record_jit_bailout();
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::FunctionCall),
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::MethodCall),
            hit: true,
            miss: true,
            guard_failure: true,
            fallback_call: true,
            monomorphic: true,
            megamorphic: true,
            disabled: true,
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::PropertyFetch),
            hit: true,
            miss: true,
            guard_failure: true,
            polymorphic: true,
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::DimFetch),
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::ClassConstantStaticProperty),
            hit: true,
            miss: true,
            guard_failure: true,
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::IncludePath),
            hit: true,
            miss: true,
            invalidation: true,
            guard_failure: true,
            ..InlineCacheObservation::empty()
        });
        counters.record_inline_cache(InlineCacheObservation {
            candidate: true,
            slot_allocated: true,
            kind: Some(InlineCacheKind::AutoloadClassLookup),
            hit: true,
            miss: true,
            invalidation: true,
            guard_failure: true,
            ..InlineCacheObservation::empty()
        });

        assert_eq!(counters.instructions_executed, 2);
        assert_eq!(counters.function_calls, 1);
        assert_eq!(counters.frame_allocations, 1);
        assert_eq!(counters.frame_reuses, 1);
        assert_eq!(counters.string_concats, 1);
        assert_eq!(counters.packed_dim_fast_path_hits, 1);
        assert_eq!(counters.packed_dim_fast_path_misses, 1);
        assert_eq!(counters.array_packed_append_fast_path_hits, 1);
        assert_eq!(counters.array_packed_read_fast_path_hits, 1);
        assert_eq!(counters.array_sequential_foreach_fast_path_hits, 1);
        assert_eq!(counters.array_count_fast_path_hits, 1);
        assert_eq!(counters.array_packed_to_mixed_transitions, 1);
        assert_eq!(counters.numeric_string_cache_hits, 2);
        assert_eq!(counters.numeric_string_cache_misses, 3);
        assert_eq!(counters.typecheck_fast_path_hits, 1);
        assert_eq!(counters.typecheck_fast_path_misses, 1);
        assert_eq!(counters.output_bytes, 12);
        assert_eq!(counters.output_buffer_appends, 3);
        assert_eq!(counters.output_buffer_batch_writes, 1);
        assert_eq!(counters.output_buffer_flushes, 2);
        assert_eq!(counters.internal_function_dispatches, 1);
        assert_eq!(counters.internal_function_dispatch_cache_hits, 1);
        assert_eq!(counters.internal_function_dispatch_cache_misses, 1);
        assert_eq!(counters.internal_count_array_direct_fast_path_hits, 1);
        assert_eq!(counters.local_slot_fast_path_hits, 1);
        assert_eq!(counters.local_slot_fast_path_misses, 1);
        assert_eq!(counters.string_concat_fast_path_hits, 1);
        assert_eq!(counters.string_concat_fast_path_misses, 1);
        assert_eq!(counters.autoloads, 1);
        assert_eq!(counters.literal_intern_hits, 1);
        assert_eq!(counters.literal_intern_misses, 1);
        assert_eq!(counters.quickening_attempts, 1);
        assert_eq!(counters.quickening_specialized, 1);
        assert_eq!(counters.quickening_guard_hits, 1);
        assert_eq!(counters.quickening_guard_misses, 1);
        assert_eq!(counters.quickening_guard_failures, 1);
        assert_eq!(counters.quickening_fallback_calls, 1);
        assert_eq!(counters.quickening_dequickens, 1);
        assert_eq!(counters.quickening_megamorphic, 1);
        assert_eq!(counters.quickening_disabled, 1);
        assert_eq!(counters.jit_compile_attempts, 1);
        assert_eq!(counters.jit_compiled, 1);
        assert_eq!(counters.jit_executed, 1);
        assert_eq!(counters.jit_bailouts, 1);
        assert_eq!(counters.inline_cache_observations, 7);
        assert_eq!(counters.inline_cache_slots, 7);
        assert_eq!(counters.inline_cache_function_slots, 1);
        assert_eq!(counters.inline_cache_method_slots, 1);
        assert_eq!(counters.inline_cache_property_slots, 1);
        assert_eq!(counters.inline_cache_dim_slots, 1);
        assert_eq!(
            counters.inline_cache_class_constant_static_property_slots,
            1
        );
        assert_eq!(counters.inline_cache_include_path_slots, 1);
        assert_eq!(counters.inline_cache_autoload_class_lookup_slots, 1);
        assert_eq!(counters.method_ic_hits, 1);
        assert_eq!(counters.method_ic_misses, 1);
        assert_eq!(counters.method_ic_guard_failures, 1);
        assert_eq!(counters.inline_cache_fallback_calls, 1);
        assert_eq!(counters.inline_cache_monomorphic, 1);
        assert_eq!(counters.inline_cache_polymorphic, 1);
        assert_eq!(counters.inline_cache_megamorphic, 1);
        assert_eq!(counters.inline_cache_disabled, 1);
        assert_eq!(counters.property_ic_hits, 1);
        assert_eq!(counters.property_ic_misses, 1);
        assert_eq!(counters.property_ic_guard_failures, 1);
        assert_eq!(counters.class_static_ic_hits, 1);
        assert_eq!(counters.class_static_ic_misses, 1);
        assert_eq!(counters.class_static_ic_guard_failures, 1);
        assert_eq!(counters.include_path_ic_hits, 1);
        assert_eq!(counters.include_path_ic_misses, 1);
        assert_eq!(counters.include_path_ic_invalidations, 1);
        assert_eq!(counters.include_path_ic_guard_failures, 1);
        assert_eq!(counters.autoload_class_lookup_ic_hits, 1);
        assert_eq!(counters.autoload_class_lookup_ic_misses, 1);
        assert_eq!(counters.autoload_class_lookup_ic_invalidations, 1);
        assert_eq!(counters.autoload_class_lookup_ic_guard_failures, 1);
        assert_eq!(counters.opcodes["binary_concat"], 1);
        assert_eq!(counters.opcodes["call_function"], 1);
    }

    #[test]
    fn counters_json_is_stable_and_parseable() {
        let mut counters = VmCounters::default();
        counters.record_instruction(&InstructionKind::Echo {
            src: php_ir::operand::Operand::Register(RegId::new(0)),
        });

        let json = counters.to_json();

        assert!(json.contains("\"instructions_executed\": 1"));
        assert!(json.contains("\"guard_failures\": 0"));
        assert!(json.contains("\"frame_allocations\": 0"));
        assert!(json.contains("\"frame_reuses\": 0"));
        assert!(json.contains("\"packed_dim_fast_path_hits\": 0"));
        assert!(json.contains("\"packed_dim_fast_path_misses\": 0"));
        assert!(json.contains("\"array_packed_append_fast_path_hits\": 0"));
        assert!(json.contains("\"array_packed_read_fast_path_hits\": 0"));
        assert!(json.contains("\"array_sequential_foreach_fast_path_hits\": 0"));
        assert!(json.contains("\"array_count_fast_path_hits\": 0"));
        assert!(json.contains("\"array_packed_to_mixed_transitions\": 0"));
        assert!(json.contains("\"numeric_string_cache_hits\": 0"));
        assert!(json.contains("\"numeric_string_cache_misses\": 0"));
        assert!(json.contains("\"typecheck_fast_path_hits\": 0"));
        assert!(json.contains("\"typecheck_fast_path_misses\": 0"));
        assert!(json.contains("\"output_bytes\": 0"));
        assert!(json.contains("\"output_buffer_appends\": 0"));
        assert!(json.contains("\"output_buffer_batch_writes\": 0"));
        assert!(json.contains("\"output_buffer_flushes\": 0"));
        assert!(json.contains("\"internal_function_dispatches\": 0"));
        assert!(json.contains("\"internal_function_dispatch_cache_hits\": 0"));
        assert!(json.contains("\"internal_function_dispatch_cache_misses\": 0"));
        assert!(json.contains("\"internal_count_array_direct_fast_path_hits\": 0"));
        assert!(json.contains("\"local_slot_fast_path_hits\": 0"));
        assert!(json.contains("\"local_slot_fast_path_misses\": 0"));
        assert!(json.contains("\"literal_intern_hits\": 0"));
        assert!(json.contains("\"literal_intern_misses\": 0"));
        assert!(json.contains("\"string_concat_fast_path_hits\": 0"));
        assert!(json.contains("\"string_concat_fast_path_misses\": 0"));
        assert!(json.contains("\"quickening_attempts\": 0"));
        assert!(json.contains("\"quickening_specialized\": 0"));
        assert!(json.contains("\"quickening_guard_hits\": 0"));
        assert!(json.contains("\"quickening_guard_misses\": 0"));
        assert!(json.contains("\"quickening_guard_failures\": 0"));
        assert!(json.contains("\"quickening_fallback_calls\": 0"));
        assert!(json.contains("\"quickening_dequickens\": 0"));
        assert!(json.contains("\"quickening_megamorphic\": 0"));
        assert!(json.contains("\"quickening_disabled\": 0"));
        assert!(json.contains("\"jit_compile_attempts\": 0"));
        assert!(json.contains("\"jit_compiled\": 0"));
        assert!(json.contains("\"jit_executed\": 0"));
        assert!(json.contains("\"jit_bailouts\": 0"));
        assert!(json.contains("\"inline_cache_observations\": 0"));
        assert!(json.contains("\"inline_cache_slots\": 0"));
        assert!(json.contains("\"inline_cache_function_slots\": 0"));
        assert!(json.contains("\"inline_cache_method_slots\": 0"));
        assert!(json.contains("\"inline_cache_property_slots\": 0"));
        assert!(json.contains("\"inline_cache_dim_slots\": 0"));
        assert!(json.contains("\"inline_cache_class_constant_static_property_slots\": 0"));
        assert!(json.contains("\"inline_cache_include_path_slots\": 0"));
        assert!(json.contains("\"inline_cache_autoload_class_lookup_slots\": 0"));
        assert!(json.contains("\"inline_cache_hits\": 0"));
        assert!(json.contains("\"inline_cache_misses\": 0"));
        assert!(json.contains("\"inline_cache_invalidations\": 0"));
        assert!(json.contains("\"inline_cache_guard_failures\": 0"));
        assert!(json.contains("\"inline_cache_fallback_calls\": 0"));
        assert!(json.contains("\"inline_cache_monomorphic\": 0"));
        assert!(json.contains("\"inline_cache_polymorphic\": 0"));
        assert!(json.contains("\"inline_cache_megamorphic\": 0"));
        assert!(json.contains("\"inline_cache_disabled\": 0"));
        assert!(json.contains("\"method_ic_hits\": 0"));
        assert!(json.contains("\"method_ic_misses\": 0"));
        assert!(json.contains("\"method_ic_guard_failures\": 0"));
        assert!(json.contains("\"property_ic_hits\": 0"));
        assert!(json.contains("\"property_ic_misses\": 0"));
        assert!(json.contains("\"property_ic_guard_failures\": 0"));
        assert!(json.contains("\"class_static_ic_hits\": 0"));
        assert!(json.contains("\"class_static_ic_misses\": 0"));
        assert!(json.contains("\"class_static_ic_guard_failures\": 0"));
        assert!(json.contains("\"include_path_ic_hits\": 0"));
        assert!(json.contains("\"include_path_ic_misses\": 0"));
        assert!(json.contains("\"include_path_ic_invalidations\": 0"));
        assert!(json.contains("\"include_path_ic_guard_failures\": 0"));
        assert!(json.contains("\"autoload_class_lookup_ic_hits\": 0"));
        assert!(json.contains("\"autoload_class_lookup_ic_misses\": 0"));
        assert!(json.contains("\"autoload_class_lookup_ic_invalidations\": 0"));
        assert!(json.contains("\"autoload_class_lookup_ic_guard_failures\": 0"));
        assert!(json.ends_with('\n'));
    }
}
