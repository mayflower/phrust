//! Structured executable Region IR lowered from `php_ir`.

use php_ir::instruction::{IrCallArgValueKind, TerminatorKind};
use php_ir::{
    BinaryOp, BlockId, CompareOp, FunctionId, InstrId, InstructionKind, IrConstant, IrReturnType,
    IrSpan, IrUnit, LocalId, Operand, RegId,
};
use std::collections::BTreeSet;

/// A typed failure while constructing an executable region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableRegionBuildError {
    pub code: &'static str,
    pub detail: String,
}

impl ExecutableRegionBuildError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ExecutableRegionBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for ExecutableRegionBuildError {}

/// Scalar binary operations currently executable without a runtime helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionBinaryOp {
    Add,
    Sub,
    Mul,
}

/// Scalar comparison operations currently executable without a runtime helper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionCompareOpCode {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// Region operand detached from the source unit's constant pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableRegionOperand {
    Register(RegId),
    Local(LocalId),
    I64(i64),
}

/// One executable Region IR instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableRegionInstruction {
    pub id: InstrId,
    pub span: IrSpan,
    /// Stable continuation ID used by native PC/deopt metadata.
    pub continuation_id: u32,
    /// Locals definitely initialized immediately before this instruction.
    pub live_locals: Vec<LocalId>,
    pub kind: ExecutableRegionInstructionKind,
}

/// Instruction kinds in the initial general scalar region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableRegionInstructionKind {
    Nop,
    Move {
        dst: RegId,
        src: ExecutableRegionOperand,
    },
    LoadLocal {
        dst: RegId,
        local: LocalId,
    },
    StoreLocal {
        local: LocalId,
        src: ExecutableRegionOperand,
    },
    Discard {
        src: ExecutableRegionOperand,
    },
    Binary {
        dst: RegId,
        op: RegionBinaryOp,
        lhs: ExecutableRegionOperand,
        rhs: ExecutableRegionOperand,
    },
    Compare {
        dst: RegId,
        op: RegionCompareOpCode,
        lhs: ExecutableRegionOperand,
        rhs: ExecutableRegionOperand,
    },
    DirectCall {
        dst: RegId,
        target: FunctionId,
        args: Vec<ExecutableRegionOperand>,
    },
}

/// Explicit control flow for one executable region block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutableRegionTerminator {
    Jump {
        target: BlockId,
    },
    JumpIfFalse {
        condition: ExecutableRegionOperand,
        target: BlockId,
        fallthrough: BlockId,
    },
    JumpIfTrue {
        condition: ExecutableRegionOperand,
        target: BlockId,
        fallthrough: BlockId,
    },
    JumpIf {
        condition: ExecutableRegionOperand,
        if_true: BlockId,
        if_false: BlockId,
    },
    Return {
        value: ExecutableRegionOperand,
    },
}

/// One basic block in an executable region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableRegionBlock {
    pub id: BlockId,
    pub entry_live_locals: Vec<LocalId>,
    pub instructions: Vec<ExecutableRegionInstruction>,
    pub terminator_span: IrSpan,
    pub terminator_continuation_id: u32,
    pub terminator_live_locals: Vec<LocalId>,
    pub terminator: ExecutableRegionTerminator,
}

/// A native OSR entry at a loop header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableRegionOsrEntry {
    pub id: u32,
    pub block: BlockId,
    pub continuation_id: u32,
    pub live_locals: Vec<LocalId>,
}

/// A verified, multi-block Region IR function ready for backend lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutableRegion {
    pub function: FunctionId,
    pub function_name: String,
    pub parameter_locals: Vec<LocalId>,
    pub local_count: u32,
    pub register_count: u32,
    pub blocks: Vec<ExecutableRegionBlock>,
    pub fast_path_operations: u64,
}

impl ExecutableRegion {
    #[must_use]
    pub fn arity(&self) -> usize {
        self.parameter_locals.len()
    }

    #[must_use]
    pub fn has_control_flow(&self) -> bool {
        self.blocks.len() > 1
    }

