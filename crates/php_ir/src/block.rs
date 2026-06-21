//! Basic block IR structure.

use crate::ids::BlockId;
use crate::instruction::{Instruction, Terminator as IrTerminator};
use serde::{Deserialize, Serialize};

/// Public terminator type.
pub type Terminator = IrTerminator;

/// Basic block with a list of instructions and an optional terminator.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BasicBlock {
    /// Block ID within a function.
    pub id: BlockId,
    /// Instructions in block order.
    pub instructions: Vec<Instruction>,
    /// Final control-flow instruction.
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    /// Creates an empty block.
    #[must_use]
    pub const fn new(id: BlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            terminator: None,
        }
    }
}
