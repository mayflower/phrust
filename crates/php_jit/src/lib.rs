//! Default-off Phase 7 JIT API skeleton.
//!
//! This crate intentionally does not allocate or execute native code. The
//! `jit-cranelift` feature is a compile-time experiment flag only until later
//! prompts add eligibility, ABI, and executable-memory safety gates.

mod abi;
#[cfg(feature = "jit-cranelift")]
mod cranelift_lowering;
mod eligibility;

pub use abi::{
    JitAbiValue, JitBailout, JitBailoutKind, JitExceptionMarker, JitFrameHandle, JitFrameView,
    JitOpaqueHandle, JitOpaqueValueKind, JitRegionResult, JitRuntimeCallout,
    JitRuntimeCalloutResult, JitVmContextHandle,
};
#[cfg(feature = "jit-cranelift")]
pub use cranelift_lowering::{
    CraneliftLoweringError, CraneliftLoweringResult, CraneliftLoweringStats,
    CraneliftMachineCodeHandle, lower_function_to_cranelift,
};
pub use eligibility::{
    JitEligibility, JitEligibilityReason, JitEligibilityReport, JitEligibilityStats,
    analyze_jit_eligibility, call_args_are_jit_primitive,
};
use php_ir::{FunctionId, IrUnit};
use std::fmt;

/// Stable backend selected for the current build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JitBackend {
    /// No native backend is compiled in.
    Stub,
    /// The Cranelift experiment feature is compiled, but execution is still off.
    CraneliftExperiment,
}

impl JitBackend {
    /// Returns the backend for this build.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(feature = "jit-cranelift") {
            Self::CraneliftExperiment
        } else {
            Self::Stub
        }
    }

    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stub => "stub",
            Self::CraneliftExperiment => "cranelift-experiment",
        }
    }
}

/// Options for constructing a JIT engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitOptions {
    /// Runtime switch. Defaults off even when a backend feature is compiled.
    pub enabled: bool,
    /// Whether native execution is allowed for this process.
    pub allow_native_execution: bool,
}

impl Default for JitOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_native_execution: false,
        }
    }
}

/// Request to compile one future JIT region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitCompileRequest {
    /// Stable region identifier chosen by the caller.
    pub region_id: String,
    /// Optional PHP function or method name for reports.
    pub function_name: Option<String>,
    /// Optional stable IR fingerprint when available.
    pub ir_fingerprint: Option<String>,
    /// Optimization level active when the request was made.
    pub opt_level: u8,
}

impl JitCompileRequest {
    /// Creates a compile request for a region.
    #[must_use]
    pub fn new(region_id: impl Into<String>) -> Self {
        Self {
            region_id: region_id.into(),
            function_name: None,
            ir_fingerprint: None,
            opt_level: 0,
        }
    }

    /// Adds a function name.
    #[must_use]
    pub fn with_function_name(mut self, function_name: impl Into<String>) -> Self {
        self.function_name = Some(function_name.into());
        self
    }

    /// Adds an IR fingerprint.
    #[must_use]
    pub fn with_ir_fingerprint(mut self, ir_fingerprint: impl Into<String>) -> Self {
        self.ir_fingerprint = Some(ir_fingerprint.into());
        self
    }

    /// Adds the active optimization level.
    #[must_use]
    pub const fn with_opt_level(mut self, opt_level: u8) -> Self {
        self.opt_level = opt_level;
        self
    }
}

/// Opaque handle for a future compiled function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitFunctionHandle {
    /// Stable handle id. It is not an executable pointer.
    pub id: u64,
    /// Region that produced this handle.
    pub region_id: String,
    /// Backend that produced this handle.
    pub backend: JitBackend,
}

/// Machine-readable compile status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JitCompileStatus {
    /// Runtime JIT flag is disabled.
    Disabled,
    /// Backend feature is not compiled in.
    BackendUnavailable,
    /// Backend feature is present, but native execution is blocked.
    NativeExecutionDisabled,
    /// Region was rejected before code generation.
    Rejected { reason: String },
}

impl JitCompileStatus {
    /// Stable report spelling.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::BackendUnavailable => "backend_unavailable",
            Self::NativeExecutionDisabled => "native_execution_disabled",
            Self::Rejected { .. } => "rejected",
        }
    }
}

/// Result of a compile attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitCompileResult {
    /// Compile status.
    pub status: JitCompileStatus,
    /// Future compiled function handle. Always `None` in Prompt 07.49.
    pub handle: Option<JitFunctionHandle>,
    /// Diagnostics suitable for logs and smoke reports.
    pub diagnostics: Vec<String>,
    /// Snapshot of engine stats after the request.
    pub stats: JitStats,
}

/// JIT error type for invalid API use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JitError {
    /// Region id was empty.
    EmptyRegionId,
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRegionId => f.write_str("JIT region id must not be empty"),
        }
    }
}

