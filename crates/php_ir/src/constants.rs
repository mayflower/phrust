//! IR constant pool values.

use serde::{Deserialize, Serialize};

/// Literal constants stored in an IR unit.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum IrConstant {
    /// PHP `null`.
    Null,
    /// PHP boolean.
    Bool(bool),
    /// PHP integer.
    Int(i64),
    /// PHP float.
    Float(f64),
    /// PHP string bytes represented as UTF-8 for the MVP.
    String(String),
    /// PHP string bytes that cannot be represented losslessly as UTF-8.
    StringBytes(Vec<u8>),
}
