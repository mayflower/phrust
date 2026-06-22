//! IR invariant verifier.

use crate::block::BasicBlock;
use crate::function::IrFunction;
use crate::ids::{BlockId, ConstId, LocalId, RegId};
use crate::instruction::{Instruction, InstructionKind, Terminator, TerminatorKind};
use crate::module::{IR_VERSION, IrUnit};
use crate::operand::Operand;
use crate::source_map::IrSpan;
use serde::{Deserialize, Serialize};

/// Stable verifier error code.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationErrorCode {
    /// Unit version is not supported.
    InvalidVersion,
    /// Entry function points outside the function table.
    InvalidEntryFunction,
    /// File table ID does not match its position.
    InvalidFileId,
    /// Class table ID does not match its position.
    InvalidClassId,
    /// A span references an unknown file or has an invalid range.
    InvalidSpan,
    /// Block ID does not match its position.
    InvalidBlockId,
    /// Instruction ID does not match its position.
    InvalidInstrId,
    /// Operand or destination register is outside the function register range.
    InvalidRegId,
    /// Operand or parameter local is outside the function local range.
    InvalidLocalId,
    /// Constant ID points outside the constant pool.
    InvalidConstId,
    /// Function lookup table entry points outside the function table.
    InvalidFunctionId,
    /// Function lookup table contains a duplicate normalized name.
    DuplicateFunctionName,
    /// Constant lookup table contains a duplicate name.
    DuplicateConstantName,
    /// Terminator target points outside the function block table.
    InvalidBlockTarget,
    /// Basic block is missing a terminator.
    MissingTerminator,
}

/// One IR verifier error.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerificationError {
    /// Stable error code.
    pub code: VerificationErrorCode,
    /// Human-readable context.
    pub message: String,
}

