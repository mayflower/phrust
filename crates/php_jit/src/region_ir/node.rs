//! Region IR node and constant model.

use super::{ConstId, EntryId, ExitId, NodeId, SnapshotId, VmSlotId};

/// Compact value classes tracked by the optimizer IR.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionValueType {
    /// Boolean scalar.
    Bool,
    /// Signed 64-bit integer scalar.
    I64,
    /// 64-bit floating-point scalar.
    F64,
    /// Opaque handle to an interned/runtime string.
    StringHandle,
    /// Opaque handle to a PHP array.
    ArrayHandle,
    /// Opaque handle to a PHP object.
    ObjectHandle,
    /// Generic PHP value where exact semantics are not modeled.
    MixedValue,
    /// Memory/effect token.
    Memory,
    /// Control token.
    Control,
}

impl RegionValueType {
    /// Stable dump spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::StringHandle => "string-handle",
            Self::ArrayHandle => "array-handle",
            Self::ObjectHandle => "object-handle",
            Self::MixedValue => "mixed-value",
            Self::Memory => "memory",
            Self::Control => "control",
        }
    }
}

/// Scheduling placement class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionPlacement {
    /// Pure data node that may be scheduled later by safe optimizer passes.
    Floating,
    /// Node pinned to a control point.
    Pinned,
    /// Node that only produces/consumes control.
    ControlOnly,
}

impl RegionPlacement {
    /// Stable dump spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Floating => "floating",
            Self::Pinned => "pinned",
            Self::ControlOnly => "control-only",
        }
    }
}

/// Effect bits attached to a node.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegionEffects {
    /// Reads VM/runtime memory.
    pub reads_memory: bool,
    /// Writes VM/runtime memory.
    pub writes_memory: bool,
    /// May call back into runtime or user code.
    pub may_call: bool,
    /// May throw/raise an exception.
    pub may_throw: bool,
    /// May emit PHP-visible diagnostics.
    pub may_diagnose: bool,
    /// May leave an optimized tier.
    pub may_deopt: bool,
}

impl RegionEffects {
    /// No PHP-visible effects.
    pub const PURE: Self = Self {
        reads_memory: false,
        writes_memory: false,
        may_call: false,
        may_throw: false,
        may_diagnose: false,
        may_deopt: false,
    };

    /// Guard/deopt effect marker.
    pub const MAY_DEOPT: Self = Self {
        reads_memory: false,
        writes_memory: false,
        may_call: false,
        may_throw: false,
        may_diagnose: false,
        may_deopt: true,
    };

    /// Returns true when no effect bit is set.
    #[must_use]
    pub const fn is_pure(self) -> bool {
        !self.reads_memory
            && !self.writes_memory
            && !self.may_call
            && !self.may_throw
            && !self.may_diagnose
            && !self.may_deopt
    }

    /// Stable dump spelling.
    #[must_use]
    pub fn dump_label(self) -> String {
        if self.is_pure() {
            return "pure".to_string();
        }

        let mut labels = Vec::new();
        if self.reads_memory {
            labels.push("reads-memory");
        }
        if self.writes_memory {
            labels.push("writes-memory");
        }
        if self.may_call {
            labels.push("may-call");
        }
        if self.may_throw {
            labels.push("may-throw");
        }
        if self.may_diagnose {
            labels.push("may-diagnose");
        }
        if self.may_deopt {
            labels.push("may-deopt");
        }
        labels.join("|")
    }
}

/// Scalar compare operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionCompareOp {
    /// `==` over the exact scalar type.
    Eq,
    /// `!=` over the exact scalar type.
    NotEq,
    /// `<` over the exact scalar type.
    Lt,
    /// `<=` over the exact scalar type.
    Lte,
    /// `>` over the exact scalar type.
    Gt,
    /// `>=` over the exact scalar type.
    Gte,
}

impl RegionCompareOp {
    /// Stable dump spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::NotEq => "ne",
            Self::Lt => "lt",
            Self::Lte => "lte",
            Self::Gt => "gt",
            Self::Gte => "gte",
        }
    }
}

