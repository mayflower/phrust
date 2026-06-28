//! VM-owned deoptimization and live-state metadata.
//!
//! This module builds report-only metadata from verified dense bytecode. It is
//! intentionally independent of executable native code so future Cranelift,
//! baseline-native, or quickening tiers can consume the same resume contract.

use php_ir::instruction::{InstructionKind, IrCallArg, TerminatorKind};
use php_ir::{IrSpan, IrUnit};

use crate::bytecode::{
    DENSE_BYTECODE_VERSION, DenseBlock, DenseBytecodeUnit, DenseFunction, DenseInstruction,
    DenseLowerError, DenseOpcode,
};

/// Stable VM-owned deoptimization reason codes.
///
/// Codes 1 through 7 intentionally match the existing Cranelift side-exit
/// reason codes so current counter reports remain compatible.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VmDeoptReason {
    /// Runtime value type did not match the optimized specialization.
    TypeMismatch = 1,
    /// Checked arithmetic or conversion overflowed.
    Overflow = 2,
    /// Runtime value shape is outside the optimized subset.
    UnsupportedValue = 3,
    /// A generated guard failed.
    GuardFailed = 4,
    /// Runtime helper returned a non-OK status.
    HelperStatus = 5,
    /// PHP exception/error state is pending.
    ExceptionPending = 6,
    /// VM/native ABI hash or call boundary did not match.
    AbiMismatch = 7,
    /// Userland or builtin call frame state must be materialized first.
    CallFrameBoundary = 8,
    /// Reference/COW identity is not represented precisely enough.
    ReferenceCowIdentity = 9,
    /// Foreach iterator state must be materialized.
    ForeachIteratorState = 10,
    /// Pending finally/unwind state must be preserved.
    PendingFinally = 11,
    /// Generator or fiber suspension state must be preserved.
    GeneratorOrFiberState = 12,
    /// Output buffering/conversion state must stay interpreter-owned.
    OutputBufferState = 13,
    /// Control-flow shape is outside the metadata generator subset.
    UnsupportedControlFlow = 14,
}

impl VmDeoptReason {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeMismatch => "type_mismatch",
            Self::Overflow => "overflow",
            Self::UnsupportedValue => "unsupported_value",
            Self::GuardFailed => "guard_failed",
            Self::HelperStatus => "helper_status",
            Self::ExceptionPending => "exception_pending",
            Self::AbiMismatch => "abi_mismatch",
            Self::CallFrameBoundary => "call_frame_boundary",
            Self::ReferenceCowIdentity => "reference_cow_identity",
            Self::ForeachIteratorState => "foreach_iterator_state",
            Self::PendingFinally => "pending_finally",
            Self::GeneratorOrFiberState => "generator_or_fiber_state",
            Self::OutputBufferState => "output_buffer_state",
            Self::UnsupportedControlFlow => "unsupported_control_flow",
        }
    }

    /// Stable numeric report/ABI code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// One interpreter resume target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeoptResumePoint {
    /// Dense function index.
    pub function: u32,
    /// Dense block index.
    pub block: u32,
    /// Dense instruction index inside the function instruction array.
    pub instruction: u32,
}

/// Value class in a live-state snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveValueClass {
    /// VM register file slot.
    Register,
    /// PHP local variable slot.
    Local,
    /// Operand stack slot, reserved for future stack-based regions.
    OperandStack,
}

/// Whether a live value can carry reference/COW identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveIdentityMarker {
    /// The value is plain or identity is irrelevant for this snapshot.
    Plain,
    /// The value may be a reference cell or COW-backed container.
    MaybeReferenceOrCow,
}

/// One value slot recorded in a live-state snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveValueSlot {
    /// Value storage class.
    pub class: LiveValueClass,
    /// Zero-based index in that storage class.
    pub index: u32,
    /// Whether the value is definitely initialized at this point.
    pub initialized: Option<bool>,
    /// Reference/COW identity marker.
    pub identity: LiveIdentityMarker,
}

/// How a snapshot represents PHP control-flow state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlStateMarker {
    /// State is known absent for the generated dense region.
    None,
    /// State is present and explicitly represented in metadata.
    Represented,
    /// State exists but this metadata generator rejects the region.
    Rejected,
}

