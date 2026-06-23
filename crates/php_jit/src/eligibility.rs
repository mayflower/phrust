//! Conservative JIT eligibility analysis for Phase 7.
//!
//! The analysis deliberately accepts only a tiny primitive, leaf-function IR
//! subset. Anything with PHP-visible dynamic behavior is rejected or marked
//! unknown before future lowering/codegen can see it.

use php_ir::instruction::{IrCallArg, TerminatorKind};
use php_ir::{
    BinaryOp, CastKind, CompareOp, FunctionId, Instruction, InstructionKind, IrCapture, IrConstant,
    IrFunction, IrParam, IrReturnType, IrUnit, Operand, UnaryOp,
};

/// Eligibility state for one JIT candidate region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JitEligibility {
    /// Region is inside the Phase 7 primitive subset.
    Eligible,
    /// Region is outside the subset for a stable, machine-readable reason.
    Rejected { reason: JitEligibilityReason },
    /// Region cannot be classified because the IR metadata is incomplete.
    Unknown { reason: JitEligibilityReason },
}

impl JitEligibility {
    /// Stable status spelling for reports.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Rejected { .. } => "rejected",
            Self::Unknown { .. } => "unknown",
        }
    }
}

/// Stable machine-readable reason attached to a rejection or unknown result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitEligibilityReason {
    /// Stable reason identifier.
    pub code: &'static str,
    /// Human-readable detail for debug output.
    pub detail: String,
    /// Block index when the reason is instruction-local.
    pub block: Option<u32>,
    /// Instruction index when the reason is instruction-local.
    pub instruction: Option<u32>,
}

impl JitEligibilityReason {
    fn function(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            block: None,
            instruction: None,
        }
    }

    fn instruction(
        code: &'static str,
        detail: impl Into<String>,
        block: u32,
        instruction: u32,
    ) -> Self {
        Self {
            code,
            detail: detail.into(),
            block: Some(block),
            instruction: Some(instruction),
        }
    }
}

/// Per-report analysis counters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JitEligibilityStats {
    /// Functions inspected by this report.
    pub functions_analyzed: u64,
    /// Blocks inspected by this report.
    pub blocks_analyzed: u64,
    /// Instructions inspected by this report.
    pub instructions_analyzed: u64,
    /// Eligible regions observed by this report.
    pub eligible: u64,
    /// Rejected regions observed by this report.
    pub rejected: u64,
    /// Unknown regions observed by this report.
    pub unknown: u64,
}

/// Stable eligibility report for logs, tests, and future CLI output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitEligibilityReport {
    /// Function ID requested by the caller.
    pub function: FunctionId,
    /// Function name when the ID resolved.
    pub function_name: Option<String>,
    /// Final eligibility state.
    pub eligibility: JitEligibility,
    /// All collected reasons, with the first reason mirrored in `eligibility`.
    pub reasons: Vec<JitEligibilityReason>,
    /// Analysis counters for this report.
    pub stats: JitEligibilityStats,
    /// Stable debug lines.
    pub debug: Vec<String>,
}

impl JitEligibilityReport {
    /// Returns a stable multi-line debug string.
    #[must_use]
    pub fn debug_output(&self) -> String {
        self.debug.join("\n")
    }
}

/// Analyzes one function in a unit for Phase 7 JIT eligibility.
#[must_use]
pub fn analyze_jit_eligibility(unit: &IrUnit, function: FunctionId) -> JitEligibilityReport {
    let Some(ir_function) = unit.functions.get(function.index()) else {
        let reason = JitEligibilityReason::function(
            "JIT_ELIGIBILITY_UNKNOWN_FUNCTION",
            format!(
                "function id {} is not present in the IR unit",
                function.raw()
            ),
        );
        return unknown_report(function, None, reason);
    };

    analyze_function(function, ir_function, &unit.constants)
}

