//! Safe VM/JIT ABI boundary types.
//!
//! These types are intentionally handle-based. They do not expose raw pointers,
//! Rust references, frame internals, GC cells, refcount state, or COW storage to
//! future native code.

use std::num::NonZeroU64;

use php_ir::{BlockId, FunctionId, InstrId, LocalId, RegId};

/// Opaque non-zero handle owned by the VM side of the ABI.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct JitOpaqueHandle(NonZeroU64);

impl JitOpaqueHandle {
    /// Creates an opaque handle. Zero is reserved for "no handle".
    #[must_use]
    pub fn new(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    /// Returns the stable raw value for logging and test snapshots.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0.get()
    }
}

/// Opaque VM request/context handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct JitVmContextHandle(JitOpaqueHandle);

impl JitVmContextHandle {
    /// Creates a VM context handle.
    #[must_use]
    pub fn new(raw: u64) -> Option<Self> {
        JitOpaqueHandle::new(raw).map(Self)
    }

    /// Returns the stable raw handle value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0.raw()
    }
}

/// Opaque frame handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct JitFrameHandle(JitOpaqueHandle);

impl JitFrameHandle {
    /// Creates a frame handle.
    #[must_use]
    pub fn new(raw: u64) -> Option<Self> {
        JitOpaqueHandle::new(raw).map(Self)
    }

    /// Returns the stable raw handle value.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0.raw()
    }
}

/// Read-only frame/register metadata exported to future JIT code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitFrameView {
    /// VM context that owns the frame.
    pub context: JitVmContextHandle,
    /// Opaque active-frame handle.
    pub frame: JitFrameHandle,
    /// IR function represented by this frame.
    pub function: FunctionId,
    /// Number of VM registers available to this frame.
    pub register_count: u32,
    /// Number of local slots available to this frame.
    pub local_count: u32,
}

impl JitFrameView {
    /// Creates a frame view from opaque VM-owned handles and arena sizes.
    #[must_use]
    pub const fn new(
        context: JitVmContextHandle,
        frame: JitFrameHandle,
        function: FunctionId,
        register_count: u32,
        local_count: u32,
    ) -> Self {
        Self {
            context,
            frame,
            function,
            register_count,
            local_count,
        }
    }

    /// Returns true when a register can be addressed through this view.
    #[must_use]
    pub const fn contains_register(&self, register: RegId) -> bool {
        register.raw() < self.register_count
    }

    /// Returns true when a local can be addressed through this view.
    #[must_use]
    pub const fn contains_local(&self, local: LocalId) -> bool {
        local.raw() < self.local_count
    }
}

/// Heap-backed PHP value categories crossing the ABI as opaque handles only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JitOpaqueValueKind {
    /// PHP string storage.
    String,
    /// PHP array storage.
    Array,
    /// PHP object storage.
    Object,
    /// PHP resource storage.
    Resource,
    /// PHP reference cell.
    Reference,
    /// PHP callable/closure value.
    Callable,
    /// PHP generator value.
    Generator,
    /// PHP fiber value.
    Fiber,
}

impl JitOpaqueValueKind {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::Resource => "resource",
            Self::Reference => "reference",
            Self::Callable => "callable",
            Self::Generator => "generator",
            Self::Fiber => "fiber",
        }
    }
}

/// ABI-safe value representation.
#[derive(Clone, Debug, PartialEq)]
pub enum JitAbiValue {
    /// PHP null.
    Null,
    /// PHP bool.
    Bool(bool),
    /// PHP int.
    Int(i64),
    /// PHP float as raw IEEE-754 bits.
    FloatBits(u64),
    /// Uninitialized register/local marker.
    Uninitialized,
    /// VM-owned heap value represented by an opaque handle.
    Opaque {
        /// Heap value family.
        kind: JitOpaqueValueKind,
        /// VM-owned handle.
        handle: JitOpaqueHandle,
    },
}

