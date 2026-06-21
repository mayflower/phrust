//! IR functions and local/register metadata.

use crate::block::BasicBlock;
use crate::constants::IrConstant;
use crate::ids::LocalId;
use crate::source_map::IrSpan;
use serde::{Deserialize, Serialize};

/// Minimal runtime type family enforced by the Prompt 30 VM.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IrReturnType {
    /// `int`
    Int,
    /// `float`
    Float,
    /// `string`
    String,
    /// `array`
    Array,
    /// `callable`
    Callable,
    /// `object`
    Object,
    /// `bool`
    Bool,
    /// `null`
    Null,
    /// `void`
    Void,
    /// `mixed`
    Mixed,
    /// Class-like return type. Runtime object checking is a known gap until
    /// object storage exists.
    Class { name: String },
    /// Nullable simple type from `?T` or normalized `T|null`.
    Nullable { inner: Box<IrReturnType> },
}

/// Function parameter metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IrParam {
    /// Parameter name without `$`.
    pub name: String,
    /// Local slot assigned to the parameter.
    pub local: LocalId,
    /// True when callers must pass this positional argument.
    pub required: bool,
    /// Constant-pool default value for omitted optional arguments.
    pub default: Option<IrConstant>,
    /// Optional Phase-3 lowered runtime type enforced by the VM MVP.
    pub type_: Option<IrReturnType>,
    /// By-reference scaffold bit. The VM rejects by-ref parameters until
    /// references exist.
    pub by_ref: bool,
    /// True when this parameter collects remaining positional arguments.
    pub variadic: bool,
}

/// Closure capture metadata stored on a synthesized closure function.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrCapture {
    /// Captured variable name without `$`.
    pub name: String,
    /// Local slot initialized from the closure value before parameters.
    pub local: LocalId,
    /// By-reference capture scaffold bit. The VM rejects this until references
    /// exist instead of silently copying the value.
    pub by_ref: bool,
}

/// Function shape flags.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FunctionFlags {
    /// True for the synthesized top-level script function.
    pub is_top_level: bool,
    /// True for closures.
    pub is_closure: bool,
    /// True for methods.
    pub is_method: bool,
}

/// IR function body.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IrFunction {
    /// Function name or synthesized top-level name.
    pub name: String,
    /// Parameters in declaration order.
    pub params: Vec<IrParam>,
    /// Local slot names without the leading `$`, indexed by `LocalId`.
    pub locals: Vec<String>,
    /// Number of local slots.
    pub local_count: u32,
    /// Number of registers.
    pub register_count: u32,
    /// Basic blocks.
    pub blocks: Vec<BasicBlock>,
    /// Source span for the function declaration/body.
    pub span: IrSpan,
    /// Function flags.
    pub flags: FunctionFlags,
    /// Optional declared return type enforced by the VM MVP.
    pub return_type: Option<IrReturnType>,
    /// Closure capture locals in deterministic declaration/discovery order.
    pub captures: Vec<IrCapture>,
}

impl IrFunction {
    /// Creates a function shell.
    #[must_use]
    pub fn new(name: impl Into<String>, flags: FunctionFlags, span: IrSpan) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            locals: Vec::new(),
            local_count: 0,
            register_count: 0,
            blocks: Vec::new(),
            span,
            flags,
            return_type: None,
            captures: Vec::new(),
        }
    }
}