    /// Returns one stable OSR entry for every loop header targeted by a backedge.
    #[must_use]
    pub fn osr_entries(&self) -> Vec<ExecutableRegionOsrEntry> {
        let mut headers = BTreeSet::new();
        for block in &self.blocks {
            for target in block.terminator.targets() {
                if target.raw() <= block.id.raw() {
                    headers.insert(target);
                }
            }
        }
        headers
            .into_iter()
            .enumerate()
            .filter_map(|(id, block)| {
                let region_block = self.blocks.get(block.index())?;
                let continuation_id = region_block
                    .instructions
                    .first()
                    .map(|instruction| instruction.continuation_id)
                    .unwrap_or(region_block.terminator_continuation_id);
                Some(ExecutableRegionOsrEntry {
                    id: id as u32,
                    block,
                    continuation_id,
                    live_locals: region_block.entry_live_locals.clone(),
                })
            })
            .collect()
    }

    /// Direct userland callees referenced by this region.
    #[must_use]
    pub fn direct_callees(&self) -> Vec<FunctionId> {
        let mut callees = BTreeSet::new();
        for block in &self.blocks {
            for instruction in &block.instructions {
                if let ExecutableRegionInstructionKind::DirectCall { target, .. } =
                    &instruction.kind
                {
                    callees.insert(*target);
                }
            }
        }
        callees.into_iter().collect()
    }