fn analyze_function(
    function_id: FunctionId,
    function: &IrFunction,
    constants: &[IrConstant],
) -> JitEligibilityReport {
    let mut stats = JitEligibilityStats {
        functions_analyzed: 1,
        blocks_analyzed: function.blocks.len() as u64,
        ..JitEligibilityStats::default()
    };
    let mut rejected = Vec::new();
    let mut unknown = Vec::new();

    check_function_shape(function, &mut rejected, &mut unknown);

    for block in &function.blocks {
        let block_index = block.id.raw();
        for instruction in &block.instructions {
            stats.instructions_analyzed += 1;
            check_instruction(
                instruction,
                block_index,
                constants,
                &mut rejected,
                &mut unknown,
            );
        }

        match &block.terminator {
            Some(terminator) => {
                check_terminator(
                    &terminator.kind,
                    block_index,
                    constants,
                    &mut rejected,
                    &mut unknown,
                );
            }
            None => unknown.push(JitEligibilityReason::function(
                "JIT_ELIGIBILITY_UNKNOWN_MISSING_TERMINATOR",
                format!("block {} has no terminator", block.id.raw()),
            )),
        }
    }

    let (eligibility, reasons) = if let Some(reason) = rejected.first().cloned() {
        stats.rejected = 1;
        let mut reasons = rejected;
        reasons.extend(unknown);
        (JitEligibility::Rejected { reason }, reasons)
    } else if let Some(reason) = unknown.first().cloned() {
        stats.unknown = 1;
        (JitEligibility::Unknown { reason }, unknown)
    } else {
        stats.eligible = 1;
        (JitEligibility::Eligible, Vec::new())
    };

    let mut debug = Vec::new();
    debug.push(format!(
        "jit-eligibility function={} status={}",
        function.name,
        eligibility.as_str()
    ));
    debug.push(format!(
        "jit-eligibility stats functions={} blocks={} instructions={}",
        stats.functions_analyzed, stats.blocks_analyzed, stats.instructions_analyzed
    ));
    for reason in &reasons {
        match (reason.block, reason.instruction) {
            (Some(block), Some(instruction)) => debug.push(format!(
                "jit-eligibility reason code={} block={} instruction={} detail={}",
                reason.code, block, instruction, reason.detail
            )),
            _ => debug.push(format!(
                "jit-eligibility reason code={} detail={}",
                reason.code, reason.detail
            )),
        }
    }

    JitEligibilityReport {
        function: function_id,
        function_name: Some(function.name.clone()),
        eligibility,
        reasons,
        stats,
        debug,
    }
}

fn check_function_shape(
    function: &IrFunction,
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    if function.flags.is_generator {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_GENERATOR",
            "generators are outside the Phase 7 JIT subset",
        ));
    }
    if function.flags.is_closure && !function.captures.is_empty() {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_CLOSURE_CAPTURE",
            "capturing closures may observe reference and lifetime behavior",
        ));
    }
    if function.returns_by_ref {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_BY_REF_RETURN",
            "by-reference returns are outside the Phase 7 JIT subset",
        ));
    }
    if function.blocks.is_empty() {
        unknown.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_UNKNOWN_EMPTY_BODY",
            "function has no basic blocks",
        ));
    }

    for param in &function.params {
        check_param(param, rejected);
    }
    for capture in &function.captures {
        check_capture(capture, rejected);
    }
    if let Some(return_type) = &function.return_type {
        check_type(return_type, "return type", rejected);
    }
}

fn check_param(param: &IrParam, rejected: &mut Vec<JitEligibilityReason>) {
    if param.by_ref {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_BY_REF_PARAM",
            format!("parameter `${}` is by-reference", param.name),
        ));
    }
    if param.variadic {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_VARIADIC_PARAM",
            format!("parameter `${}` is variadic", param.name),
        ));
    }
    if let Some(type_) = &param.type_ {
        check_type(type_, "parameter type", rejected);
    }
}

