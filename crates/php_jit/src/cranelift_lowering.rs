//! Optional Cranelift IR lowering prototype for Prompt 07.52.
//!
//! This module is compiled only with `jit-cranelift`. It produces and verifies
//! Cranelift IR text for a tiny integer subset, but it never allocates
//! executable memory and never returns a callable native pointer.

use crate::{JitEligibility, analyze_jit_eligibility};
use cranelift_codegen::ir::{
    self, AbiParam, Function, InstBuilder, Signature, UserFuncName, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use php_ir::instruction::TerminatorKind;
use php_ir::{
    BinaryOp, FunctionId, InstructionKind, IrConstant, IrFunction, IrUnit, LocalId, Operand, RegId,
};
use std::collections::BTreeMap;
use std::fmt;

/// Stable Cranelift lowering result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CraneliftLoweringResult {
    /// Function ID lowered from the IR unit.
    pub function: FunctionId,
    /// IR function name used for diagnostics.
    pub function_name: String,
    /// Generated Cranelift IR text.
    pub clif: String,
    /// Prototype counters.
    pub stats: CraneliftLoweringStats,
    /// Native execution handle. Always `None` in Prompt 07.52.
    pub machine_code_handle: Option<CraneliftMachineCodeHandle>,
}

/// Opaque future machine-code handle.
///
/// Prompt 07.52 does not create these handles; the type exists so callers can
/// model the boundary without treating IR text as executable code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CraneliftMachineCodeHandle {
    /// Stable opaque handle ID.
    pub id: u64,
}

/// Per-lowering counters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CraneliftLoweringStats {
    /// Basic blocks lowered.
    pub blocks_lowered: u64,
    /// Instructions lowered.
    pub instructions_lowered: u64,
    /// Cranelift verifier ran successfully.
    pub verified: bool,
}

/// Typed Cranelift lowering failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CraneliftLoweringError {
    /// Stable machine-readable rejection code.
    pub code: &'static str,
    /// Human-readable detail.
    pub detail: String,
}

impl CraneliftLoweringError {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for CraneliftLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for CraneliftLoweringError {}

/// Lowers one eligible integer leaf function into Cranelift IR text.
///
/// The supported subset is intentionally minimal:
/// - integer constants,
/// - integer add/sub/mul,
/// - register moves of lowered integer values,
/// - a single integer return.
pub fn lower_function_to_cranelift(
    unit: &IrUnit,
    function: FunctionId,
) -> Result<CraneliftLoweringResult, CraneliftLoweringError> {
    let eligibility = analyze_jit_eligibility(unit, function);
    match &eligibility.eligibility {
        JitEligibility::Eligible => {}
        JitEligibility::Rejected { reason } => {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_ELIGIBILITY",
                format!(
                    "eligibility rejected function before lowering: {} ({})",
                    reason.code, reason.detail
                ),
            ));
        }
        JitEligibility::Unknown { reason } => {
            return Err(CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_ELIGIBILITY_UNKNOWN",
                format!(
                    "eligibility could not classify function before lowering: {} ({})",
                    reason.code, reason.detail
                ),
            ));
        }
    }

    let ir_function = unit.functions.get(function.index()).ok_or_else(|| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_MISSING_FUNCTION",
            format!("function id {} is not present", function.raw()),
        )
    })?;
    lower_checked_function(unit, function, ir_function)
}