impl std::error::Error for JitError {}

/// Accumulated JIT counters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JitStats {
    /// Compile requests observed.
    pub compile_requests: u64,
    /// Requests skipped because runtime JIT was disabled.
    pub disabled_requests: u64,
    /// Requests skipped because no backend feature was compiled in.
    pub backend_unavailable: u64,
    /// Requests blocked because native execution is not enabled.
    pub native_execution_disabled: u64,
    /// Regions rejected by the skeleton before code generation.
    pub rejected: u64,
    /// Native compile successes. Always zero in Prompt 07.49.
    pub native_compiles: u64,
    /// Executable memory allocations. Always zero in Prompt 07.49.
    pub executable_memory_allocations: u64,
    /// Eligibility analyses observed.
    pub eligibility_analyses: u64,
    /// Functions accepted by the conservative eligibility analysis.
    pub eligibility_eligible: u64,
    /// Functions rejected by the conservative eligibility analysis.
    pub eligibility_rejected: u64,
    /// Functions the conservative eligibility analysis could not classify.
    pub eligibility_unknown: u64,
    /// Blocks inspected by eligibility analysis.
    pub eligibility_blocks_analyzed: u64,
    /// Instructions inspected by eligibility analysis.
    pub eligibility_instructions_analyzed: u64,
}

/// Default-off JIT engine skeleton.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitEngine {
    options: JitOptions,
    backend: JitBackend,
    stats: JitStats,
}

impl JitEngine {
    /// Creates a default-off engine.
    #[must_use]
    pub fn new() -> Self {
        Self::with_options(JitOptions::default())
    }

    /// Creates an engine with explicit options.
    #[must_use]
    pub fn with_options(options: JitOptions) -> Self {
        Self {
            options,
            backend: JitBackend::current(),
            stats: JitStats::default(),
        }
    }

    /// Returns the selected backend for this build.
    #[must_use]
    pub const fn backend(&self) -> JitBackend {
        self.backend
    }

    /// Returns accumulated stats.
    #[must_use]
    pub const fn stats(&self) -> &JitStats {
        &self.stats
    }

    /// Analyzes one IR function for the future JIT subset.
    ///
    /// This does not compile or execute native code. It only records whether a
    /// function is eligible for later experimental lowering.
    pub fn analyze_eligibility(
        &mut self,
        unit: &IrUnit,
        function: FunctionId,
    ) -> JitEligibilityReport {
        let report = analyze_jit_eligibility(unit, function);
        self.stats.eligibility_analyses += 1;
        self.stats.eligibility_blocks_analyzed += report.stats.blocks_analyzed;
        self.stats.eligibility_instructions_analyzed += report.stats.instructions_analyzed;
        match &report.eligibility {
            JitEligibility::Eligible => self.stats.eligibility_eligible += 1,
            JitEligibility::Rejected { .. } => self.stats.eligibility_rejected += 1,
            JitEligibility::Unknown { .. } => self.stats.eligibility_unknown += 1,
        }
        report
    }

    /// Attempts to compile a region.
    ///
    /// Prompt 07.49 never emits native code. The method only records why the
    /// request is skipped or rejected.
    pub fn compile(&mut self, request: JitCompileRequest) -> Result<JitCompileResult, JitError> {
        if request.region_id.is_empty() {
            return Err(JitError::EmptyRegionId);
        }

        self.stats.compile_requests += 1;
        let status = if !self.options.enabled {
            self.stats.disabled_requests += 1;
            JitCompileStatus::Disabled
        } else if self.backend == JitBackend::Stub {
            self.stats.backend_unavailable += 1;
            JitCompileStatus::BackendUnavailable
        } else if !self.options.allow_native_execution {
            self.stats.native_execution_disabled += 1;
            JitCompileStatus::NativeExecutionDisabled
        } else {
            self.stats.rejected += 1;
            JitCompileStatus::Rejected {
                reason: "jit code generation is not implemented in prompt 07.49".to_owned(),
            }
        };

        let diagnostics = vec![format!(
            "jit region `{}` skipped with status `{}`",
            request.region_id,
            status.as_str()
        )];
        Ok(JitCompileResult {
            status,
            handle: None,
            diagnostics,
            stats: self.stats.clone(),
        })
    }
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{JitBackend, JitCompileRequest, JitCompileStatus, JitEngine, JitError, JitOptions};
    use crate::{
        JitAbiValue, JitBailout, JitBailoutKind, JitEligibility, JitExceptionMarker,
        JitFrameHandle, JitFrameView, JitOpaqueHandle, JitOpaqueValueKind, JitRegionResult,
        JitRuntimeCallout, JitRuntimeCalloutResult, JitVmContextHandle, analyze_jit_eligibility,
    };
    use php_ir::{
        BinaryOp, BlockId, FunctionFlags, FunctionId, InstrId, InstructionKind, IrBuilder,
        IrConstant, IrSpan, LocalId, Operand, RegId, UnitId,
    };