fn check_capture(capture: &IrCapture, rejected: &mut Vec<JitEligibilityReason>) {
    if capture.by_ref {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_BY_REF_CAPTURE",
            format!("capture `${}` is by-reference", capture.name),
        ));
    }
}

fn check_type(
    type_: &IrReturnType,
    context: &'static str,
    rejected: &mut Vec<JitEligibilityReason>,
) {
    if !matches!(
        type_,
        IrReturnType::Int | IrReturnType::Bool | IrReturnType::False | IrReturnType::True
    ) {
        rejected.push(JitEligibilityReason::function(
            "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_TYPE",
            format!("{context} is not an int/bool primitive"),
        ));
    }
}

fn check_instruction(
    instruction: &Instruction,
    block: u32,
    constants: &[IrConstant],
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    let id = instruction.id.raw();
    match &instruction.kind {
        InstructionKind::Nop
        | InstructionKind::LoadLocal { .. }
        | InstructionKind::LoadLocalQuiet { .. }
        | InstructionKind::StoreLocal { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Discard { .. }
        | InstructionKind::IssetLocal { .. }
        | InstructionKind::EmptyLocal { .. } => {
            check_instruction_operands(instruction, block, constants, rejected, unknown);
        }
        InstructionKind::LoadConst { constant, .. } => {
            check_constant(*constant, block, id, constants, rejected, unknown);
        }
        InstructionKind::Binary { op, .. } => {
            if !is_allowed_binary(*op) {
                rejected.push(JitEligibilityReason::instruction(
                    "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_BINARY_OP",
                    format!("binary op {op:?} is outside the primitive int subset"),
                    block,
                    id,
                ));
            }
            check_instruction_operands(instruction, block, constants, rejected, unknown);
        }
        InstructionKind::Compare { op, .. } => {
            if !is_allowed_compare(*op) {
                rejected.push(JitEligibilityReason::instruction(
                    "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_COMPARE_OP",
                    format!("compare op {op:?} is outside the primitive subset"),
                    block,
                    id,
                ));
            }
            check_instruction_operands(instruction, block, constants, rejected, unknown);
        }
        InstructionKind::Unary { op, .. } => {
            if !is_allowed_unary(*op) {
                rejected.push(JitEligibilityReason::instruction(
                    "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_UNARY_OP",
                    format!("unary op {op:?} is outside the primitive int/bool subset"),
                    block,
                    id,
                ));
            }
            check_instruction_operands(instruction, block, constants, rejected, unknown);
        }
        InstructionKind::Cast { kind, .. } => {
            if !matches!(kind, CastKind::Bool | CastKind::Int) {
                rejected.push(JitEligibilityReason::instruction(
                    "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_CAST",
                    format!("cast {kind:?} is outside the primitive int/bool subset"),
                    block,
                    id,
                ));
            }
            check_instruction_operands(instruction, block, constants, rejected, unknown);
        }
        InstructionKind::BindReference { .. }
        | InstructionKind::BindGlobal { .. }
        | InstructionKind::BindReferenceDim { .. }
        | InstructionKind::BindReferenceFromDim { .. }
        | InstructionKind::BindReferenceFromCall { .. } => {
            rejected.push(JitEligibilityReason::instruction(
                "JIT_ELIGIBILITY_REJECT_REFERENCE_OPCODE",
                "reference-producing opcodes are outside the JIT subset",
                block,
                id,
            ))
        }
        InstructionKind::CallFunction { .. }
        | InstructionKind::CallMethod { .. }
        | InstructionKind::CallStaticMethod { .. }
        | InstructionKind::CallClosure { .. }
        | InstructionKind::CallCallable { .. }
        | InstructionKind::Pipe { .. }
        | InstructionKind::ResolveCallable { .. }
        | InstructionKind::MakeClosure { .. } => rejected.push(JitEligibilityReason::instruction(
            "JIT_ELIGIBILITY_REJECT_CALL_OPCODE",
            "calls and callable resolution are outside the default JIT subset",
            block,
            id,
        )),
        InstructionKind::EnterTry { .. }
        | InstructionKind::LeaveTry
        | InstructionKind::EndFinally { .. }
        | InstructionKind::Throw { .. }
        | InstructionKind::MakeException { .. } => {
            rejected.push(JitEligibilityReason::instruction(
                "JIT_ELIGIBILITY_REJECT_EXCEPTION_OPCODE",
                "exception control-flow is outside the JIT subset",
                block,
                id,
            ))
        }
        InstructionKind::Yield { .. } | InstructionKind::YieldFrom { .. } => {
            rejected.push(JitEligibilityReason::instruction(
                "JIT_ELIGIBILITY_REJECT_GENERATOR_OPCODE",
                "generator opcodes are outside the JIT subset",
                block,
                id,
            ))
        }
        InstructionKind::Include { .. } | InstructionKind::Eval { .. } => {
            rejected.push(JitEligibilityReason::instruction(
                "JIT_ELIGIBILITY_REJECT_INCLUDE_EVAL_OPCODE",
                "include/eval/autoload-sensitive opcodes are outside the JIT subset",
                block,
                id,
            ))
        }
        InstructionKind::NewArray { .. }
        | InstructionKind::ArrayInsert { .. }
        | InstructionKind::FetchDim { .. }
        | InstructionKind::AssignDim { .. }
        | InstructionKind::AppendDim { .. }
        | InstructionKind::IssetDim { .. }
        | InstructionKind::EmptyDim { .. }
        | InstructionKind::UnsetDim { .. }
        | InstructionKind::ForeachInit { .. }
        | InstructionKind::ForeachNext { .. }
        | InstructionKind::ForeachInitRef { .. }
        | InstructionKind::ForeachNextRef { .. }
        | InstructionKind::ArrayGet { .. } => rejected.push(JitEligibilityReason::instruction(
            "JIT_ELIGIBILITY_REJECT_ARRAY_OPCODE",
            "array and foreach opcodes are outside the JIT subset",
            block,
            id,
        )),
        InstructionKind::NewObject { .. }
        | InstructionKind::CloneObject { .. }
        | InstructionKind::CloneWith { .. }
        | InstructionKind::InstanceOf { .. }
        | InstructionKind::FetchProperty { .. }
        | InstructionKind::IssetProperty { .. }
        | InstructionKind::EmptyProperty { .. }
        | InstructionKind::UnsetProperty { .. }
        | InstructionKind::FetchStaticProperty { .. }
        | InstructionKind::FetchClassConstant { .. }
        | InstructionKind::AssignProperty { .. }
        | InstructionKind::AssignStaticProperty { .. } => {
            rejected.push(JitEligibilityReason::instruction(
                "JIT_ELIGIBILITY_REJECT_OBJECT_OPCODE",
                "objects, properties, classes, and destructors are outside the JIT subset",
                block,
                id,
            ))
        }
        InstructionKind::FetchConst { .. }
        | InstructionKind::InitStaticLocal { .. }
        | InstructionKind::UnsetLocal { .. }
        | InstructionKind::Echo { .. }
        | InstructionKind::Unsupported { .. }
        | InstructionKind::RuntimeError { .. } => rejected.push(JitEligibilityReason::instruction(
            "JIT_ELIGIBILITY_REJECT_OBSERVABLE_OPCODE",
            "observable or dynamic VM behavior is outside the JIT subset",
            block,
            id,
        )),
    }
}

fn check_terminator(
    terminator: &TerminatorKind,
    block: u32,
    constants: &[IrConstant],
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    match terminator {
        TerminatorKind::Jump { .. } => {}
        TerminatorKind::JumpIfFalse { condition, .. }
        | TerminatorKind::JumpIfTrue { condition, .. }
        | TerminatorKind::JumpIf { condition, .. } => {
            check_operand(*condition, block, u32::MAX, constants, rejected, unknown);
        }
        TerminatorKind::Return {
            value,
            by_ref_local,
        } => {
            if by_ref_local.is_some() {
                rejected.push(JitEligibilityReason::function(
                    "JIT_ELIGIBILITY_REJECT_BY_REF_RETURN",
                    "return terminator returns a local by reference",
                ));
            }
            if let Some(value) = value {
                check_operand(*value, block, u32::MAX, constants, rejected, unknown);
            }
        }
    }
}

fn check_instruction_operands(
    instruction: &Instruction,
    block: u32,
    constants: &[IrConstant],
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    let id = instruction.id.raw();
    match &instruction.kind {
        InstructionKind::Move { src, .. }
        | InstructionKind::StoreLocal { src, .. }
        | InstructionKind::Discard { src }
        | InstructionKind::Cast { src, .. }
        | InstructionKind::Unary { src, .. } => {
            check_operand(*src, block, id, constants, rejected, unknown);
        }
        InstructionKind::Binary { lhs, rhs, .. } | InstructionKind::Compare { lhs, rhs, .. } => {
            check_operand(*lhs, block, id, constants, rejected, unknown);
            check_operand(*rhs, block, id, constants, rejected, unknown);
        }
        _ => {}
    }
}

fn check_operand(
    operand: Operand,
    block: u32,
    instruction: u32,
    constants: &[IrConstant],
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    if let Operand::Constant(constant) = operand {
        check_constant(constant, block, instruction, constants, rejected, unknown);
    }
}

fn check_constant(
    constant: php_ir::ConstId,
    block: u32,
    instruction: u32,
    constants: &[IrConstant],
    rejected: &mut Vec<JitEligibilityReason>,
    unknown: &mut Vec<JitEligibilityReason>,
) {
    let Some(value) = constants.get(constant.index()) else {
        unknown.push(JitEligibilityReason::instruction(
            "JIT_ELIGIBILITY_UNKNOWN_CONSTANT",
            format!(
                "constant id {} is not present in the IR unit",
                constant.raw()
            ),
            block,
            instruction,
        ));
        return;
    };

    if !matches!(value, IrConstant::Int(_) | IrConstant::Bool(_)) {
        rejected.push(JitEligibilityReason::instruction(
            "JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_CONSTANT",
            format!("constant {value:?} is not an int/bool primitive"),
            block,
            instruction,
        ));
    }
}

fn is_allowed_binary(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Mod
    )
}

