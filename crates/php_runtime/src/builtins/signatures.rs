//! Shared internal builtin signatures.

use super::{BuiltinContext, BuiltinError, RuntimeSourceSpan};
use crate::Value;

/// Result returned by an internal builtin.
pub type BuiltinResult = Result<Value, BuiltinError>;

/// Internal builtin function signature.
pub type InternalFunction =
    fn(&mut BuiltinContext<'_>, Vec<Value>, RuntimeSourceSpan) -> BuiltinResult;