    #[test]
    fn backend_reflects_feature_flag_without_requiring_native_execution() {
        let engine = JitEngine::new();
        if cfg!(feature = "jit-cranelift") {
            assert_eq!(engine.backend(), JitBackend::CraneliftExperiment);
        } else {
            assert_eq!(engine.backend(), JitBackend::Stub);
        }
        assert_eq!(engine.stats().executable_memory_allocations, 0);
    }

    #[test]
    fn default_engine_skips_compile_when_disabled() {
        let mut engine = JitEngine::new();
        let result = engine
            .compile(JitCompileRequest::new("main"))
            .expect("compile request is valid");

        assert_eq!(result.status, JitCompileStatus::Disabled);
        assert!(result.handle.is_none());
        assert_eq!(result.stats.compile_requests, 1);
        assert_eq!(result.stats.disabled_requests, 1);
        assert_eq!(result.stats.native_compiles, 0);
        assert_eq!(result.stats.executable_memory_allocations, 0);
    }

    #[test]
    fn enabled_stub_reports_backend_unavailable_without_native_code() {
        let mut engine = JitEngine::with_options(JitOptions {
            enabled: true,
            allow_native_execution: false,
        });
        let result = engine
            .compile(
                JitCompileRequest::new("loop")
                    .with_function_name("loop")
                    .with_ir_fingerprint("abc123")
                    .with_opt_level(1),
            )
            .expect("compile request is valid");

        if cfg!(feature = "jit-cranelift") {
            assert_eq!(result.status, JitCompileStatus::NativeExecutionDisabled);
            assert_eq!(result.stats.native_execution_disabled, 1);
        } else {
            assert_eq!(result.status, JitCompileStatus::BackendUnavailable);
            assert_eq!(result.stats.backend_unavailable, 1);
        }
        assert!(result.handle.is_none());
        assert_eq!(result.stats.native_compiles, 0);
        assert_eq!(result.stats.executable_memory_allocations, 0);
    }

    #[test]
    fn empty_region_id_is_an_api_error() {
        let mut engine = JitEngine::new();
        let error = engine
            .compile(JitCompileRequest::new(""))
            .expect_err("empty ids are rejected");

        assert_eq!(error, JitError::EmptyRegionId);
        assert_eq!(engine.stats().compile_requests, 0);
    }

    #[test]
    fn eligibility_accepts_primitive_int_bool_leaf_fixture() {
        let (unit, function) = eligible_int_add_fixture();
        let report = analyze_jit_eligibility(&unit, function);

        assert_eq!(report.eligibility, JitEligibility::Eligible);
        assert!(report.reasons.is_empty());
        assert_eq!(report.stats.blocks_analyzed, 1);
        assert_eq!(report.stats.instructions_analyzed, 3);
        assert!(
            report
                .debug_output()
                .contains("jit-eligibility function=eligible_int_add status=eligible")
        );

        let mut engine = JitEngine::new();
        let report = engine.analyze_eligibility(&unit, function);
        assert_eq!(report.eligibility, JitEligibility::Eligible);
        assert_eq!(engine.stats().eligibility_analyses, 1);
        assert_eq!(engine.stats().eligibility_eligible, 1);
        assert_eq!(engine.stats().eligibility_rejected, 0);
        assert_eq!(engine.stats().eligibility_instructions_analyzed, 3);
        assert_eq!(engine.stats().executable_memory_allocations, 0);
    }

    #[test]
    fn eligibility_rejects_calls_arrays_and_nonprimitive_constants() {
        let (unit, function) = rejected_dynamic_fixture();
        let report = analyze_jit_eligibility(&unit, function);

        assert_eq!(report.eligibility.as_str(), "rejected");
        let codes: Vec<_> = report.reasons.iter().map(|reason| reason.code).collect();
        assert!(codes.contains(&"JIT_ELIGIBILITY_REJECT_CALL_OPCODE"));
        assert!(codes.contains(&"JIT_ELIGIBILITY_REJECT_ARRAY_OPCODE"));
        assert!(codes.contains(&"JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_CONSTANT"));
        assert!(report.debug_output().contains("status=rejected"));
    }

    #[test]
    fn eligibility_reports_unknown_for_missing_ir_function() {
        let (unit, _) = eligible_int_add_fixture();
        let report = analyze_jit_eligibility(&unit, FunctionId::new(99));

        assert_eq!(report.eligibility.as_str(), "unknown");
        assert_eq!(report.reasons[0].code, "JIT_ELIGIBILITY_UNKNOWN_FUNCTION");
    }

