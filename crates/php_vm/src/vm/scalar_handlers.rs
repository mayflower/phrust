use super::dispatch_contract::DenseBinaryRequest;
use super::prelude::*;

pub(super) fn int_int_specialization_for_binary_op(
    op: BinaryOp,
) -> Option<QuickeningSpecialization> {
    match op {
        BinaryOp::Add => Some(QuickeningSpecialization::AddIntInt),
        BinaryOp::Sub => Some(QuickeningSpecialization::SubIntInt),
        BinaryOp::Mul => Some(QuickeningSpecialization::MulIntInt),
        BinaryOp::Div
        | BinaryOp::Mod
        | BinaryOp::Concat
        | BinaryOp::Pow
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight => None,
    }
}

fn dense_binary_op(opcode: DenseOpcode) -> Option<BinaryOp> {
    match opcode {
        DenseOpcode::BinaryAdd => Some(BinaryOp::Add),
        DenseOpcode::BinarySub => Some(BinaryOp::Sub),
        DenseOpcode::BinaryMul => Some(BinaryOp::Mul),
        DenseOpcode::BinaryDiv => Some(BinaryOp::Div),
        DenseOpcode::BinaryMod => Some(BinaryOp::Mod),
        DenseOpcode::BinaryConcat => Some(BinaryOp::Concat),
        DenseOpcode::BinaryPow => Some(BinaryOp::Pow),
        DenseOpcode::BinaryBitAnd => Some(BinaryOp::BitAnd),
        DenseOpcode::BinaryBitOr => Some(BinaryOp::BitOr),
        DenseOpcode::BinaryBitXor => Some(BinaryOp::BitXor),
        DenseOpcode::BinaryShiftLeft => Some(BinaryOp::ShiftLeft),
        DenseOpcode::BinaryShiftRight => Some(BinaryOp::ShiftRight),
        _ => None,
    }
}

pub(super) fn checked_int_binary(op: BinaryOp, lhs: i64, rhs: i64) -> Option<i64> {
    match op {
        BinaryOp::Add => lhs.checked_add(rhs),
        BinaryOp::Sub => lhs.checked_sub(rhs),
        BinaryOp::Mul => lhs.checked_mul(rhs),
        BinaryOp::Div
        | BinaryOp::Mod
        | BinaryOp::Concat
        | BinaryOp::Pow
        | BinaryOp::BitAnd
        | BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight => None,
    }
}

fn dense_compare_op(opcode: DenseOpcode) -> Option<CompareOp> {
    match opcode {
        DenseOpcode::CompareEqual => Some(CompareOp::Equal),
        DenseOpcode::CompareNotEqual => Some(CompareOp::NotEqual),
        DenseOpcode::CompareIdentical => Some(CompareOp::Identical),
        DenseOpcode::CompareNotIdentical => Some(CompareOp::NotIdentical),
        DenseOpcode::CompareLess => Some(CompareOp::Less),
        DenseOpcode::CompareLessEqual => Some(CompareOp::LessEqual),
        DenseOpcode::CompareGreater => Some(CompareOp::Greater),
        DenseOpcode::CompareGreaterEqual => Some(CompareOp::GreaterEqual),
        DenseOpcode::CompareSpaceship => Some(CompareOp::Spaceship),
        _ => None,
    }
}

fn dense_unary_op(opcode: DenseOpcode) -> Option<UnaryOp> {
    match opcode {
        DenseOpcode::UnaryPlus => Some(UnaryOp::Plus),
        DenseOpcode::UnaryMinus => Some(UnaryOp::Minus),
        DenseOpcode::UnaryNot => Some(UnaryOp::Not),
        DenseOpcode::UnaryBitNot => Some(UnaryOp::BitNot),
        _ => None,
    }
}

pub(super) fn execute_arithmetic(
    op: BinaryOp,
    lhs: NumericValue,
    rhs: NumericValue,
) -> Result<Value, String> {
    native_binary_result(op, &numeric_value(lhs), &numeric_value(rhs))
}