    /// Verifies dense IDs and all explicit CFG targets.
    pub fn verify(&self) -> Result<(), ExecutableRegionBuildError> {
        if self.blocks.is_empty() {
            return Err(ExecutableRegionBuildError::new(
                "JIT_REGION_REJECT_EMPTY",
                "executable region has no blocks",
            ));
        }
        for (index, block) in self.blocks.iter().enumerate() {
            if block.id.index() != index {
                return Err(ExecutableRegionBuildError::new(
                    "JIT_REGION_REJECT_BLOCK_IDS",
                    format!("block {} appears at position {index}", block.id.raw()),
                ));
            }
            for target in block.terminator.targets() {
                if target.index() >= self.blocks.len() {
                    return Err(ExecutableRegionBuildError::new(
                        "JIT_REGION_REJECT_TARGET",
                        format!(
                            "block {} targets missing block {}",
                            block.id.raw(),
                            target.raw()
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl ExecutableRegionTerminator {
    fn targets(&self) -> Vec<BlockId> {
        match self {
            Self::Jump { target } => vec![*target],
            Self::JumpIfFalse {
                target,
                fallthrough,
                ..
            }
            | Self::JumpIfTrue {
                target,
                fallthrough,
                ..
            } => vec![*target, *fallthrough],
            Self::JumpIf {
                if_true, if_false, ..
            } => vec![*if_true, *if_false],
            Self::Return { .. } => Vec::new(),
        }
    }
}

/// Builds the initial general executable region from the authoritative PHP IR.
pub fn build_executable_region(
    unit: &IrUnit,
    function: FunctionId,
) -> Result<ExecutableRegion, ExecutableRegionBuildError> {
    let ir_function = unit.functions.get(function.index()).ok_or_else(|| {
        ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_MISSING_FUNCTION",
            format!("function id {} is not present", function.raw()),
        )
    })?;
    if ir_function.flags.is_top_level
        || ir_function.flags.is_closure
        || ir_function.flags.is_method
        || ir_function.flags.is_generator
        || ir_function.returns_by_ref
        || !ir_function.captures.is_empty()
    {
        return Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_FUNCTION_STATE",
            "initial executable region requires an ordinary function",
        ));
    }
    if ir_function.return_type.as_ref() != Some(&IrReturnType::Int) {
        return Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_RETURN_TYPE",
            "initial executable region requires an int return type",
        ));
    }
    if ir_function.params.len() > 4 {
        return Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_ARITY",
            "initial executable region supports at most four parameters",
        ));
    }
    if ir_function.local_count as usize > crate::JIT_DEOPT_MAX_SLOTS {
        return Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_DEOPT_SLOTS",
            format!(
                "region has {} locals but the native deopt ABI supports {}",
                ir_function.local_count,
                crate::JIT_DEOPT_MAX_SLOTS
            ),
        ));
    }
    for param in &ir_function.params {
        if param.by_ref
            || param.variadic
            || param.default.is_some()
            || param.type_.as_ref() != Some(&IrReturnType::Int)
        {
            return Err(ExecutableRegionBuildError::new(
                "JIT_REGION_REJECT_PARAM",
                "initial executable region requires plain int parameters",
            ));
        }
    }

    let mut fast_path_operations = 0_u64;
    let mut blocks = Vec::with_capacity(ir_function.blocks.len());
    let mut next_continuation = 0_u32;
    for (block_index, block) in ir_function.blocks.iter().enumerate() {
        let mut instructions = Vec::with_capacity(block.instructions.len());
        for instruction in &block.instructions {
            let kind = match &instruction.kind {
                InstructionKind::Nop => ExecutableRegionInstructionKind::Nop,
                InstructionKind::LoadConst { dst, constant } => {
                    ExecutableRegionInstructionKind::Move {
                        dst: *dst,
                        src: lower_constant(unit, *constant)?,
                    }
                }
                InstructionKind::Move { dst, src } => ExecutableRegionInstructionKind::Move {
                    dst: *dst,
                    src: lower_operand(unit, *src)?,
                },
                InstructionKind::LoadLocal { dst, local }
                | InstructionKind::LoadLocalQuiet { dst, local } => {
                    ExecutableRegionInstructionKind::LoadLocal {
                        dst: *dst,
                        local: *local,
                    }
                }
                InstructionKind::StoreLocal { local, src } => {
                    ExecutableRegionInstructionKind::StoreLocal {
                        local: *local,
                        src: lower_operand(unit, *src)?,
                    }
                }
                InstructionKind::Discard { src } => ExecutableRegionInstructionKind::Discard {
                    src: lower_operand(unit, *src)?,
                },
                InstructionKind::Binary { dst, op, lhs, rhs } => {
                    fast_path_operations = fast_path_operations.saturating_add(1);
                    ExecutableRegionInstructionKind::Binary {
                        dst: *dst,
                        op: lower_binary(*op)?,
                        lhs: lower_operand(unit, *lhs)?,
                        rhs: lower_operand(unit, *rhs)?,
                    }
                }
                InstructionKind::Compare { dst, op, lhs, rhs } => {
                    fast_path_operations = fast_path_operations.saturating_add(1);
                    ExecutableRegionInstructionKind::Compare {
                        dst: *dst,
                        op: lower_compare(*op)?,
                        lhs: lower_operand(unit, *lhs)?,
                        rhs: lower_operand(unit, *rhs)?,
                    }
                }
                InstructionKind::CallFunction { dst, name, args } => {
                    if args.iter().any(|arg| {
                        arg.name.is_some()
                            || arg.unpack
                            || arg.value_kind != IrCallArgValueKind::Direct
                    }) {
                        return Err(ExecutableRegionBuildError::new(
                            "JIT_REGION_REJECT_CALL_ARGUMENTS",
                            format!("call to {name} requires complex argument binding"),
                        ));
                    }
                    let target = unit
                        .function_table
                        .iter()
                        .find(|entry| entry.name == *name)
                        .map(|entry| entry.function)
                        .ok_or_else(|| {
                            ExecutableRegionBuildError::new(
                                "JIT_REGION_REJECT_CALL_TARGET",
                                format!("call target {name} is not stable in this IR unit"),
                            )
                        })?;
                    let target_function = unit.functions.get(target.index()).ok_or_else(|| {
                        ExecutableRegionBuildError::new(
                            "JIT_REGION_REJECT_CALL_TARGET",
                            format!("call target {name} is missing from this IR unit"),
                        )
                    })?;
                    if args.len() != target_function.params.len() {
                        return Err(ExecutableRegionBuildError::new(
                            "JIT_REGION_REJECT_CALL_ARITY",
                            format!(
                                "call to {name} passes {} arguments but requires {}",
                                args.len(),
                                target_function.params.len()
                            ),
                        ));
                    }
                    fast_path_operations = fast_path_operations.saturating_add(1);
                    ExecutableRegionInstructionKind::DirectCall {
                        dst: *dst,
                        target,
                        args: args
                            .iter()
                            .map(|arg| lower_operand(unit, arg.value))
                            .collect::<Result<Vec<_>, _>>()?,
                    }
                }
                other => {
                    return Err(ExecutableRegionBuildError::new(
                        "JIT_REGION_REJECT_INSTRUCTION",
                        format!("instruction {other:?} has no executable Region IR lowering"),
                    ));
                }
            };
            instructions.push(ExecutableRegionInstruction {
                id: instruction.id,
                span: instruction.span,
                continuation_id: next_continuation,
                live_locals: Vec::new(),
                kind,
            });
            next_continuation = next_continuation.saturating_add(1);
        }
        let terminator = lower_terminator(unit, ir_function.blocks.len(), block_index, block)?;
        let terminator_span = block
            .terminator
            .as_ref()
            .expect("lower_terminator accepted this block")
            .span;
        blocks.push(ExecutableRegionBlock {
            id: block.id,
            entry_live_locals: Vec::new(),
            instructions,
            terminator_span,
            terminator_continuation_id: next_continuation,
            terminator_live_locals: Vec::new(),
            terminator,
        });
        next_continuation = next_continuation.saturating_add(1);
    }
    populate_live_locals(
        &mut blocks,
        &ir_function
            .params
            .iter()
            .map(|param| param.local)
            .collect::<Vec<_>>(),
    );
    let region = ExecutableRegion {
        function,
        function_name: ir_function.name.clone(),
        parameter_locals: ir_function.params.iter().map(|param| param.local).collect(),
        local_count: ir_function.local_count,
        register_count: ir_function.register_count,
        blocks,
        fast_path_operations,
    };
    region.verify()?;
    Ok(region)
}

