//! IR instruction operands.

use crate::ids::{ConstId, LocalId, RegId};
use serde::{Deserialize, Serialize};

/// Operand accepted by IR instructions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Operand {
    /// Register operand.
    Register(RegId),
    /// Local variable slot operand.
    Local(LocalId),
    /// Constant-pool operand.
    Constant(ConstId),
}