impl JitAbiValue {
    /// Creates a float value while preserving exact bits.
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self::FloatBits(value.to_bits())
    }

    /// Returns true for heap-backed values that require VM side handling.
    #[must_use]
    pub const fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque { .. })
    }
}

/// Why future native code left the compiled region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JitBailoutKind {
    /// Type/value guard failed.
    GuardFailed,
    /// Encountered a value outside the primitive subset.
    UnsupportedValue,
    /// Runtime callout requested interpreter fallback.
    RuntimeCallout,
    /// Deoptimization requested by invalidation or missing metadata.
    Deopt,
}

impl JitBailoutKind {
    /// Stable report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GuardFailed => "guard_failed",
            Self::UnsupportedValue => "unsupported_value",
            Self::RuntimeCallout => "runtime_callout",
            Self::Deopt => "deopt",
        }
    }
}

/// Bailout/deoptimization metadata returned to the VM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitBailout {
    /// Bailout family.
    pub kind: JitBailoutKind,
    /// Optional block to resume in interpreter mode.
    pub resume_block: Option<BlockId>,
    /// Optional instruction to resume in interpreter mode.
    pub resume_instruction: Option<InstrId>,
    /// Stable debug reason.
    pub reason: String,
}

impl JitBailout {
    /// Creates a bailout result.
    #[must_use]
    pub fn new(kind: JitBailoutKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            resume_block: None,
            resume_instruction: None,
            reason: reason.into(),
        }
    }

    /// Adds an interpreter resume point.
    #[must_use]
    pub const fn with_resume(mut self, block: BlockId, instruction: InstrId) -> Self {
        self.resume_block = Some(block);
        self.resume_instruction = Some(instruction);
        self
    }
}

/// Exception marker crossing the ABI.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitExceptionMarker {
    /// Stable PHP exception/error class name when known.
    pub class_name: Option<String>,
    /// Stable message snapshot when known.
    pub message: Option<String>,
    /// Opaque VM-owned exception object handle when already allocated.
    pub exception: Option<JitOpaqueHandle>,
}

impl JitExceptionMarker {
    /// Creates a marker from a class/message pair without exposing the object.
    #[must_use]
    pub fn named(class_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            class_name: Some(class_name.into()),
            message: Some(message.into()),
            exception: None,
        }
    }

    /// Creates a marker for an existing VM-owned exception object.
    #[must_use]
    pub fn opaque(exception: JitOpaqueHandle) -> Self {
        Self {
            class_name: None,
            message: None,
            exception: Some(exception),
        }
    }
}

/// Runtime callout identity and arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct JitRuntimeCallout {
    /// Stable callout name.
    pub name: String,
    /// ABI values copied or represented by opaque handles.
    pub args: Vec<JitAbiValue>,
    /// True when the VM side may report an exception marker.
    pub can_throw: bool,
}

impl JitRuntimeCallout {
    /// Creates a runtime callout descriptor.
    #[must_use]
    pub fn new(name: impl Into<String>, args: Vec<JitAbiValue>, can_throw: bool) -> Self {
        Self {
            name: name.into(),
            args,
            can_throw,
        }
    }
}

/// Result returned from a VM runtime callout.
#[derive(Clone, Debug, PartialEq)]
pub enum JitRuntimeCalloutResult {
    /// Callout returned a normal ABI value.
    Returned(JitAbiValue),
    /// Callout requested interpreter fallback/deopt.
    Bailout(JitBailout),
    /// Callout propagated a PHP exception/error.
    Exception(JitExceptionMarker),
}

/// Result of a future compiled region.
#[derive(Clone, Debug, PartialEq)]
pub enum JitRegionResult {
    /// Region produced a normal PHP value.
    Returned(JitAbiValue),
    /// Region bailed out to interpreter execution.
    Bailout(JitBailout),
    /// Region propagated an exception marker to the VM.
    Exception(JitExceptionMarker),
}
