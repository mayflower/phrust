//! Native PHP execution coordinator.

mod jit_abi;
mod options;
mod result;

pub use options::{NativeBlacklistMode, NativeOptimizationPolicy, VmOptions};
pub use result::VmResult;

use crate::compiled_unit::CompiledUnit;
use jit_abi::{
    jit_array_fetch_int_slow_abi, jit_array_len_abi, jit_concat_string_string_fast,
    jit_count_known_abi, jit_native_call_dispatch_abi, jit_native_dynamic_code_abi,
    jit_property_load_monomorphic_fast, jit_record_array_lookup_abi, jit_runtime_helper_table,
    jit_strlen_known_abi,
};
use php_runtime::api::{OutputBuffer, Value};
use std::time::{Duration, Instant};

/// Process-owned state shared by native request coordinators.
#[derive(Clone, Debug, Default)]
pub struct VmWorkerState;

impl VmWorkerState {
    #[must_use]
    pub fn new(_tiering: crate::tiering::TieringOptions) -> Self {
        Self
    }
}

/// Coordinates mandatory native compilation and outer result assembly.
pub struct Vm {
    options: VmOptions,
    _worker_state: VmWorkerState,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(VmOptions::default())
    }

    #[must_use]
    pub fn with_options(options: VmOptions) -> Self {
        let worker_state = VmWorkerState::new(options.tiering.clone());
        Self::with_options_and_worker_state(options, worker_state)
    }

    #[must_use]
    pub fn with_options_and_worker_state(options: VmOptions, worker_state: VmWorkerState) -> Self {
        Self {
            options,
            _worker_state: worker_state,
        }
    }

    /// Compile and publish native entries without entering application code.
    #[must_use]
    pub fn prewarm_cranelift(&self, unit: &CompiledUnit) -> u64 {
        let entry = unit.unit().entry;
        let Some(function) = unit.unit().functions.get(entry.index()) else {
            return 0;
        };
        let mut compiler = php_jit::JitEngine::new();
        compiler
            .compile_unit_with_runtime_helpers(
                unit.unit(),
                php_jit::JitCompileRequest::new(format!("unit.{}", unit.unit().id.raw()))
                    .with_function_name(function.name.clone())
                    .with_opt_level(if self.options.native_optimization.is_optimizing() {
                        2
                    } else {
                        0
                    }),
                runtime_helper_addresses(),
            )
            .map_or(0, |records| {
                records
                    .iter()
                    .filter(|record| {
                        matches!(record.result.status, php_jit::JitCompileStatus::Compiled)
                    })
                    .count() as u64
            })
    }

    /// Compile every function from authoritative IR and enter the published
    /// Cranelift entry. There is no alternate execution engine.
    #[must_use]
    pub fn execute(&self, unit: impl Into<CompiledUnit>) -> VmResult {
        let unit = unit.into();
        let output = OutputBuffer::default();
        let entry = unit.unit().entry;
        let Some(function) = unit.unit().functions.get(entry.index()) else {
            return VmResult::compile_error(output, "entry function is missing");
        };
        if self.options.verify_ir && unit.prepared_ir_verification_errors() > 0 {
            return VmResult::compile_error(
                output,
                format!(
                    "IR verifier failed with {} error(s)",
                    unit.prepared_ir_verification_errors()
                ),
            );
        }

        let mut cache_load_time = Duration::ZERO;
        let mut native_compile_time = Duration::ZERO;
        let cache = match self.native_cache() {
            Ok(cache) => cache,
            Err(error) => {
                return VmResult::compile_error(output, format!("E_NATIVE_CACHE_SETUP: {error}"));
            }
        };
        let cache_identity = cache
            .as_ref()
            .and_then(|_| native_cache_identity(&unit, &self.options).ok());
        let mut cached_compile_records = None;
        let mut cached_compile_error = None;

        if let (Some(cache), Some(identity)) = (&cache, &cache_identity) {
            if cache.config().mode.can_write() && native_cache_candidate(unit.unit(), entry) {
                let cache_started = Instant::now();
                let result = cache.get_or_compile(
                    identity,
                    |_| None,
                    || {
                        let compile_started = Instant::now();
                        let records = match self.compile_native(&unit, function) {
                            Ok(records) => records,
                            Err(error) => {
                                native_compile_time += compile_started.elapsed();
                                cached_compile_error = Some(error.clone());
                                return Err(php_jit::NativeCacheError::InvalidHeader(error));
                            }
                        };
                        native_compile_time += compile_started.elapsed();
                        let image = cache_image(identity.clone(), entry, &records);
                        cached_compile_records = Some(records);
                        image
                    },
                );
                cache_load_time += cache_started.elapsed().saturating_sub(native_compile_time);
                if let Ok((artifact, _)) = result {
                    let result = match artifact.invoke_i64_status_out(entry.raw()) {
                        Ok(value) => VmResult::success(output, Some(Value::Int(value))),
                        Err(error) => VmResult::compile_error(
                            output,
                            format!(
                                "E_NATIVE_CACHE_ENTRY: cached entry invocation failed: {error}"
                            ),
                        ),
                    };
                    return self.attach_native_cache_metrics(
                        result,
                        cache,
                        cache_load_time,
                        native_compile_time,
                    );
                }
                if let Some(error) = cached_compile_error {
                    let result =
                        VmResult::compile_error(output, format!("E_NATIVE_COMPILE_SETUP: {error}"));
                    return self.attach_native_cache_metrics(
                        result,
                        cache,
                        cache_load_time,
                        native_compile_time,
                    );
                }
            } else if cache.config().mode.can_read() {
                let cache_started = Instant::now();
                let loaded = cache.load(identity, |_| None);
                cache_load_time += cache_started.elapsed();
                if let Ok(Some(artifact)) = loaded {
                    let result = match artifact.invoke_i64_status_out(entry.raw()) {
                        Ok(value) => VmResult::success(output, Some(Value::Int(value))),
                        Err(error) => VmResult::compile_error(
                            output,
                            format!(
                                "E_NATIVE_CACHE_ENTRY: cached entry invocation failed: {error}"
                            ),
                        ),
                    };
                    return self.attach_native_cache_metrics(
                        result,
                        cache,
                        cache_load_time,
                        native_compile_time,
                    );
                }
            }
        }

        let compile_started = Instant::now();
        let records = match cached_compile_records {
            Some(records) => records,
            None => match self.compile_native(&unit, function) {
                Ok(records) => records,
                Err(error) => {
                    native_compile_time += compile_started.elapsed();
                    let result =
                        VmResult::compile_error(output, format!("E_NATIVE_COMPILE_SETUP: {error}"));
                    return self.attach_optional_native_cache_metrics(
                        result,
                        cache.as_ref(),
                        cache_load_time,
                        native_compile_time,
                    );
                }
            },
        };
        if native_compile_time.is_zero() {
            native_compile_time += compile_started.elapsed();
        }
        let Some(entry_record) = records.iter().find(|record| record.function == entry) else {
            let result =
                VmResult::compile_error(output, "E_NATIVE_COMPILE_SETUP: entry record missing");
            return self.attach_optional_native_cache_metrics(
                result,
                cache.as_ref(),
                cache_load_time,
                native_compile_time,
            );
        };
        if let Some(rejected) = records
            .iter()
            .find(|record| !matches!(&record.result.status, php_jit::JitCompileStatus::Compiled))
        {
            let name = unit
                .unit()
                .functions
                .get(rejected.function.index())
                .map_or("<missing>", |function| function.name.as_str());
            let reason = match &rejected.result.status {
                php_jit::JitCompileStatus::Rejected { reason } => reason.as_str(),
                php_jit::JitCompileStatus::Compiled => "compiler reported no native code",
            };
            let detail = rejected
                .result
                .diagnostics
                .first()
                .map_or("", String::as_str);
            let result = VmResult::compile_error(
                output,
                format!("E_NATIVE_UNSUPPORTED_LOWERING: function={name}: {reason}: {detail}"),
            );
            return self.attach_optional_native_cache_metrics(
                result,
                cache.as_ref(),
                cache_load_time,
                native_compile_time,
            );
        }
        let compiled = &entry_record.result;
        let Some(handle) = compiled.handle.as_ref() else {
            let reason = match &compiled.status {
                php_jit::JitCompileStatus::Rejected { reason } => reason.clone(),
                php_jit::JitCompileStatus::Compiled => {
                    "compiler reported success without a native entry".to_owned()
                }
            };
            let result = VmResult::compile_error(output, format!("E_NATIVE_COMPILE: {reason}"));
            return self.attach_optional_native_cache_metrics(
                result,
                cache.as_ref(),
                cache_load_time,
                native_compile_time,
            );
        };
        let result = match handle.invoke_i64(&[], php_jit::JIT_RUNTIME_ABI_HASH) {
            Ok(value) => VmResult::success(output, Some(Value::Int(value))),
            Err(error) => VmResult::compile_error(
                output,
                format!("E_NATIVE_ENTRY: native entry invocation failed: {error:?}"),
            ),
        };
        self.attach_optional_native_cache_metrics(
            result,
            cache.as_ref(),
            cache_load_time,
            native_compile_time,
        )
    }

    fn compile_native(
        &self,
        unit: &CompiledUnit,
        function: &php_ir::IrFunction,
    ) -> Result<Vec<php_jit::JitUnitCompileRecord>, String> {
        let mut compiler = php_jit::JitEngine::new();
        compiler
            .compile_unit_with_runtime_helpers(
                unit.unit(),
                php_jit::JitCompileRequest::new(format!("unit.{}", unit.unit().id.raw()))
                    .with_function_name(function.name.clone())
                    .with_opt_level(if self.options.native_optimization.is_optimizing() {
                        2
                    } else {
                        0
                    }),
                runtime_helper_addresses(),
            )
            .map_err(|error| error.to_string())
    }

    fn native_cache(
        &self,
    ) -> Result<Option<php_jit::NativeArtifactCache>, php_jit::NativeCacheError> {
        if self.options.native_cache == php_jit::NativeCacheMode::Off {
            return Ok(None);
        }
        php_jit::NativeArtifactCache::new(php_jit::NativeCacheConfig {
            mode: self.options.native_cache,
            directory: self.options.native_cache_dir.clone(),
            ..php_jit::NativeCacheConfig::default()
        })
        .map(Some)
    }

    fn attach_optional_native_cache_metrics(
        &self,
        result: VmResult,
        cache: Option<&php_jit::NativeArtifactCache>,
        cache_load_time: Duration,
        native_compile_time: Duration,
    ) -> VmResult {
        self.attach_native_metrics(
            result,
            cache.map(php_jit::NativeArtifactCache::stats),
            cache_load_time,
            native_compile_time,
        )
    }

    fn attach_native_cache_metrics(
        &self,
        result: VmResult,
        cache: &php_jit::NativeArtifactCache,
        cache_load_time: Duration,
        native_compile_time: Duration,
    ) -> VmResult {
        self.attach_native_metrics(
            result,
            Some(cache.stats()),
            cache_load_time,
            native_compile_time,
        )
    }

    fn attach_native_metrics(
        &self,
        mut result: VmResult,
        cache_stats: Option<php_jit::NativeCacheStats>,
        cache_load_time: Duration,
        native_compile_time: Duration,
    ) -> VmResult {
        result.native_cache_load_nanos =
            cache_load_time.as_nanos().min(u128::from(u64::MAX)) as u64;
        result.native_compile_nanos =
            native_compile_time.as_nanos().min(u128::from(u64::MAX)) as u64;
        if self.options.native_cache_stats
            && let Some(stats) = cache_stats
        {
            result.native_cache_stats = Some(Box::new(stats));
        }
        if self.options.collect_counters {
            let mut counters = crate::counters::VmCounters::default();
            let compiled = !native_compile_time.is_zero();
            let executed = result.status.is_success();
            counters.native_compile_attempts = u64::from(compiled);
            counters.native_compile_successes = u64::from(compiled && executed);
            counters.native_compile_failures = u64::from(compiled && !executed);
            counters.native_compile_time_nanos = result.native_compile_nanos;
            counters.native_execution_entries = u64::from(executed);
            counters.native_region_entries = u64::from(executed);
            counters.native_version_published = u64::from(compiled && executed);
            if let Some(stats) = cache_stats {
                counters.native_cache_hits = stats.hits;
                counters.native_cache_misses = stats.misses;
                counters.native_cache_writes = stats.writes;
                counters.native_cache_rebuilds = stats.rebuilds;
                counters.native_cache_invalid_artifacts = stats.invalid_artifacts;
                counters.native_cache_compile_waits = stats.compile_waits;
                counters.native_cache_bytes_loaded = stats.bytes_loaded;
                counters.native_cache_bytes_written = stats.bytes_written;
            }
            result.counters = Some(Box::new(counters));
        }
        result
    }
}

