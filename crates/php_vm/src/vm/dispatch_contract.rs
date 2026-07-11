use super::prelude::*;

pub(super) struct DenseExecutionRequest<'unit, 'call> {
    pub(super) compiled: &'unit CompiledUnit,
    pub(super) dense: &'unit DenseBytecodeUnit,
    pub(super) plan: Option<&'unit DenseExecutionPlan>,
    pub(super) dense_function: &'unit DenseFunction,
    pub(super) ir_function: &'unit IrFunction,
    pub(super) function_id: FunctionId,
    pub(super) call: FunctionCall<'call>,
}

pub(super) struct DenseBinaryRequest<'unit> {
    pub(super) compiled: &'unit CompiledUnit,
    pub(super) unit_id: UnitId,
    pub(super) function_id: FunctionId,
    pub(super) instruction_index: u32,
    pub(super) opcode: DenseOpcode,
    pub(super) dst: u32,
    pub(super) lhs: DenseOperand,
    pub(super) rhs: DenseOperand,
    pub(super) span: IrSpan,
}