fn lower_checked_function(
    unit: &IrUnit,
    function_id: FunctionId,
    ir_function: &IrFunction,
) -> Result<CraneliftLoweringResult, CraneliftLoweringError> {
    if ir_function.blocks.len() != 1 {
        return Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_CONTROL_FLOW",
            format!(
                "expected exactly one basic block, found {}",
                ir_function.blocks.len()
            ),
        ));
    }

    let mut signature = Signature::new(CallConv::SystemV);
    for _ in &ir_function.params {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(types::I64));
    let mut function =
        Function::with_name_signature(UserFuncName::user(0, function_id.raw()), signature);
    let mut builder_context = FunctionBuilderContext::new();
    let mut registers = BTreeMap::new();
    let mut locals = BTreeMap::new();
    let mut stats = CraneliftLoweringStats::default();

    {
        let mut builder = FunctionBuilder::new(&mut function, &mut builder_context);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);
        for (param, cl_value) in ir_function
            .params
            .iter()
            .zip(builder.block_params(block).iter().copied())
        {
            locals.insert(param.local, cl_value);
        }

        let ir_block = &ir_function.blocks[0];
        stats.blocks_lowered = 1;
        for instruction in &ir_block.instructions {
            match &instruction.kind {
                InstructionKind::Nop => {}
                InstructionKind::LoadConst { dst, constant } => {
                    let value = constant_value(unit, *constant)?;
                    let cl_value = builder.ins().iconst(types::I64, value);
                    registers.insert(*dst, cl_value);
                    stats.instructions_lowered += 1;
                }
                InstructionKind::Move { dst, src } => {
                    let cl_value = lower_operand(&mut builder, &registers, &locals, unit, src)?;
                    registers.insert(*dst, cl_value);
                    stats.instructions_lowered += 1;
                }
                InstructionKind::LoadLocal { dst, local } => {
                    let cl_value = locals.get(local).copied().ok_or_else(|| {
                        CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_MISSING_LOCAL",
                            format!("local {} has not been lowered", local.raw()),
                        )
                    })?;
                    registers.insert(*dst, cl_value);
                    stats.instructions_lowered += 1;
                }
                InstructionKind::StoreLocal { local, src } => {
                    let cl_value = lower_operand(&mut builder, &registers, &locals, unit, src)?;
                    locals.insert(*local, cl_value);
                    stats.instructions_lowered += 1;
                }
                InstructionKind::Binary { dst, op, lhs, rhs } => {
                    let lhs = lower_operand(&mut builder, &registers, &locals, unit, lhs)?;
                    let rhs = lower_operand(&mut builder, &registers, &locals, unit, rhs)?;
                    let cl_value = match op {
                        BinaryOp::Add => builder.ins().iadd(lhs, rhs),
                        BinaryOp::Sub => builder.ins().isub(lhs, rhs),
                        BinaryOp::Mul => builder.ins().imul(lhs, rhs),
                        other => {
                            return Err(CraneliftLoweringError::new(
                                "JIT_CRANELIFT_REJECT_UNSUPPORTED_BINARY",
                                format!("binary op {other:?} is outside the prototype subset"),
                            ));
                        }
                    };
                    registers.insert(*dst, cl_value);
                    stats.instructions_lowered += 1;
                }
                other => {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_UNSUPPORTED_OPCODE",
                        format!("instruction {other:?} is outside the prototype subset"),
                    ));
                }
            }
        }

        match &ir_block.terminator {
            Some(terminator) => match &terminator.kind {
                TerminatorKind::Return {
                    value: Some(value),
                    by_ref_local: None,
                } => {
                    let value = lower_operand(&mut builder, &registers, &locals, unit, value)?;
                    builder.ins().return_(&[value]);
                }
                TerminatorKind::Return {
                    value: None,
                    by_ref_local: _,
                } => {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_RETURN",
                        "prototype requires an integer return value",
                    ));
                }
                TerminatorKind::Return {
                    value: Some(_),
                    by_ref_local: Some(_),
                } => {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_BY_REF_RETURN",
                        "by-reference returns are outside the prototype subset",
                    ));
                }
                other => {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_CONTROL_FLOW",
                        format!("terminator {other:?} is outside the prototype subset"),
                    ));
                }
            },
            None => {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_MISSING_TERMINATOR",
                    "basic block has no return terminator",
                ));
            }
        }

        builder.finalize();
    }

    let flags = settings::Flags::new(settings::builder());
    verify_function(&function, &flags).map_err(|error| {
        CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_VERIFIER",
            format!("Cranelift verifier rejected generated IR: {error}"),
        )
    })?;
    stats.verified = true;

    Ok(CraneliftLoweringResult {
        function: function_id,
        function_name: ir_function.name.clone(),
        clif: function.display().to_string(),
        stats,
        machine_code_handle: None,
    })
}

fn lower_operand(
    builder: &mut FunctionBuilder<'_>,
    registers: &BTreeMap<RegId, ir::Value>,
    locals: &BTreeMap<LocalId, ir::Value>,
    unit: &IrUnit,
    operand: &Operand,
) -> Result<ir::Value, CraneliftLoweringError> {
    match operand {
        Operand::Register(reg) => registers.get(reg).copied().ok_or_else(|| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_MISSING_REGISTER",
                format!("register {} has not been lowered", reg.raw()),
            )
        }),
        Operand::Constant(constant) => {
            let value = constant_value(unit, *constant)?;
            Ok(builder.ins().iconst(types::I64, value))
        }
        Operand::Local(local) => locals.get(local).copied().ok_or_else(|| {
            CraneliftLoweringError::new(
                "JIT_CRANELIFT_REJECT_MISSING_LOCAL",
                format!("local {} has not been lowered", local.raw()),
            )
        }),
    }
}