fn native_cache_candidate(unit: &php_ir::IrUnit, entry: php_ir::FunctionId) -> bool {
    if unit.functions.len() != 1 {
        return false;
    }
    let Some(function) = unit.functions.get(entry.index()) else {
        return false;
    };
    function.params.is_empty()
        && function.blocks.iter().all(|block| {
            block.instructions.iter().all(|instruction| {
                matches!(
                    instruction.kind,
                    php_ir::InstructionKind::Nop
                        | php_ir::InstructionKind::LoadConst { .. }
                        | php_ir::InstructionKind::Move { .. }
                        | php_ir::InstructionKind::LoadLocal { .. }
                        | php_ir::InstructionKind::LoadLocalQuiet { .. }
                        | php_ir::InstructionKind::StoreLocal { .. }
                )
            }) && block.terminator.as_ref().is_some_and(|terminator| {
                matches!(
                    terminator.kind,
                    php_ir::instruction::TerminatorKind::Jump { .. }
                        | php_ir::instruction::TerminatorKind::Return { .. }
                )
            })
        })
}

fn native_cache_identity(
    unit: &CompiledUnit,
    options: &VmOptions,
) -> Result<php_jit::NativeCacheIdentity, php_jit::CraneliftHostIsaError> {
    let isa = php_jit::cranelift_host_isa_identity()?;
    let optimization_tier = options.native_optimization.as_str().to_owned();
    Ok(php_jit::NativeCacheIdentity {
        source_hash: format!("compiled-source-v1-{:016x}", unit.artifact_identity()),
        ir_hash: php_jit::stable_ir_fingerprint(unit.unit()),
        dependency_graph_hash: php_jit::stable_dependency_identity(unit.unit()),
        build_id: option_env!("PHRUST_BUILD_ID")
            .unwrap_or(env!("PHRUST_AUTO_BUILD_ID"))
            .to_owned(),
        cranelift_version: php_jit::CRANELIFT_VERSION.to_owned(),
        cranelift_settings_hash: isa.feature_fingerprint,
        region_ir_schema_version: php_jit::region_ir::REGION_IR_SCHEMA_VERSION,
        runtime_abi_hash: php_jit::JIT_RUNTIME_ABI_HASH
            ^ php_runtime::api::NATIVE_OPERATION_ABI_HASH,
        helper_abi_hash: php_jit::JIT_HELPER_REGISTRY_ABI_HASH,
        target_triple: isa.target_triple,
        pointer_width: usize::BITS as u8,
        cpu_feature_fingerprint: isa.feature_fingerprint,
        optimization_tier,
        optimization_config_hash: u64::from(options.native_optimization.is_optimizing()),
        php_semantic_config_hash: 0x0008_0005_0007,
    })
}