    #[test]
    fn abi_handles_are_opaque_and_non_zero() {
        assert!(JitOpaqueHandle::new(0).is_none());
        assert!(JitVmContextHandle::new(0).is_none());
        assert!(JitFrameHandle::new(0).is_none());

        let context = JitVmContextHandle::new(1).expect("non-zero context");
        let frame = JitFrameHandle::new(2).expect("non-zero frame");
        let view = JitFrameView::new(context, frame, FunctionId::new(7), 3, 2);

        assert_eq!(view.context.raw(), 1);
        assert_eq!(view.frame.raw(), 2);
        assert!(view.contains_register(RegId::new(2)));
        assert!(!view.contains_register(RegId::new(3)));
        assert!(view.contains_local(LocalId::new(1)));
        assert!(!view.contains_local(LocalId::new(2)));
    }

    #[test]
    fn abi_value_boundary_uses_by_value_or_opaque_values() {
        let string_handle = JitOpaqueHandle::new(44).expect("non-zero handle");
        let value = JitAbiValue::Opaque {
            kind: JitOpaqueValueKind::String,
            handle: string_handle,
        };

        assert!(value.is_opaque());
        assert_eq!(JitOpaqueValueKind::String.as_str(), "string");
        assert_eq!(
            JitAbiValue::float(1.5),
            JitAbiValue::FloatBits(1.5f64.to_bits())
        );

        let callout = JitRuntimeCallout::new("strlen", vec![value], true);
        assert_eq!(callout.name, "strlen");
        assert!(callout.can_throw);
        assert_eq!(
            JitRuntimeCalloutResult::Returned(JitAbiValue::Int(3)),
            JitRuntimeCalloutResult::Returned(JitAbiValue::Int(3))
        );
    }

    #[test]
    fn abi_models_bailout_deopt_and_exception_markers() {
        let bailout = JitBailout::new(JitBailoutKind::GuardFailed, "type guard failed")
            .with_resume(BlockId::new(1), InstrId::new(2));
        assert_eq!(bailout.kind.as_str(), "guard_failed");
        assert_eq!(bailout.resume_block, Some(BlockId::new(1)));
        assert_eq!(bailout.resume_instruction, Some(InstrId::new(2)));

        let exception = JitExceptionMarker::named("TypeError", "bad argument");
        assert_eq!(exception.class_name.as_deref(), Some("TypeError"));

        let opaque_exception =
            JitExceptionMarker::opaque(JitOpaqueHandle::new(99).expect("non-zero handle"));
        assert_eq!(opaque_exception.exception.expect("handle").raw(), 99);

        assert_eq!(
            JitRegionResult::Bailout(bailout.clone()),
            JitRegionResult::Bailout(bailout)
        );
        assert_eq!(
            JitRuntimeCalloutResult::Exception(exception.clone()),
            JitRuntimeCalloutResult::Exception(exception)
        );
    }

    fn eligible_int_add_fixture() -> (php_ir::IrUnit, FunctionId) {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("tests/fixtures/phase7/jit/eligible-int-add.php");
        let span = IrSpan::new(file, 0, 0);
        let function = builder.start_function("eligible_int_add", FunctionFlags::default(), span);
        builder.set_entry(function);
        let block = builder.append_block(function);
        let one = builder.add_constant(IrConstant::Int(1));
        let two = builder.add_constant(IrConstant::Int(2));
        let r0 = builder.alloc_register(function);
        let r1 = builder.alloc_register(function);
        let r2 = builder.alloc_register(function);
        builder.emit_load_const(function, block, r0, one, span);
        builder.emit_load_const(function, block, r1, two, span);
        builder.emit(
            function,
            block,
            InstructionKind::Binary {
                dst: r2,
                op: BinaryOp::Add,
                lhs: Operand::Register(r0),
                rhs: Operand::Register(r1),
            },
            span,
        );
        builder.terminate_return(function, block, Some(Operand::Register(r2)), span);
        (builder.finish(), function)
    }

    fn rejected_dynamic_fixture() -> (php_ir::IrUnit, FunctionId) {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("tests/fixtures/phase7/jit/rejected-dynamic.php");
        let span = IrSpan::new(file, 0, 0);
        let function = builder.start_function("rejected_dynamic", FunctionFlags::default(), span);
        builder.set_entry(function);
        let block = builder.append_block(function);
        let text = builder.add_constant(IrConstant::String("not primitive".to_owned()));
        let r0 = builder.alloc_register(function);
        let r1 = builder.alloc_register(function);
        builder.emit_load_const(function, block, r0, text, span);
        builder.emit(
            function,
            block,
            InstructionKind::CallFunction {
                dst: r1,
                name: "strlen".to_owned(),
                args: Vec::new(),
            },
            span,
        );
        builder.emit(function, block, InstructionKind::NewArray { dst: r0 }, span);
        builder.terminate_return(function, block, Some(Operand::Register(r1)), span);
        (builder.finish(), function)
    }
}