/// VM-owned live-state snapshot for optimized side exits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveStateSnapshot {
    /// Resume location and current bytecode location.
    pub resume: DeoptResumePoint,
    /// Source span for diagnostics and traces.
    pub span: IrSpan,
    /// Register file slots to materialize.
    pub registers: Vec<LiveValueSlot>,
    /// PHP local slots to materialize.
    pub locals: Vec<LiveValueSlot>,
    /// Operand stack slots, empty for the current register VM.
    pub operand_stack: Vec<LiveValueSlot>,
    /// Pending exception marker.
    pub pending_exception: ControlStateMarker,
    /// Pending finally/unwind marker.
    pub pending_finally: ControlStateMarker,
    /// Foreach iterator state marker.
    pub foreach_iterator: ControlStateMarker,
    /// Reference/COW state marker.
    pub reference_cow: ControlStateMarker,
    /// Output-buffer state marker.
    pub output_buffer: ControlStateMarker,
    /// Call frame identity marker.
    pub call_frame_identity: ControlStateMarker,
}

/// One side-exit point from an optimized region back to the interpreter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeoptSideExitPoint {
    /// Stable reason.
    pub reason: VmDeoptReason,
    /// Interpreter resume location.
    pub resume: DeoptResumePoint,
    /// Live values and control markers at the exit.
    pub snapshot: LiveStateSnapshot,
}

/// One metadata region. FPE-16 uses dense basic blocks as conservative regions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeoptRegionMetadata {
    /// Stable region label.
    pub region_id: String,
    /// Dense function index.
    pub function: u32,
    /// Entry dense block index.
    pub entry_block: u32,
    /// Dense blocks covered by this region.
    pub blocks: Vec<u32>,
    /// Dense instruction indexes covered by this region.
    pub instructions: Vec<u32>,
    /// Side exits that can resume in the interpreter.
    pub side_exits: Vec<DeoptSideExitPoint>,
}

/// Report-only VM-owned deopt metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeoptMetadata {
    /// Metadata schema version.
    pub schema_version: u32,
    /// Dense bytecode version consumed by this metadata.
    pub dense_bytecode_version: u32,
    /// This foundation never enables native execution.
    pub native_execution: bool,
    /// Generated regions.
    pub regions: Vec<DeoptRegionMetadata>,
}

impl DeoptMetadata {
    /// Generate metadata from rich IR by first rejecting unsupported VM state,
    /// then lowering to verified dense bytecode.
    pub fn generate_from_ir(unit: &IrUnit) -> Result<Self, Vec<DeoptMetadataError>> {
        let rejections = collect_ir_rejections(unit);
        if !rejections.is_empty() {
            return Err(rejections);
        }
        let dense = DenseBytecodeUnit::lower_from_ir(unit)
            .map_err(|error| vec![DeoptMetadataError::from_dense_lower_error(error)])?;
        Self::generate_from_dense(&dense)
    }

    /// Generate metadata from an already lowered dense bytecode unit.
    pub fn generate_from_dense(unit: &DenseBytecodeUnit) -> Result<Self, Vec<DeoptMetadataError>> {
        if let Err(errors) = unit.verify() {
            return Err(errors
                .into_iter()
                .map(|error| DeoptMetadataError {
                    reason: VmDeoptReason::UnsupportedControlFlow,
                    message: format!("dense bytecode verification failed: {}", error.message),
                })
                .collect());
        }

        let mut regions = Vec::new();
        for (function_index, function) in unit.functions.iter().enumerate() {
            for block in &function.blocks {
                regions.push(region_for_block(
                    unit,
                    function_index as u32,
                    function,
                    block,
                ));
            }
        }

        let metadata = Self {
            schema_version: 1,
            dense_bytecode_version: DENSE_BYTECODE_VERSION,
            native_execution: false,
            regions,
        };
        metadata.verify()?;
        Ok(metadata)
    }

