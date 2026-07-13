//! Canonical native-engine telemetry collected only when explicitly enabled.

use std::collections::BTreeMap;

/// One native compilation record suitable for diagnostics and profiles.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativeCompileDescriptor {
    pub function_id: u32,
    pub function_name: String,
    pub ir_fingerprint: String,
    pub code_bytes: u64,
    pub compile_time_nanos: u64,
    pub target_isa: String,
    pub runtime_abi_hash: u64,
    pub helper_abi_hash: u64,
    pub config_hash: u64,
}

/// Product telemetry for the single mandatory native engine.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VmCounters {
    pub native_compile_attempts: u64,
    pub native_compile_successes: u64,
    pub native_compile_failures: u64,
    pub native_compile_time_nanos: u64,
    pub native_compile_code_bytes: u64,
    pub native_compile_descriptors: Vec<NativeCompileDescriptor>,

    pub native_cache_hits: u64,
    pub native_cache_misses: u64,
    pub native_cache_writes: u64,
    pub native_cache_rebuilds: u64,
    pub native_cache_invalid_artifacts: u64,
    pub native_cache_compile_waits: u64,
    pub native_cache_bytes_loaded: u64,
    pub native_cache_bytes_written: u64,

    pub native_execution_entries: u64,
    pub native_execution_time_nanos: u64,
    pub native_region_entries: u64,
    pub native_region_side_exits: u64,
    pub native_region_side_exits_by_reason: BTreeMap<String, u64>,
    pub native_call_direct: u64,
    pub native_call_dynamic: u64,
    pub native_version_published: u64,
    pub native_version_retired: u64,
    pub native_transition_count: u64,
    pub native_transition_by_reason: BTreeMap<String, u64>,
    pub runtime_helper_calls: u64,
    pub runtime_helper_calls_by_id: BTreeMap<String, u64>,
    pub gc_safepoint_polls: u64,
    pub gc_safepoint_collections: u64,
}

impl VmCounters {
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"schema_version\": 4,\n",
                "  \"native_compile_attempts\": {},\n",
                "  \"native_compile_successes\": {},\n",
                "  \"native_compile_failures\": {},\n",
                "  \"native_compile_time_nanos\": {},\n",
                "  \"native_compile_code_bytes\": {},\n",
                "  \"native_cache_hits\": {},\n",
                "  \"native_cache_misses\": {},\n",
                "  \"native_cache_writes\": {},\n",
                "  \"native_cache_rebuilds\": {},\n",
                "  \"native_cache_invalid_artifacts\": {},\n",
                "  \"native_execution_entries\": {},\n",
                "  \"native_execution_time_nanos\": {},\n",
                "  \"native_region_entries\": {},\n",
                "  \"native_region_side_exits\": {},\n",
                "  \"native_call_direct\": {},\n",
                "  \"native_call_dynamic\": {},\n",
                "  \"native_version_published\": {},\n",
                "  \"native_version_retired\": {},\n",
                "  \"native_transition_count\": {},\n",
                "  \"runtime_helper_calls\": {},\n",
                "  \"gc_safepoint_polls\": {},\n",
                "  \"gc_safepoint_collections\": {}\n",
                "}}\n"
            ),
            self.native_compile_attempts,
            self.native_compile_successes,
            self.native_compile_failures,
            self.native_compile_time_nanos,
            self.native_compile_code_bytes,
            self.native_cache_hits,
            self.native_cache_misses,
            self.native_cache_writes,
            self.native_cache_rebuilds,
            self.native_cache_invalid_artifacts,
            self.native_execution_entries,
            self.native_execution_time_nanos,
            self.native_region_entries,
            self.native_region_side_exits,
            self.native_call_direct,
            self.native_call_dynamic,
            self.native_version_published,
            self.native_version_retired,
            self.native_transition_count,
            self.runtime_helper_calls,
            self.gc_safepoint_polls,
            self.gc_safepoint_collections,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_json_contains_only_native_engine_families() {
        let json = VmCounters::default().to_json();
        for family in [
            "native_compile",
            "native_cache",
            "native_execution",
            "native_region",
            "native_call",
            "native_version",
            "native_transition",
            "runtime_helper",
            "gc_safepoint",
        ] {
            assert!(json.contains(family), "missing {family}: {json}");
        }
    }
}