fn populate_live_locals(blocks: &mut [ExecutableRegionBlock], params: &[LocalId]) {
    let mut candidates = params.iter().copied().collect::<BTreeSet<_>>();
    let mut definitions = Vec::with_capacity(blocks.len());
    let mut predecessors = vec![Vec::<usize>::new(); blocks.len()];
    for block in blocks.iter() {
        let mut defs = BTreeSet::new();
        for instruction in &block.instructions {
            if let ExecutableRegionInstructionKind::StoreLocal { local, .. } = instruction.kind {
                defs.insert(local);
                candidates.insert(local);
            }
        }
        definitions.push(defs);
        for target in block.terminator.targets() {
            if let Some(target_predecessors) = predecessors.get_mut(target.index()) {
                target_predecessors.push(block.id.index());
            }
        }
    }

    let entry = params.iter().copied().collect::<BTreeSet<_>>();
    let mut initialized_in = vec![candidates.clone(); blocks.len()];
    if let Some(first) = initialized_in.first_mut() {
        *first = entry;
    }
    loop {
        let initialized_out = initialized_in
            .iter()
            .zip(&definitions)
            .map(|(incoming, defs)| incoming.union(defs).copied().collect::<BTreeSet<_>>())
            .collect::<Vec<_>>();
        let mut changed = false;
        for block_index in 1..blocks.len() {
            let Some((first, rest)) = predecessors[block_index].split_first() else {
                continue;
            };
            let mut incoming = initialized_out[*first].clone();
            for predecessor in rest {
                incoming = incoming
                    .intersection(&initialized_out[*predecessor])
                    .copied()
                    .collect();
            }
            if initialized_in[block_index] != incoming {
                initialized_in[block_index] = incoming;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for (block, incoming) in blocks.iter_mut().zip(initialized_in) {
        let mut initialized = incoming;
        block.entry_live_locals = initialized.iter().copied().collect();
        for instruction in &mut block.instructions {
            instruction.live_locals = initialized.iter().copied().collect();
            if let ExecutableRegionInstructionKind::StoreLocal { local, .. } = instruction.kind {
                initialized.insert(local);
            }
        }
        block.terminator_live_locals = initialized.into_iter().collect();
    }
}

fn lower_binary(op: BinaryOp) -> Result<RegionBinaryOp, ExecutableRegionBuildError> {
    match op {
        BinaryOp::Add => Ok(RegionBinaryOp::Add),
        BinaryOp::Sub => Ok(RegionBinaryOp::Sub),
        BinaryOp::Mul => Ok(RegionBinaryOp::Mul),
        other => Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_BINARY",
            format!("binary operation {other:?} has no scalar Region IR lowering"),
        )),
    }
}

fn lower_compare(op: CompareOp) -> Result<RegionCompareOpCode, ExecutableRegionBuildError> {
    match op {
        CompareOp::Equal | CompareOp::Identical => Ok(RegionCompareOpCode::Equal),
        CompareOp::NotEqual | CompareOp::NotIdentical => Ok(RegionCompareOpCode::NotEqual),
        CompareOp::Less => Ok(RegionCompareOpCode::Less),
        CompareOp::LessEqual => Ok(RegionCompareOpCode::LessEqual),
        CompareOp::Greater => Ok(RegionCompareOpCode::Greater),
        CompareOp::GreaterEqual => Ok(RegionCompareOpCode::GreaterEqual),
        other => Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_COMPARE",
            format!("comparison {other:?} has no scalar Region IR lowering"),
        )),
    }
}