    /// Verify metadata consistency against its own resume and live-state
    /// contract. Dense bytecode verification remains the source of bytecode
    /// structural truth.
    pub fn verify(&self) -> Result<(), Vec<DeoptMetadataError>> {
        let mut errors = Vec::new();
        if self.dense_bytecode_version != DENSE_BYTECODE_VERSION {
            errors.push(DeoptMetadataError {
                reason: VmDeoptReason::AbiMismatch,
                message: format!(
                    "metadata dense bytecode version {} does not match {}",
                    self.dense_bytecode_version, DENSE_BYTECODE_VERSION
                ),
            });
        }
        if self.native_execution {
            errors.push(DeoptMetadataError {
                reason: VmDeoptReason::UnsupportedControlFlow,
                message: "FPE-16 metadata must not enable native execution".to_string(),
            });
        }
        for region in &self.regions {
            if region.blocks.is_empty() || region.instructions.is_empty() {
                errors.push(DeoptMetadataError {
                    reason: VmDeoptReason::UnsupportedControlFlow,
                    message: format!("region {} is empty", region.region_id),
                });
            }
            for exit in &region.side_exits {
                if exit.resume.function != region.function {
                    errors.push(DeoptMetadataError {
                        reason: VmDeoptReason::UnsupportedControlFlow,
                        message: format!("region {} has cross-function resume", region.region_id),
                    });
                }
                if !region.blocks.contains(&exit.resume.block) {
                    errors.push(DeoptMetadataError {
                        reason: VmDeoptReason::UnsupportedControlFlow,
                        message: format!(
                            "region {} resume block {} is outside region blocks",
                            region.region_id, exit.resume.block
                        ),
                    });
                }
                if !region.instructions.contains(&exit.resume.instruction) {
                    errors.push(DeoptMetadataError {
                        reason: VmDeoptReason::UnsupportedControlFlow,
                        message: format!(
                            "region {} resume instruction {} is outside region instructions",
                            region.region_id, exit.resume.instruction
                        ),
                    });
                }
                verify_snapshot(region, &exit.snapshot, &mut errors);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Metadata generation/rejection error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeoptMetadataError {
    /// Stable reason.
    pub reason: VmDeoptReason,
    /// Human-readable detail.
    pub message: String,
}

impl DeoptMetadataError {
    fn from_dense_lower_error(error: DenseLowerError) -> Self {
        Self {
            reason: VmDeoptReason::UnsupportedControlFlow,
            message: format!("dense lowering rejected metadata region: {}", error.message),
        }
    }
}

fn region_for_block(
    unit: &DenseBytecodeUnit,
    function_index: u32,
    function: &DenseFunction,
    block: &DenseBlock,
) -> DeoptRegionMetadata {
    let first = block.first_instruction as usize;
    let end = first + block.instruction_len as usize;
    let instructions: Vec<u32> = (first as u32..end as u32).collect();
    let mut side_exits = Vec::new();
    for instruction_index in first..end {
        let instruction = &function.instructions[instruction_index];
        for reason in reasons_for_instruction(instruction) {
            let resume = DeoptResumePoint {
                function: function_index,
                block: block.id,
                instruction: instruction_index as u32,
            };
            side_exits.push(DeoptSideExitPoint {
                reason,
                resume,
                snapshot: snapshot_for_instruction(unit, function, instruction, resume, reason),
            });
        }
    }
    DeoptRegionMetadata {
        region_id: format!("f{function_index}:b{}", block.id),
        function: function_index,
        entry_block: block.id,
        blocks: vec![block.id],
        instructions,
        side_exits,
    }
}

fn snapshot_for_instruction(
    unit: &DenseBytecodeUnit,
    function: &DenseFunction,
    instruction: &DenseInstruction,
    resume: DeoptResumePoint,
    reason: VmDeoptReason,
) -> LiveStateSnapshot {
    let span = unit
        .spans
        .get(instruction.span.index())
        .copied()
        .unwrap_or_default();
    let value_identity = if matches!(
        reason,
        VmDeoptReason::ReferenceCowIdentity | VmDeoptReason::ForeachIteratorState
    ) {
        LiveIdentityMarker::MaybeReferenceOrCow
    } else {
        LiveIdentityMarker::Plain
    };
    LiveStateSnapshot {
        resume,
        span,
        registers: (0..function.register_count)
            .map(|index| LiveValueSlot {
                class: LiveValueClass::Register,
                index,
                initialized: None,
                identity: value_identity,
            })
            .collect(),
        locals: (0..function.local_count)
            .map(|index| LiveValueSlot {
                class: LiveValueClass::Local,
                index,
                initialized: None,
                identity: value_identity,
            })
            .collect(),
        operand_stack: Vec::new(),
        pending_exception: marker_for(reason, VmDeoptReason::ExceptionPending),
        pending_finally: marker_for(reason, VmDeoptReason::PendingFinally),
        foreach_iterator: marker_for(reason, VmDeoptReason::ForeachIteratorState),
        reference_cow: marker_for(reason, VmDeoptReason::ReferenceCowIdentity),
        output_buffer: marker_for(reason, VmDeoptReason::OutputBufferState),
        call_frame_identity: marker_for(reason, VmDeoptReason::CallFrameBoundary),
    }
}

fn marker_for(actual: VmDeoptReason, target: VmDeoptReason) -> ControlStateMarker {
    if actual == target {
        ControlStateMarker::Represented
    } else {
        ControlStateMarker::None
    }
}

fn reasons_for_instruction(instruction: &DenseInstruction) -> Vec<VmDeoptReason> {
    match instruction.opcode {
        DenseOpcode::Nop
        | DenseOpcode::LoadConst
        | DenseOpcode::Move
        | DenseOpcode::StoreLocal
        | DenseOpcode::Jump
        | DenseOpcode::Return
        | DenseOpcode::Discard => Vec::new(),
        DenseOpcode::LoadLocal | DenseOpcode::LoadLocalEcho => {
            vec![VmDeoptReason::UnsupportedValue]
        }
        DenseOpcode::BinaryAdd
        | DenseOpcode::BinarySub
        | DenseOpcode::BinaryMul
        | DenseOpcode::BinaryDiv
        | DenseOpcode::BinaryMod
        | DenseOpcode::BinaryConcat
        | DenseOpcode::BinaryPow
        | DenseOpcode::BinaryBitAnd
        | DenseOpcode::BinaryBitOr
        | DenseOpcode::BinaryBitXor
        | DenseOpcode::BinaryShiftLeft
        | DenseOpcode::BinaryShiftRight
        | DenseOpcode::BinaryConcatEcho
        | DenseOpcode::UnaryPlus
        | DenseOpcode::UnaryMinus
        | DenseOpcode::UnaryNot
        | DenseOpcode::UnaryBitNot
        | DenseOpcode::CompareEqual
        | DenseOpcode::CompareNotEqual
        | DenseOpcode::CompareIdentical
        | DenseOpcode::CompareNotIdentical
        | DenseOpcode::CompareLess
        | DenseOpcode::CompareLessEqual
        | DenseOpcode::CompareGreater
        | DenseOpcode::CompareGreaterEqual
        | DenseOpcode::CompareSpaceship => vec![
            VmDeoptReason::TypeMismatch,
            VmDeoptReason::Overflow,
            VmDeoptReason::HelperStatus,
        ],
        DenseOpcode::CallFunction => vec![VmDeoptReason::CallFrameBoundary],
        DenseOpcode::LoadConstEcho | DenseOpcode::Echo => {
            vec![VmDeoptReason::OutputBufferState]
        }
        DenseOpcode::NewArray
        | DenseOpcode::ArrayInsert
        | DenseOpcode::FetchDim
        | DenseOpcode::AssignDim
        | DenseOpcode::AppendDim => vec![VmDeoptReason::ReferenceCowIdentity],
        DenseOpcode::ForeachInit | DenseOpcode::ForeachNext => {
            vec![VmDeoptReason::ForeachIteratorState]
        }
        DenseOpcode::JumpIfFalse | DenseOpcode::JumpIfTrue | DenseOpcode::JumpIf => {
            vec![VmDeoptReason::TypeMismatch, VmDeoptReason::GuardFailed]
        }
    }
}

fn collect_ir_rejections(unit: &IrUnit) -> Vec<DeoptMetadataError> {
    let mut errors = Vec::new();
    for (function_index, function) in unit.functions.iter().enumerate() {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Some(reason) = rejection_for_ir_instruction(&instruction.kind) {
                    errors.push(DeoptMetadataError {
                        reason,
                        message: format!(
                            "function {function_index} block {} instruction {} requires {}",
                            block.id.raw(),
                            instruction.id.raw(),
                            reason.as_str()
                        ),
                    });
                }
            }
            if let Some(terminator) = &block.terminator
                && let TerminatorKind::Return {
                    by_ref_local: Some(_),
                    ..
                } = terminator.kind
            {
                errors.push(DeoptMetadataError {
                    reason: VmDeoptReason::ReferenceCowIdentity,
                    message: format!(
                        "function {function_index} block {} by-reference return requires {}",
                        block.id.raw(),
                        VmDeoptReason::ReferenceCowIdentity.as_str()
                    ),
                });
            }
        }
    }
    errors
}

fn rejection_for_ir_instruction(kind: &InstructionKind) -> Option<VmDeoptReason> {
    match kind {
        InstructionKind::BindReference { .. }
        | InstructionKind::BindGlobal { .. }
        | InstructionKind::BindReferenceDim { .. }
        | InstructionKind::BindReferenceFromDim { .. }
        | InstructionKind::BindReferenceFromCall { .. }
        | InstructionKind::ForeachInitRef { .. }
        | InstructionKind::ForeachNextRef { .. } => Some(VmDeoptReason::ReferenceCowIdentity),
        InstructionKind::CallFunction { args, .. }
        | InstructionKind::CallMethod { args, .. }
        | InstructionKind::CallStaticMethod { args, .. }
        | InstructionKind::CallClosure { args, .. }
        | InstructionKind::CallCallable { args, .. }
        | InstructionKind::NewObject { args, .. }
        | InstructionKind::DynamicNewObject { args, .. }
            if args.iter().any(argument_needs_reference_metadata) =>
        {
            Some(VmDeoptReason::ReferenceCowIdentity)
        }
        InstructionKind::EnterTry { .. }
        | InstructionKind::LeaveTry
        | InstructionKind::EndFinally { .. } => Some(VmDeoptReason::PendingFinally),
        InstructionKind::Throw { .. } | InstructionKind::MakeException { .. } => {
            Some(VmDeoptReason::ExceptionPending)
        }
        InstructionKind::Yield { .. } | InstructionKind::YieldFrom { .. } => {
            Some(VmDeoptReason::GeneratorOrFiberState)
        }
        InstructionKind::Include { .. }
        | InstructionKind::Eval { .. }
        | InstructionKind::Unsupported { .. }
        | InstructionKind::RuntimeError { .. } => Some(VmDeoptReason::UnsupportedControlFlow),
        _ => None,
    }
}

fn argument_needs_reference_metadata(arg: &IrCallArg) -> bool {
    arg.by_ref_local.is_some() || arg.by_ref_dim.is_some() || arg.by_ref_property.is_some()
}

fn verify_snapshot(
    region: &DeoptRegionMetadata,
    snapshot: &LiveStateSnapshot,
    errors: &mut Vec<DeoptMetadataError>,
) {
    if snapshot.resume.function != region.function
        || !region.blocks.contains(&snapshot.resume.block)
        || !region.instructions.contains(&snapshot.resume.instruction)
    {
        errors.push(DeoptMetadataError {
            reason: VmDeoptReason::UnsupportedControlFlow,
            message: format!(
                "region {} snapshot resume {:?} is outside region",
                region.region_id, snapshot.resume
            ),
        });
    }
    for value in snapshot
        .registers
        .iter()
        .chain(snapshot.locals.iter())
        .chain(snapshot.operand_stack.iter())
    {
        if matches!(value.class, LiveValueClass::OperandStack) {
            errors.push(DeoptMetadataError {
                reason: VmDeoptReason::UnsupportedControlFlow,
                message: format!(
                    "region {} has operand-stack slot {} but the VM is register-based",
                    region.region_id, value.index
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata_from_source(source: &str) -> Result<DeoptMetadata, Vec<DeoptMetadataError>> {
        let frontend = php_semantics::analyze_source(source);
        let result = php_ir::lower_frontend_result(
            &frontend,
            php_ir::LoweringOptions {
                source_path: "fixtures/deopt/fpe16.php".to_string(),
                ..php_ir::LoweringOptions::default()
            },
        );
        result
            .verification
            .expect("IR should verify before deopt metadata");
        DeoptMetadata::generate_from_ir(&result.unit)
    }

    fn rejected_reasons(source: &str) -> Vec<VmDeoptReason> {
        metadata_from_source(source)
            .expect_err("source should be rejected")
            .into_iter()
            .map(|error| error.reason)
            .collect()
    }

    #[test]
    fn deopt_metadata_covers_straight_line_scalar_region() {
        let metadata = metadata_from_source("<?php $x = 1 + 2; echo $x;")
            .expect("straight-line scalar metadata");
        metadata.verify().expect("metadata verifies");
        assert!(!metadata.native_execution);
        assert!(
            metadata
                .regions
                .iter()
                .flat_map(|region| &region.side_exits)
                .any(|exit| exit.reason == VmDeoptReason::TypeMismatch)
        );
        assert!(
            metadata
                .regions
                .iter()
                .flat_map(|region| &region.side_exits)
                .all(|exit| exit.snapshot.operand_stack.is_empty())
        );
    }

    #[test]
    fn deopt_metadata_covers_branch_resume_points() {
        let metadata = metadata_from_source("<?php $x = 1; if ($x) { echo 1; } else { echo 2; }")
            .expect("branch metadata");
        assert!(metadata.regions.len() >= 3);
        assert!(
            metadata
                .regions
                .iter()
                .flat_map(|region| &region.side_exits)
                .any(|exit| exit.reason == VmDeoptReason::GuardFailed)
        );
    }

    #[test]
    fn deopt_metadata_covers_loop_regions() {
        let metadata =
            metadata_from_source("<?php $i = 0; while ($i < 3) { $i = $i + 1; } echo $i;")
                .expect("loop metadata");
        assert!(metadata.regions.len() >= 2);
        assert!(
            metadata
                .regions
                .iter()
                .flat_map(|region| &region.side_exits)
                .any(|exit| exit.resume.block != 0)
        );
    }

    #[test]
    fn deopt_metadata_represents_by_value_foreach_state() {
        let metadata = metadata_from_source(
            "<?php $items = [1, 2]; foreach ($items as $value) { echo $value; }",
        )
        .expect("foreach metadata");
        assert!(
            metadata
                .regions
                .iter()
                .flat_map(|region| &region.side_exits)
                .any(|exit| {
                    exit.reason == VmDeoptReason::ForeachIteratorState
                        && exit.snapshot.foreach_iterator == ControlStateMarker::Represented
                })
        );
    }

    #[test]
    fn deopt_metadata_rejects_try_finally_state() {
        let reasons = rejected_reasons("<?php try { echo 1; } finally { echo 2; }");
        assert!(reasons.contains(&VmDeoptReason::PendingFinally));
    }

    #[test]
    fn deopt_metadata_rejects_exception_paths() {
        let reasons = rejected_reasons("<?php throw new Exception('boom');");
        assert!(reasons.contains(&VmDeoptReason::ExceptionPending));
    }

    #[test]
    fn deopt_metadata_rejects_generator_or_fiber_state() {
        let reasons = rejected_reasons(
            "<?php function gen() { yield 1; } foreach (gen() as $v) { echo $v; }",
        );
        assert!(reasons.contains(&VmDeoptReason::GeneratorOrFiberState));
    }

    #[test]
    fn deopt_metadata_rejects_reference_cow_state() {
        let reasons = rejected_reasons("<?php $a = 1; $b =& $a; echo $b;");
        assert!(reasons.contains(&VmDeoptReason::ReferenceCowIdentity));
    }

    #[test]
    fn deopt_reason_codes_match_existing_cranelift_side_exit_prefix() {
        assert_eq!(VmDeoptReason::TypeMismatch.code(), 1);
        assert_eq!(VmDeoptReason::Overflow.code(), 2);
        assert_eq!(VmDeoptReason::UnsupportedValue.code(), 3);
        assert_eq!(VmDeoptReason::GuardFailed.code(), 4);
        assert_eq!(VmDeoptReason::HelperStatus.code(), 5);
        assert_eq!(VmDeoptReason::ExceptionPending.code(), 6);
        assert_eq!(VmDeoptReason::AbiMismatch.code(), 7);
    }
}