fn cache_image(
    identity: php_jit::NativeCacheIdentity,
    entry: php_ir::FunctionId,
    records: &[php_jit::JitUnitCompileRecord],
) -> Result<php_jit::NativeArtifactImage, php_jit::NativeCacheError> {
    let record = records
        .iter()
        .find(|record| record.function == entry)
        .ok_or_else(|| {
            php_jit::NativeCacheError::InvalidHeader("entry record missing".to_owned())
        })?;
    let handle = record.result.handle.as_ref().ok_or_else(|| {
        php_jit::NativeCacheError::InvalidHeader("entry has no native handle".to_owned())
    })?;
    let code = handle.copy_relocation_free_machine_code().ok_or_else(|| {
        php_jit::NativeCacheError::InvalidRelocation(
            "entry requires relocation-aware cache emission".to_owned(),
        )
    })?;
    Ok(php_jit::NativeArtifactImage::minimal(
        identity,
        code.clone(),
        php_jit::NativeFunctionImage {
            function_id: entry.raw(),
            code_offset: 0,
            code_len: code.len() as u64,
            arity: 0,
            abi: php_jit::NativeFunctionAbi::I64StatusOut,
        },
    ))
}

fn runtime_helper_addresses() -> php_jit::JitRuntimeHelperAddresses {
    php_jit::JitRuntimeHelperAddresses {
        helper_table: jit_runtime_helper_table() as *const _ as usize,
        packed_array_len: jit_array_len_abi as *const () as usize,
        packed_array_fetch_int_slow: jit_array_fetch_int_slow_abi as *const () as usize,
        known_strlen: jit_strlen_known_abi as *const () as usize,
        known_count: jit_count_known_abi as *const () as usize,
        string_concat: jit_concat_string_string_fast as *const () as usize,
        property_load: jit_property_load_monomorphic_fast as *const () as usize,
        record_array_lookup: jit_record_array_lookup_abi as *const () as usize,
        native_call_dispatch: jit_native_call_dispatch_abi as *const () as usize,
        native_dynamic_code: jit_native_dynamic_code_abi as *const () as usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_ir::builder::IrBuilder;
    use php_ir::{
        FunctionFlags, InstructionKind, IrConstant, IrReturnType, IrSpan, Operand, UnitId,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn returning_unit(value: i64) -> CompiledUnit {
        let mut builder = IrBuilder::new(UnitId::new(991));
        let file = builder.add_file("native-cache-vm.php");
        let span = IrSpan::new(file, 0, 20);
        let constant = builder.intern_constant(IrConstant::Int(value));
        let function = builder.start_function("main", FunctionFlags::default(), span);
        builder.set_return_type(function, Some(IrReturnType::Int));
        let block = builder.append_block(function);
        let register = builder.alloc_register(function);
        builder.emit(
            function,
            block,
            InstructionKind::LoadConst {
                dst: register,
                constant,
            },
            span,
        );
        builder.terminate_return(function, block, Some(Operand::Register(register)), span);
        builder.set_entry(function);
        CompiledUnit::new(builder.finish())
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn vm_reloads_native_artifact_without_compilation() {
        let directory = std::env::temp_dir().join(format!(
            "phrust-vm-pna-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let unit = returning_unit(42);
        let first = Vm::with_options(VmOptions {
            native_cache: php_jit::NativeCacheMode::ReadWrite,
            native_cache_dir: directory.clone(),
            native_cache_stats: true,
            ..VmOptions::default()
        })
        .execute(unit.clone());
        assert_eq!(first.return_value, Some(Value::Int(42)));
        assert_eq!(first.native_cache_stats.unwrap().writes, 1);

        let second = Vm::with_options(VmOptions {
            native_cache: php_jit::NativeCacheMode::Read,
            native_cache_dir: directory.clone(),
            native_cache_stats: true,
            ..VmOptions::default()
        })
        .execute(unit);
        assert_eq!(second.return_value, Some(Value::Int(42)));
        assert_eq!(second.native_cache_stats.unwrap().hits, 1);
        assert_eq!(second.native_compile_nanos, 0);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