/// Verifies basic IR invariants.
pub fn verify_unit(unit: &IrUnit) -> Result<(), Vec<VerificationError>> {
    let mut errors = Vec::new();

    if unit.version != IR_VERSION {
        errors.push(error(
            VerificationErrorCode::InvalidVersion,
            format!("unsupported IR version {}", unit.version),
        ));
    }
    if unit.entry.index() >= unit.functions.len() {
        errors.push(error(
            VerificationErrorCode::InvalidEntryFunction,
            format!("entry function {} is not defined", unit.entry.raw()),
        ));
    }
    for (index, file) in unit.files.iter().enumerate() {
        if file.id.index() != index {
            errors.push(error(
                VerificationErrorCode::InvalidFileId,
                format!("file table entry {index} has id {}", file.id.raw()),
            ));
        }
    }
    for (index, class) in unit.classes.iter().enumerate() {
        if class.id.index() != index {
            errors.push(error(
                VerificationErrorCode::InvalidClassId,
                format!("class table entry {index} has id {}", class.id.raw()),
            ));
        }
        verify_span(unit, class.span, &mut errors);
        for method in &class.methods {
            verify_function_id(method.function, unit.functions.len(), &mut errors);
        }
        for property in &class.properties {
            if let Some(default) = property.default {
                verify_constant(default, unit.constants.len(), &mut errors);
            }
        }
        if let Some(constructor) = class.constructor {
            verify_function_id(constructor, unit.functions.len(), &mut errors);
        }
    }
    for entry in &unit.function_table {
        if entry.function.index() >= unit.functions.len() {
            errors.push(error(
                VerificationErrorCode::InvalidFunctionId,
                format!(
                    "function table entry {:?} points at missing function {}",
                    entry.name,
                    entry.function.raw()
                ),
            ));
        }
        if unit
            .function_table
            .iter()
            .filter(|other| other.name == entry.name)
            .count()
            > 1
        {
            errors.push(error(
                VerificationErrorCode::DuplicateFunctionName,
                format!("function table contains duplicate name {:?}", entry.name),
            ));
        }
    }
    for entry in &unit.constant_table {
        verify_constant(entry.value, unit.constants.len(), &mut errors);
        verify_span(unit, entry.span, &mut errors);
        if unit
            .constant_table
            .iter()
            .filter(|other| other.name == entry.name)
            .count()
            > 1
        {
            errors.push(error(
                VerificationErrorCode::DuplicateConstantName,
                format!("constant table contains duplicate name {:?}", entry.name),
            ));
        }
    }
    for function in &unit.functions {
        verify_function(unit, function, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn verify_function(unit: &IrUnit, function: &IrFunction, errors: &mut Vec<VerificationError>) {
    verify_span(unit, function.span, errors);
    if function.locals.len() != function.local_count as usize {
        errors.push(error(
            VerificationErrorCode::InvalidLocalId,
            format!(
                "function has {} local names but local_count is {}",
                function.locals.len(),
                function.local_count
            ),
        ));
    }
    for param in &function.params {
        verify_local(param.local, function.local_count, errors);
    }
    for capture in &function.captures {
        verify_local(capture.local, function.local_count, errors);
    }
    for (index, block) in function.blocks.iter().enumerate() {
        verify_block_id(block.id, index, errors);
        verify_block(unit, function, block, errors);
    }
}

fn verify_block(
    unit: &IrUnit,
    function: &IrFunction,
    block: &BasicBlock,
    errors: &mut Vec<VerificationError>,
) {
    for (index, instruction) in block.instructions.iter().enumerate() {
        if instruction.id.index() != index {
            errors.push(error(
                VerificationErrorCode::InvalidInstrId,
                format!(
                    "block {} instruction {index} has id {}",
                    block.id.raw(),
                    instruction.id.raw()
                ),
            ));
        }
        verify_instruction(unit, function, instruction, errors);
    }
    match &block.terminator {
        Some(terminator) => verify_terminator(unit, function, terminator, errors),
        None => errors.push(error(
            VerificationErrorCode::MissingTerminator,
            format!("block {} has no terminator", block.id.raw()),
        )),
    }
}

fn verify_instruction(
    unit: &IrUnit,
    function: &IrFunction,
    instruction: &Instruction,
    errors: &mut Vec<VerificationError>,
) {
    verify_span(unit, instruction.span, errors);
    match &instruction.kind {
        InstructionKind::Nop
        | InstructionKind::Unsupported { .. }
        | InstructionKind::RuntimeError { .. } => {}
        InstructionKind::LoadConst { dst, constant } => {
            verify_register(*dst, function.register_count, errors);
            verify_constant(*constant, unit.constants.len(), errors);
        }
        InstructionKind::FetchConst { dst, .. } => {
            verify_register(*dst, function.register_count, errors);
        }
        InstructionKind::Move { dst, src } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(src, function, unit, errors);
        }
        InstructionKind::LoadLocal { dst, local }
        | InstructionKind::LoadLocalQuiet { dst, local } => {
            verify_register(*dst, function.register_count, errors);
            verify_local(*local, function.local_count, errors);
        }
        InstructionKind::StoreLocal { local, src } => {
            verify_local(*local, function.local_count, errors);
            verify_operand(src, function, unit, errors);
        }
        InstructionKind::BindReference { target, source } => {
            verify_local(*target, function.local_count, errors);
            verify_local(*source, function.local_count, errors);
        }
        InstructionKind::BindGlobal { local, .. } => {
            verify_local(*local, function.local_count, errors);
        }
        InstructionKind::BindReferenceDim {
            local,
            dims,
            source,
            ..
        } => {
            verify_local(*local, function.local_count, errors);
            verify_local(*source, function.local_count, errors);
            for dim in dims {
                verify_operand(dim, function, unit, errors);
            }
        }
        InstructionKind::BindReferenceFromDim {
            target,
            local,
            dims,
        } => {
            verify_local(*target, function.local_count, errors);
            verify_local(*local, function.local_count, errors);
            for dim in dims {
                verify_operand(dim, function, unit, errors);
            }
        }
        InstructionKind::InitStaticLocal { local, default, .. } => {
            verify_local(*local, function.local_count, errors);
            verify_operand(default, function, unit, errors);
        }
        InstructionKind::Binary { dst, lhs, rhs, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(lhs, function, unit, errors);
            verify_operand(rhs, function, unit, errors);
        }
        InstructionKind::Compare { dst, lhs, rhs, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(lhs, function, unit, errors);
            verify_operand(rhs, function, unit, errors);
        }
        InstructionKind::InstanceOf { dst, object, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
        }
        InstructionKind::Unary { dst, src, .. } | InstructionKind::Cast { dst, src, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(src, function, unit, errors);
        }
        InstructionKind::Discard { src } => verify_operand(src, function, unit, errors),
        InstructionKind::Echo { src } => verify_operand(src, function, unit, errors),
        InstructionKind::Yield { dst, key, value } => {
            verify_register(*dst, function.register_count, errors);
            if let Some(key) = key {
                verify_operand(key, function, unit, errors);
            }
            if let Some(value) = value {
                verify_operand(value, function, unit, errors);
            }
        }
        InstructionKind::YieldFrom { dst, source } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(source, function, unit, errors);
        }
        InstructionKind::BindReferenceFromCall { target, args, .. } => {
            verify_local(*target, function.local_count, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::CallFunction { dst, args, .. } => {
            verify_register(*dst, function.register_count, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::CallMethod {
            dst, object, args, ..
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::CallStaticMethod { dst, args, .. } => {
            verify_register(*dst, function.register_count, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::CloneObject { dst, object } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
        }
        InstructionKind::CloneWith {
            dst,
            object,
            replacements,
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
            verify_operand(replacements, function, unit, errors);
        }
        InstructionKind::EnterTry {
            catch,
            catch_types: _,
            finally,
            after,
            exception_local,
        } => {
            if let Some(catch) = catch {
                verify_block_target(*catch, function, errors);
            }
            if let Some(finally) = finally {
                verify_block_target(*finally, function, errors);
            }
            verify_block_target(*after, function, errors);
            if let Some(local) = exception_local {
                verify_local(*local, function.local_count, errors);
            }
        }
        InstructionKind::LeaveTry => {}
        InstructionKind::EndFinally { after } => verify_block_target(*after, function, errors),
        InstructionKind::Throw { value } => verify_operand(value, function, unit, errors),
        InstructionKind::MakeException {
            dst,
            class_name: _,
            message,
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(message, function, unit, errors);
        }
        InstructionKind::MakeClosure {
            dst,
            function: closure_function,
            captures,
        } => {
            verify_register(*dst, function.register_count, errors);
            if closure_function.index() >= unit.functions.len() {
                errors.push(error(
                    VerificationErrorCode::InvalidFunctionId,
                    format!(
                        "make_closure points at missing function {}",
                        closure_function.raw()
                    ),
                ));
            }
            for capture in captures {
                verify_operand(&capture.src, function, unit, errors);
            }
        }
        InstructionKind::CallClosure { dst, callee, args } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(callee, function, unit, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::ResolveCallable { dst, .. } => {
            verify_register(*dst, function.register_count, errors);
        }
        InstructionKind::CallCallable { dst, callee, args } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(callee, function, unit, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::Pipe {
            dst,
            input,
            callable,
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(input, function, unit, errors);
            verify_operand(callable, function, unit, errors);
        }
        InstructionKind::Include { dst, path, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(path, function, unit, errors);
        }
        InstructionKind::Eval { dst, code } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(code, function, unit, errors);
        }
        InstructionKind::NewObject { dst, args, .. } => {
            verify_register(*dst, function.register_count, errors);
            for arg in args {
                verify_operand(&arg.value, function, unit, errors);
                if let Some(local) = arg.by_ref_local {
                    verify_local(local, function.local_count, errors);
                }
            }
        }
        InstructionKind::FetchProperty { dst, object, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
        }
        InstructionKind::IssetProperty { dst, object, .. }
        | InstructionKind::EmptyProperty { dst, object, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
        }
        InstructionKind::UnsetProperty { object, .. } => {
            verify_operand(object, function, unit, errors);
        }
        InstructionKind::FetchStaticProperty { dst, .. }
        | InstructionKind::FetchClassConstant { dst, .. } => {
            verify_register(*dst, function.register_count, errors);
        }
        InstructionKind::AssignProperty {
            dst, object, value, ..
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(object, function, unit, errors);
            verify_operand(value, function, unit, errors);
        }
        InstructionKind::AssignStaticProperty { dst, value, .. } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(value, function, unit, errors);
        }
        InstructionKind::NewArray { dst } => {
            verify_register(*dst, function.register_count, errors);
        }
        InstructionKind::ArrayInsert { array, key, value } => {
            verify_register(*array, function.register_count, errors);
            if let Some(key) = key {
                verify_operand(key, function, unit, errors);
            }
            verify_operand(value, function, unit, errors);
        }
        InstructionKind::FetchDim {
            dst, array, key, ..
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(array, function, unit, errors);
            verify_operand(key, function, unit, errors);
        }
        InstructionKind::AssignDim {
            dst,
            local,
            dims,
            value,
        }
        | InstructionKind::AppendDim {
            dst,
            local,
            dims,
            value,
        } => {
            verify_register(*dst, function.register_count, errors);
            verify_local(*local, function.local_count, errors);
            for dim in dims {
                verify_operand(dim, function, unit, errors);
            }
            verify_operand(value, function, unit, errors);
        }
        InstructionKind::IssetLocal { dst, local } | InstructionKind::EmptyLocal { dst, local } => {
            verify_register(*dst, function.register_count, errors);
            verify_local(*local, function.local_count, errors);
        }
        InstructionKind::UnsetLocal { local } => {
            verify_local(*local, function.local_count, errors);
        }
        InstructionKind::IssetDim { dst, local, dims }
        | InstructionKind::EmptyDim { dst, local, dims } => {
            verify_register(*dst, function.register_count, errors);
            verify_local(*local, function.local_count, errors);
            for dim in dims {
                verify_operand(dim, function, unit, errors);
            }
        }
        InstructionKind::UnsetDim { local, dims } => {
            verify_local(*local, function.local_count, errors);
            for dim in dims {
                verify_operand(dim, function, unit, errors);
            }
        }
        InstructionKind::ForeachInit { iterator, source } => {
            verify_register(*iterator, function.register_count, errors);
            verify_operand(source, function, unit, errors);
        }
        InstructionKind::ForeachNext {
            has_value,
            iterator,
            key,
            value,
        } => {
            verify_register(*has_value, function.register_count, errors);
            verify_register(*iterator, function.register_count, errors);
            if let Some(key) = key {
                verify_register(*key, function.register_count, errors);
            }
            verify_register(*value, function.register_count, errors);
        }
        InstructionKind::ForeachInitRef { iterator, local } => {
            verify_register(*iterator, function.register_count, errors);
            verify_local(*local, function.local_count, errors);
        }
        InstructionKind::ForeachNextRef {
            has_value,
            iterator,
            key,
            value_local,
        } => {
            verify_register(*has_value, function.register_count, errors);
            verify_register(*iterator, function.register_count, errors);
            if let Some(key) = key {
                verify_register(*key, function.register_count, errors);
            }
            verify_local(*value_local, function.local_count, errors);
        }
        InstructionKind::ArrayGet { dst, array, index } => {
            verify_register(*dst, function.register_count, errors);
            verify_operand(array, function, unit, errors);
            verify_operand(index, function, unit, errors);
        }
    }
}

fn verify_terminator(
    unit: &IrUnit,
    function: &IrFunction,
    terminator: &Terminator,
    errors: &mut Vec<VerificationError>,
) {
    verify_span(unit, terminator.span, errors);
    match &terminator.kind {
        TerminatorKind::Jump { target } => verify_block_target(*target, function, errors),
        TerminatorKind::JumpIfFalse { condition, target }
        | TerminatorKind::JumpIfTrue { condition, target } => {
            verify_operand(condition, function, unit, errors);
            verify_block_target(*target, function, errors);
        }
        TerminatorKind::JumpIf {
            condition,
            if_true,
            if_false,
        } => {
            verify_operand(condition, function, unit, errors);
            verify_block_target(*if_true, function, errors);
            verify_block_target(*if_false, function, errors);
        }
        TerminatorKind::Return {
            value,
            by_ref_local,
        } => {
            if let Some(value) = value {
                verify_operand(value, function, unit, errors);
            }
            if let Some(local) = by_ref_local {
                verify_local(*local, function.local_count, errors);
            }
        }
    }
}

fn verify_span(unit: &IrUnit, span: IrSpan, errors: &mut Vec<VerificationError>) {
    if span.start > span.end || span.file.index() >= unit.files.len() {
        errors.push(error(
            VerificationErrorCode::InvalidSpan,
            format!(
                "span file {} range {}..{} is invalid",
                span.file.raw(),
                span.start,
                span.end
            ),
        ));
    }
}

fn verify_operand(
    operand: &Operand,
    function: &IrFunction,
    unit: &IrUnit,
    errors: &mut Vec<VerificationError>,
) {
    match operand {
        Operand::Register(id) => verify_register(*id, function.register_count, errors),
        Operand::Local(id) => verify_local(*id, function.local_count, errors),
        Operand::Constant(id) => verify_constant(*id, unit.constants.len(), errors),
    }
}

fn verify_block_id(id: BlockId, expected: usize, errors: &mut Vec<VerificationError>) {
    if id.index() != expected {
        errors.push(error(
            VerificationErrorCode::InvalidBlockId,
            format!("block table entry {expected} has id {}", id.raw()),
        ));
    }
}

fn verify_block_target(id: BlockId, function: &IrFunction, errors: &mut Vec<VerificationError>) {
    if id.index() >= function.blocks.len() {
        errors.push(error(
            VerificationErrorCode::InvalidBlockTarget,
            format!("target block {} is not defined", id.raw()),
        ));
    }
}

fn verify_register(id: RegId, register_count: u32, errors: &mut Vec<VerificationError>) {
    if id.raw() >= register_count {
        errors.push(error(
            VerificationErrorCode::InvalidRegId,
            format!("register {} exceeds count {register_count}", id.raw()),
        ));
    }
}

fn verify_function_id(
    id: crate::ids::FunctionId,
    function_count: usize,
    errors: &mut Vec<VerificationError>,
) {
    if id.index() >= function_count {
        errors.push(error(
            VerificationErrorCode::InvalidFunctionId,
            format!("function {} is not defined", id.raw()),
        ));
    }
}

fn verify_local(id: LocalId, local_count: u32, errors: &mut Vec<VerificationError>) {
    if id.raw() >= local_count {
        errors.push(error(
            VerificationErrorCode::InvalidLocalId,
            format!("local {} exceeds count {local_count}", id.raw()),
        ));
    }
}

fn verify_constant(id: ConstId, constant_count: usize, errors: &mut Vec<VerificationError>) {
    if id.index() >= constant_count {
        errors.push(error(
            VerificationErrorCode::InvalidConstId,
            format!("constant {} is not defined", id.raw()),
        ));
    }
}

fn error(code: VerificationErrorCode, message: String) -> VerificationError {
    VerificationError { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::IrBuilder;
    use crate::constants::IrConstant;
    use crate::function::FunctionFlags;
    use crate::ids::{FileId, FunctionId, InstrId, UnitId};
    use crate::instruction::InstructionKind;

    #[test]
    fn verifier_accepts_basic_unit() {
        let unit = valid_unit();
        verify_unit(&unit).expect("valid unit should verify");
    }

    #[test]
    fn verifier_rejects_missing_terminator() {
        let mut unit = valid_unit();
        unit.functions[0].blocks[0].terminator = None;
        assert_has_error(&unit, VerificationErrorCode::MissingTerminator);
    }

    #[test]
    fn verifier_rejects_invalid_const_register_block_and_span() {
        let mut unit = valid_unit();
        unit.functions[0].blocks[0].instructions[0].kind = InstructionKind::LoadConst {
            dst: RegId::new(99),
            constant: ConstId::new(99),
        };
        unit.functions[0].blocks[0].instructions[0].span = IrSpan::new(FileId::new(99), 7, 1);
        unit.functions[0].blocks[0].terminator = Some(Terminator {
            span: IrSpan::new(FileId::new(0), 0, 1),
            kind: TerminatorKind::Jump {
                target: BlockId::new(99),
            },
        });
        let errors = verify_unit(&unit).expect_err("unit should fail verification");
        assert!(
            errors
                .iter()
                .any(|error| error.code == VerificationErrorCode::InvalidRegId)
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == VerificationErrorCode::InvalidConstId)
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == VerificationErrorCode::InvalidSpan)
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == VerificationErrorCode::InvalidBlockTarget)
        );
    }

    #[test]
    fn verifier_rejects_invalid_entry_and_local() {
        let mut unit = valid_unit();
        unit.entry = FunctionId::new(99);
        unit.functions[0].blocks[0].instructions.push(Instruction {
            id: InstrId::new(1),
            span: IrSpan::new(FileId::new(0), 0, 1),
            kind: InstructionKind::StoreLocal {
                local: LocalId::new(99),
                src: Operand::Register(RegId::new(0)),
            },
        });
        let errors = verify_unit(&unit).expect_err("unit should fail verification");
        assert!(
            errors
                .iter()
                .any(|error| { error.code == VerificationErrorCode::InvalidEntryFunction })
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == VerificationErrorCode::InvalidLocalId)
        );
    }

    fn assert_has_error(unit: &IrUnit, code: VerificationErrorCode) {
        let errors = verify_unit(unit).expect_err("unit should fail verification");
        assert!(errors.iter().any(|error| error.code == code), "{errors:#?}");
    }

    fn valid_unit() -> IrUnit {
        let mut builder = IrBuilder::new(UnitId::new(0));
        let file = builder.add_file("valid.php");
        let function = builder.start_function(
            "main",
            FunctionFlags {
                is_top_level: true,
                ..FunctionFlags::default()
            },
            IrSpan::new(file, 0, 5),
        );
        let block = builder.append_block(function);
        let constant = builder.add_constant(IrConstant::Int(1));
        let register = builder.alloc_register(function);
        builder.emit_load_const(function, block, register, constant, IrSpan::new(file, 6, 7));
        builder.terminate_return(
            function,
            block,
            Some(Operand::Register(register)),
            IrSpan::new(file, 6, 7),
        );
        builder.set_entry(function);
        builder.finish()
    }
}