pub(super) fn execute_power(lhs: NumericValue, rhs: NumericValue) -> Result<Value, String> {
    native_binary_result(BinaryOp::Pow, &numeric_value(lhs), &numeric_value(rhs))
}

pub(super) fn execute_bitwise(op: BinaryOp, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    native_binary_result(op, lhs, rhs)
}

fn numeric_value(value: NumericValue) -> Value {
    match value {
        NumericValue::Int(value) => Value::Int(value),
        NumericValue::Float(value) => Value::float(value),
    }
}

fn native_binary_op(op: BinaryOp) -> NativeBinaryOp {
    match op {
        BinaryOp::Add => NativeBinaryOp::Add,
        BinaryOp::Sub => NativeBinaryOp::Sub,
        BinaryOp::Mul => NativeBinaryOp::Mul,
        BinaryOp::Div => NativeBinaryOp::Div,
        BinaryOp::Mod => NativeBinaryOp::Mod,
        BinaryOp::Concat => NativeBinaryOp::Concat,
        BinaryOp::Pow => NativeBinaryOp::Pow,
        BinaryOp::BitAnd => NativeBinaryOp::BitAnd,
        BinaryOp::BitOr => NativeBinaryOp::BitOr,
        BinaryOp::BitXor => NativeBinaryOp::BitXor,
        BinaryOp::ShiftLeft => NativeBinaryOp::ShiftLeft,
        BinaryOp::ShiftRight => NativeBinaryOp::ShiftRight,
    }
}

fn native_binary_result(op: BinaryOp, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    let mut context = NativeOperationContext::default();
    let mut out = Value::Uninitialized;
    match native_binary(&mut context, native_binary_op(op), lhs, rhs, &mut out) {
        NativeOperationStatus::Ok => Ok(out),
        NativeOperationStatus::RuntimeError
        | NativeOperationStatus::Throw
        | NativeOperationStatus::CallUserland
        | NativeOperationStatus::Suspend
        | NativeOperationStatus::Unsupported => Err(context
            .message
            .unwrap_or_else(|| "native binary operation failed".to_owned())),
    }
}

/// Deprecation text for a float (or float-string) used in an int-only
/// context when the conversion loses precision; integral in-range floats
/// convert silently.
pub(super) fn implicit_int_deprecation_message(value: &Value) -> Option<String> {
    match effective_value(value) {
        Value::Float(float_value) => {
            let raw = float_value.to_f64();
            let lossless = php_runtime::api::float_fits_int(raw) && raw.trunc() == raw;
            (!lossless).then(|| {
                let rendered = to_string(&Value::float(raw))
                    .map(|text| text.to_string_lossy())
                    .unwrap_or_else(|_| raw.to_string());
                format!("Implicit conversion from float {rendered} to int loses precision")
            })
        }
        Value::String(text) => {
            let Ok(NumericValue::Float(raw)) = to_number(&Value::String(text.clone())) else {
                return None;
            };
            let lossless = php_runtime::api::float_fits_int(raw) && raw.trunc() == raw;
            (!lossless).then(|| {
                format!(
                    "Implicit conversion from float-string \"{}\" to int loses precision",
                    text.to_string_lossy()
                )
            })
        }
        _ => None,
    }
}

/// Executes a unary operator; the second tuple slot carries a pending
/// implicit-int-conversion deprecation for the stateful caller to emit.
fn execute_unary(op: UnaryOp, src: &Value) -> Result<(Value, Option<String>), String> {
    let native_op = match op {
        UnaryOp::Plus => NativeUnaryOp::Plus,
        UnaryOp::Minus => NativeUnaryOp::Minus,
        UnaryOp::Not => NativeUnaryOp::Not,
        UnaryOp::BitNot => NativeUnaryOp::BitNot,
    };
    let deprecation = matches!(op, UnaryOp::BitNot)
        .then(|| implicit_int_deprecation_message(src))
        .flatten();
    let mut context = NativeOperationContext::default();
    let mut out = Value::Uninitialized;
    match native_unary(&mut context, native_op, src, &mut out) {
        NativeOperationStatus::Ok => Ok((out, deprecation)),
        NativeOperationStatus::RuntimeError
        | NativeOperationStatus::Throw
        | NativeOperationStatus::CallUserland
        | NativeOperationStatus::Suspend
        | NativeOperationStatus::Unsupported => Err(context
            .message
            .unwrap_or_else(|| "native unary operation failed".to_owned())),
    }
}

