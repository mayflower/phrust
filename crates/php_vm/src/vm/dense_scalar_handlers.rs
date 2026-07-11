use super::dispatch_contract::DenseBinaryRequest;
use super::prelude::*;

impl Vm {
    pub(super) fn execute_dense_binary_op(
        &self,
        request: DenseBinaryRequest<'_>,
        output: &mut OutputBuffer,
        stack: &mut CallStack,
        state: &mut ExecutionState,
    ) -> Result<(), VmResult> {
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
                    compiled,
                    op,
                    &lhs,
                    &rhs,
                    runtime_source_span(compiled, span),
                    output,
                    stack,
                    state,
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
    ) -> Result<(), String> {
        let src = self.read_dense_operand_ref(compiled, stack, src)?;
        let op = dense_unary_op(opcode).expect("dense unary opcode matched");
        let value = execute_unary(op, src.as_value())?;
        stack
            .current_mut()
            .expect("bytecode frame was pushed")
            .registers
            .set(RegId::new(dst), value)?;
        Ok(())
    }
}