/// Constants are stored in a table and referenced by compact IDs.
#[derive(Clone, Debug, PartialEq)]
pub enum RegionConst {
    /// Boolean scalar.
    Bool(bool),
    /// Signed 64-bit integer scalar.
    I64(i64),
    /// 64-bit floating-point scalar.
    F64(f64),
    /// Opaque string handle marker.
    StringHandle(String),
}

impl RegionConst {
    /// Returns the value type produced by this constant.
    #[must_use]
    pub const fn value_type(&self) -> RegionValueType {
        match self {
            Self::Bool(_) => RegionValueType::Bool,
            Self::I64(_) => RegionValueType::I64,
            Self::F64(_) => RegionValueType::F64,
            Self::StringHandle(_) => RegionValueType::StringHandle,
        }
    }
}

/// Region IR node kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegionNodeKind {
    Start,
    End,
    Begin,
    Merge,
    LoopBegin,
    LoopEnd,
    If,
    IfTrue,
    IfFalse,
    Return,
    Entry(EntryId),
    Exit(ExitId),
    Param { slot: VmSlotId },
    Const(ConstId),
    Phi,
    Copy,
    Add,
    Sub,
    Mul,
    AndBool,
    OrBool,
    Div,
    Mod,
    Neg,
    Compare(RegionCompareOp),
    Cast,
    Select,
    Load,
    Store,
    Call,
    RuntimeHelper,
    ArrayAccess,
    PropertyAccess,
    Guard { snapshot: SnapshotId },
    Snapshot(SnapshotId),
    Assumption,
    DeoptPoint { snapshot: SnapshotId },
}

impl RegionNodeKind {
    /// Stable dump spelling.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::End => "End",
            Self::Begin => "Begin",
            Self::Merge => "Merge",
            Self::LoopBegin => "LoopBegin",
            Self::LoopEnd => "LoopEnd",
            Self::If => "If",
            Self::IfTrue => "IfTrue",
            Self::IfFalse => "IfFalse",
            Self::Return => "Return",
            Self::Entry(_) => "Entry",
            Self::Exit(_) => "Exit",
            Self::Param { .. } => "Param",
            Self::Const(_) => "Const",
            Self::Phi => "Phi",
            Self::Copy => "Copy",
            Self::Add => "Add",
            Self::Sub => "Sub",
            Self::Mul => "Mul",
            Self::AndBool => "AndBool",
            Self::OrBool => "OrBool",
            Self::Div => "Div",
            Self::Mod => "Mod",
            Self::Neg => "Neg",
            Self::Compare(_) => "Compare",
            Self::Cast => "Cast",
            Self::Select => "Select",
            Self::Load => "Load",
            Self::Store => "Store",
            Self::Call => "Call",
            Self::RuntimeHelper => "RuntimeHelper",
            Self::ArrayAccess => "ArrayAccess",
            Self::PropertyAccess => "PropertyAccess",
            Self::Guard { .. } => "Guard",
            Self::Snapshot(_) => "Snapshot",
            Self::Assumption => "Assumption",
            Self::DeoptPoint { .. } => "DeoptPoint",
        }
    }

    /// Returns true for control-only nodes.
    #[must_use]
    pub const fn is_control(&self) -> bool {
        matches!(
            self,
            Self::Start
                | Self::End
                | Self::Begin
                | Self::Merge
                | Self::LoopBegin
                | Self::LoopEnd
                | Self::If
                | Self::IfTrue
                | Self::IfFalse
                | Self::Return
                | Self::Entry(_)
                | Self::Exit(_)
        )
    }
}

/// One compact region node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionNode {
    /// Node operation.
    pub kind: RegionNodeKind,
    /// Data inputs.
    pub inputs: Vec<NodeId>,
    /// Optional control dependency.
    pub control: Option<NodeId>,
    /// Produced value/control type.
    pub value_type: RegionValueType,
    /// Scheduling placement.
    pub placement: RegionPlacement,
    /// PHP-visible effect flags.
    pub effects: RegionEffects,
}

impl RegionNode {
    /// Creates a new node.
    #[must_use]
    pub fn new(
        kind: RegionNodeKind,
        inputs: Vec<NodeId>,
        control: Option<NodeId>,
        value_type: RegionValueType,
        placement: RegionPlacement,
        effects: RegionEffects,
    ) -> Self {
        Self {
            kind,
            inputs,
            control,
            value_type,
            placement,
            effects,
        }
    }
}