fn execute_compare(op: CompareOp, lhs: &Value, rhs: &Value) -> Result<Value, String> {
    let native_op = match op {
        CompareOp::Equal => NativeCompareOp::Equal,
        CompareOp::NotEqual => NativeCompareOp::NotEqual,
        CompareOp::Identical => NativeCompareOp::Identical,
        CompareOp::NotIdentical => NativeCompareOp::NotIdentical,
        CompareOp::Less => NativeCompareOp::Less,
        CompareOp::LessEqual => NativeCompareOp::LessEqual,
        CompareOp::Greater => NativeCompareOp::Greater,
        CompareOp::GreaterEqual => NativeCompareOp::GreaterEqual,
        CompareOp::Spaceship => NativeCompareOp::Spaceship,
    };
    let mut context = NativeOperationContext::default();
    let mut out = Value::Uninitialized;
    match native_compare(&mut context, native_op, lhs, rhs, &mut out) {
        NativeOperationStatus::Ok => Ok(out),
        NativeOperationStatus::RuntimeError
        | NativeOperationStatus::Throw
        | NativeOperationStatus::CallUserland
        | NativeOperationStatus::Suspend
        | NativeOperationStatus::Unsupported => Err(context
            .message
            .unwrap_or_else(|| "native compare operation failed".to_owned())),
    }
}

pub(super) fn execute_rich_compare_op(
    request: RichCompareRequest<'_>,
    stack: &mut CallStack,
) -> Result<(), String> {
    let RichCompareRequest {
        unit,
        frame_index,
        dst,
        op,
        lhs,
        rhs,
    } = request;
    let lhs = read_operand_at_frame(unit, stack, frame_index, lhs)?;
    let rhs = read_operand_at_frame(unit, stack, frame_index, rhs)?;
    let value = execute_compare(op, &lhs, &rhs)?;
    stack
        .frame_mut(frame_index)
        .expect("frame was pushed")
        .registers
        .set(dst, value)?;
    Ok(())
}

pub(super) fn execute_rich_binary_op(
    vm: &Vm,
    request: RichBinaryRequest<'_>,
    output: &mut OutputBuffer,
    stack: &mut CallStack,
    state: &mut ExecutionState,
) -> Result<(), RichBinaryError> {
    let RichBinaryRequest {
        compiled,
        unit,
        frame_index,
        function_id,
        block_id,
        instruction_id,
        dst,
        op,
        lhs,
        rhs,
        span,
    } = request;
    let lhs = read_operand_at_frame(unit, stack, frame_index, lhs).map_err(|message| {
        RichBinaryError::Direct(Box::new(vm.runtime_error(output, compiled, stack, message)))
    })?;
    let rhs = read_operand_at_frame(unit, stack, frame_index, rhs).map_err(|message| {
        RichBinaryError::Direct(Box::new(vm.runtime_error(output, compiled, stack, message)))
    })?;
    let value = match op {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
            vm.try_quickened_int_int_binary(function_id, block_id, instruction_id, op, &lhs, &rhs)
        }
        BinaryOp::Concat => {
            vm.try_quickened_concat_string_string(function_id, block_id, instruction_id, &lhs, &rhs)
        }
        _ => None,
    };
    let value = match value {
        Some(value) => value,
        None => vm
            .execute_binary(
                ExecutionCursor::new(compiled, output, stack, state),
                op,
                &lhs,
                &rhs,
                runtime_source_span(compiled, span),
            )
            .map_err(|result| RichBinaryError::Route(Box::new(*result)))?,
    };
    stack
        .frame_mut(frame_index)
        .expect("frame was pushed")
        .registers
        .set(dst, value)
        .map_err(|message| {
            RichBinaryError::Direct(Box::new(vm.runtime_error(output, compiled, stack, message)))
        })?;
    Ok(())
}

