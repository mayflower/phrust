//! Deterministic internal builtin registry for the runtime VM.

mod context;
mod error;
pub(in crate::builtins) mod modules;
mod registry;
mod signatures;

pub use context::{BuiltinContext, RuntimeSourceSpan, StrtokState};
pub use error::BuiltinError;
pub use registry::{BuiltinCompatibility, BuiltinEntry, BuiltinRegistry};
pub use signatures::{BuiltinResult, InternalFunction};