fn is_allowed_compare(op: CompareOp) -> bool {
    matches!(
        op,
        CompareOp::Identical
            | CompareOp::NotIdentical
            | CompareOp::Less
            | CompareOp::LessEqual
            | CompareOp::Greater
            | CompareOp::GreaterEqual
    )
}

fn is_allowed_unary(op: UnaryOp) -> bool {
    matches!(op, UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Not)
}

fn unknown_report(
    function: FunctionId,
    function_name: Option<String>,
    reason: JitEligibilityReason,
) -> JitEligibilityReport {
    JitEligibilityReport {
        function,
        function_name,
        eligibility: JitEligibility::Unknown {
            reason: reason.clone(),
        },
        reasons: vec![reason.clone()],
        stats: JitEligibilityStats {
            functions_analyzed: 0,
            unknown: 1,
            ..JitEligibilityStats::default()
        },
        debug: vec![
            "jit-eligibility function=<unknown> status=unknown".to_owned(),
            format!(
                "jit-eligibility reason code={} detail={}",
                reason.code, reason.detail
            ),
        ],
    }
}

/// Returns true when all call arguments stay in the future primitive intrinsic subset.
#[must_use]
pub fn call_args_are_jit_primitive(args: &[IrCallArg]) -> bool {
    args.iter()
        .all(|arg| arg.name.is_none() && !arg.unpack && arg.by_ref_local.is_none())
}