pub(super) fn execute_rich_unary_op(
    request: RichUnaryRequest<'_>,
    stack: &mut CallStack,
) -> Result<Option<String>, String> {
    let RichUnaryRequest {
        unit,
        frame_index,
        dst,
        op,
        src,
    } = request;
    let src = read_operand_at_frame(unit, stack, frame_index, src)?;
    let (value, deprecation) = execute_unary(op, &src)?;
    stack
        .frame_mut(frame_index)
        .expect("frame was pushed")
        .registers
        .set(dst, value)?;
    Ok(deprecation)
}

impl Vm {
    pub(super) fn execute_dense_binary_op(
        &self,
        request: DenseBinaryRequest<'_>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<(), Box<VmResult>> {
        let DenseBinaryRequest {
            compiled,
            unit_id,
            function_id,
            instruction_index,
            opcode,
            dst,
            lhs,
            rhs,
            span,
        } = request;
        let lhs = self
            .read_dense_operand_ref(compiled, stack, lhs)
            .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
        let rhs = self
            .read_dense_operand_ref(compiled, stack, rhs)
            .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
        let op = dense_binary_op(opcode).expect("dense binary opcode matched");
        let value = match op {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => self
                .try_quickened_dense_int_int_binary(
                    unit_id,
                    function_id,
                    instruction_index,
                    op,
                    lhs.as_value(),
                    rhs.as_value(),
                ),
            BinaryOp::Concat => self.try_quickened_dense_concat_string_string(
                unit_id,
                function_id,
                instruction_index,
                lhs.as_value(),
                rhs.as_value(),
            ),
            _ => None,
        };
        let value = match value {
            Some(value) => value,
            None => {
                let lhs = lhs.into_owned();
                let rhs = rhs.into_owned();
                self.execute_binary(
                    ExecutionCursor::new(compiled, output, stack, state),
                    op,
                    &lhs,
                    &rhs,
                    runtime_source_span(compiled, span),
                )?
            }
        };
        stack
            .current_mut()
            .expect("bytecode frame was pushed")
            .registers
            .set(RegId::new(dst), value)
            .map_err(|message| self.runtime_error(output, compiled, stack, message))?;
        Ok(())
    }

    pub(super) fn execute_dense_compare_op(
        &self,
        compiled: &CompiledUnit,
        stack: &mut CallStack,
        opcode: DenseOpcode,
        dst: u32,
        lhs: DenseOperand,
        rhs: DenseOperand,
    ) -> Result<(), String> {
        let lhs = self.read_dense_operand_ref(compiled, stack, lhs)?;
        let rhs = self.read_dense_operand_ref(compiled, stack, rhs)?;
        let op = dense_compare_op(opcode).expect("dense compare opcode matched");
        let value = execute_compare(op, lhs.as_value(), rhs.as_value())?;
        stack
            .current_mut()
            .expect("bytecode frame was pushed")
            .registers
            .set(RegId::new(dst), value)?;
        Ok(())
    }

    pub(super) fn execute_dense_unary_op(
        &self,
        compiled: &CompiledUnit,
        stack: &mut CallStack,
        opcode: DenseOpcode,
        dst: u32,
        src: DenseOperand,
    ) -> Result<Option<String>, String> {
        let src = self.read_dense_operand_ref(compiled, stack, src)?;
        let op = dense_unary_op(opcode).expect("dense unary opcode matched");
        let (value, deprecation) = execute_unary(op, src.as_value())?;
        stack
            .current_mut()
            .expect("bytecode frame was pushed")
            .registers
            .set(RegId::new(dst), value)?;
        Ok(deprecation)
    }
}