fn constant_value(unit: &IrUnit, constant: php_ir::ConstId) -> Result<i64, CraneliftLoweringError> {
    match unit.constants.get(constant.index()) {
        Some(IrConstant::Int(value)) => Ok(*value),
        Some(other) => Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_NON_INT_CONSTANT",
            format!("constant {other:?} is not an integer"),
        )),
        None => Err(CraneliftLoweringError::new(
            "JIT_CRANELIFT_REJECT_MISSING_CONSTANT",
            format!("constant id {} is not present", constant.raw()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::lower_function_to_cranelift;
    use php_ir::{
        BinaryOp, FunctionFlags, FunctionId, InstructionKind, IrBuilder, IrConstant, IrSpan,
        Operand, UnitId,
    };

    #[test]
    fn lowers_integer_arithmetic_to_cranelift_ir_without_execution() {
        let (unit, function) = arithmetic_fixture();
        let result =
            lower_function_to_cranelift(&unit, function).expect("arithmetic subset lowers");

        assert_eq!(result.function, function);
        assert_eq!(result.function_name, "jit_arithmetic");
        assert!(result.clif.contains("iconst.i64 10"));
        assert!(result.clif.contains("iconst.i64 3"));
        assert!(result.clif.contains("iadd"));
        assert!(result.clif.contains("isub"));
        assert!(result.clif.contains("imul"));
        assert!(result.clif.contains("return"));
        assert!(result.stats.verified);
        assert_eq!(result.stats.blocks_lowered, 1);
        assert_eq!(result.stats.instructions_lowered, 6);
        assert!(result.machine_code_handle.is_none());
    }

    #[test]
    fn rejects_unsupported_ir_with_typed_error() {
        let (unit, function) = unsupported_binary_fixture();
        let error = lower_function_to_cranelift(&unit, function)
            .expect_err("division must not be silently lowered");

        assert_eq!(error.code, "JIT_CRANELIFT_REJECT_ELIGIBILITY");
        assert!(
            error
                .detail
                .contains("JIT_ELIGIBILITY_REJECT_NON_PRIMITIVE_BINARY_OP")
        );
    }

    #[test]
    fn rejects_non_int_constant_after_eligibility() {
        let (unit, function) = bool_return_fixture();
        let error = lower_function_to_cranelift(&unit, function)
            .expect_err("bool constants are not part of 07.52 lowering");

        assert_eq!(error.code, "JIT_CRANELIFT_REJECT_NON_INT_CONSTANT");
    }

    fn arithmetic_fixture() -> (php_ir::IrUnit, FunctionId) {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("tests/fixtures/phase7/jit/eligible-int-add.php");
        let span = IrSpan::new(file, 0, 0);
        let function = builder.start_function("jit_arithmetic", FunctionFlags::default(), span);
        builder.set_entry(function);
        let block = builder.append_block(function);
        let ten = builder.add_constant(IrConstant::Int(10));
        let three = builder.add_constant(IrConstant::Int(3));
        let two = builder.add_constant(IrConstant::Int(2));
        let r0 = builder.alloc_register(function);
        let r1 = builder.alloc_register(function);
        let r2 = builder.alloc_register(function);
        let r3 = builder.alloc_register(function);
        let r4 = builder.alloc_register(function);
        let r5 = builder.alloc_register(function);
        builder.emit_load_const(function, block, r0, ten, span);
        builder.emit_load_const(function, block, r1, three, span);
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
        builder.emit(
            function,
            block,
            InstructionKind::Binary {
                dst: r3,
                op: BinaryOp::Sub,
                lhs: Operand::Register(r2),
                rhs: Operand::Register(r1),
            },
            span,
        );
        builder.emit_load_const(function, block, r4, two, span);
        builder.emit(
            function,
            block,
            InstructionKind::Binary {
                dst: r5,
                op: BinaryOp::Mul,
                lhs: Operand::Register(r3),
                rhs: Operand::Register(r4),
            },
            span,
        );
        builder.terminate_return(function, block, Some(Operand::Register(r5)), span);
        (builder.finish(), function)
    }

    fn unsupported_binary_fixture() -> (php_ir::IrUnit, FunctionId) {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("tests/fixtures/phase7/jit/rejected-dynamic.php");
        let span = IrSpan::new(file, 0, 0);
        let function = builder.start_function("jit_division", FunctionFlags::default(), span);
        builder.set_entry(function);
        let block = builder.append_block(function);
        let six = builder.add_constant(IrConstant::Int(6));
        let three = builder.add_constant(IrConstant::Int(3));
        let r0 = builder.alloc_register(function);
        let r1 = builder.alloc_register(function);
        let r2 = builder.alloc_register(function);
        builder.emit_load_const(function, block, r0, six, span);
        builder.emit_load_const(function, block, r1, three, span);
        builder.emit(
            function,
            block,
            InstructionKind::Binary {
                dst: r2,
                op: BinaryOp::Div,
                lhs: Operand::Register(r0),
                rhs: Operand::Register(r1),
            },
            span,
        );
        builder.terminate_return(function, block, Some(Operand::Register(r2)), span);
        (builder.finish(), function)
    }

    fn bool_return_fixture() -> (php_ir::IrUnit, FunctionId) {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("tests/fixtures/phase7/jit/rejected-dynamic.php");
        let span = IrSpan::new(file, 0, 0);
        let function = builder.start_function("jit_bool", FunctionFlags::default(), span);
        builder.set_entry(function);
        let block = builder.append_block(function);
        let value = builder.add_constant(IrConstant::Bool(true));
        builder.terminate_return(function, block, Some(Operand::Constant(value)), span);
        (builder.finish(), function)
    }
}