fn lower_operand(
    unit: &IrUnit,
    operand: Operand,
) -> Result<ExecutableRegionOperand, ExecutableRegionBuildError> {
    match operand {
        Operand::Register(register) => Ok(ExecutableRegionOperand::Register(register)),
        Operand::Local(local) => Ok(ExecutableRegionOperand::Local(local)),
        Operand::Constant(constant) => lower_constant(unit, constant),
    }
}

fn lower_constant(
    unit: &IrUnit,
    constant: php_ir::ConstId,
) -> Result<ExecutableRegionOperand, ExecutableRegionBuildError> {
    match unit.constants.get(constant.index()) {
        Some(IrConstant::Int(value)) => Ok(ExecutableRegionOperand::I64(*value)),
        Some(other) => Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_CONSTANT",
            format!("constant {other:?} is outside the scalar Region IR"),
        )),
        None => Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_CONSTANT",
            format!("constant {} is missing", constant.raw()),
        )),
    }
}

fn lower_terminator(
    unit: &IrUnit,
    block_count: usize,
    block_index: usize,
    block: &php_ir::BasicBlock,
) -> Result<ExecutableRegionTerminator, ExecutableRegionBuildError> {
    let terminator = block.terminator.as_ref().ok_or_else(|| {
        ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_TERMINATOR",
            format!("block {} has no terminator", block.id.raw()),
        )
    })?;
    let fallthrough = || {
        (block_index + 1 < block_count)
            .then(|| BlockId::new((block_index + 1) as u32))
            .ok_or_else(|| {
                ExecutableRegionBuildError::new(
                    "JIT_REGION_REJECT_FALLTHROUGH",
                    format!("block {} has no fallthrough block", block.id.raw()),
                )
            })
    };
    match &terminator.kind {
        TerminatorKind::Jump { target } => Ok(ExecutableRegionTerminator::Jump { target: *target }),
        TerminatorKind::JumpIfFalse { condition, target } => {
            Ok(ExecutableRegionTerminator::JumpIfFalse {
                condition: lower_operand(unit, *condition)?,
                target: *target,
                fallthrough: fallthrough()?,
            })
        }
        TerminatorKind::JumpIfTrue { condition, target } => {
            Ok(ExecutableRegionTerminator::JumpIfTrue {
                condition: lower_operand(unit, *condition)?,
                target: *target,
                fallthrough: fallthrough()?,
            })
        }
        TerminatorKind::JumpIf {
            condition,
            if_true,
            if_false,
        } => Ok(ExecutableRegionTerminator::JumpIf {
            condition: lower_operand(unit, *condition)?,
            if_true: *if_true,
            if_false: *if_false,
        }),
        TerminatorKind::Return {
            value: Some(value),
            by_ref_local: None,
        } => Ok(ExecutableRegionTerminator::Return {
            value: lower_operand(unit, *value)?,
        }),
        other => Err(ExecutableRegionBuildError::new(
            "JIT_REGION_REJECT_TERMINATOR",
            format!("terminator {other:?} has no executable Region IR lowering"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_ir::{FunctionFlags, IrBuilder, IrParam, IrSpan, UnitId};

    #[test]
    fn builds_verified_multiblock_region_from_php_ir() {
        let mut builder = IrBuilder::new(UnitId::new(91));
        let file = builder.add_file("region.php");
        let span = IrSpan::new(file, 0, 1);
        let function = builder.start_function("region", FunctionFlags::default(), span);
        let local = builder.intern_local(function, "value");
        builder.push_param(
            function,
            IrParam {
                name: "value".to_owned(),
                local,
                required: true,
                type_: Some(IrReturnType::Int),
                by_ref: false,
                variadic: false,
                default: None,
                attributes: Vec::new(),
            },
        );
        builder.set_return_type(function, Some(IrReturnType::Int));
        let entry = builder.append_block(function);
        let body = builder.append_block(function);
        builder.terminate_jump(function, entry, body, span);
        let loaded = builder.alloc_register(function);
        builder.emit(
            function,
            body,
            InstructionKind::LoadLocal { dst: loaded, local },
            span,
        );
        builder.terminate_return(function, body, Some(Operand::Register(loaded)), span);
        let unit = builder.finish();
        let region = build_executable_region(&unit, function).expect("region");
        assert_eq!(region.arity(), 1);
        assert_eq!(region.blocks.len(), 2);
        region.verify().expect("verified region");
    }
}
