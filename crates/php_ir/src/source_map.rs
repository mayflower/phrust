//! Source-map primitives for IR instructions.

use crate::ids::{BlockId, FileId, FunctionId, InstrId};
use php_source::TextRange;
use serde::{Deserialize, Serialize};

/// Source span attached to IR instructions and terminators.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSpan {
    /// Source file index in the IR unit file table.
    pub file: FileId,
    /// Start byte offset.
    pub start: u32,
    /// End byte offset.
    pub end: u32,
}

impl IrSpan {
    /// Creates a source span.
    #[must_use]
    pub const fn new(file: FileId, start: u32, end: u32) -> Self {
        Self { file, start, end }
    }

    /// Creates a source span from a `php_source` text range.
    #[must_use]
    pub fn from_text_range(file: FileId, range: TextRange) -> Self {
        Self {
            file,
            start: range.start().to_usize() as u32,
            end: range.end().to_usize() as u32,
        }
    }
}

/// Source-map target inside an IR unit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IrSourceMapTarget {
    /// Function table entry.
    Function { function: FunctionId },
    /// Basic block inside a function.
    Block {
        function: FunctionId,
        block: BlockId,
    },
    /// Instruction inside a basic block.
    Instruction {
        function: FunctionId,
        block: BlockId,
        instruction: InstrId,
    },
    /// Block terminator.
    Terminator {
        function: FunctionId,
        block: BlockId,
    },
}

/// One IR-to-source-map entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSourceMapEntry {
    /// IR target.
    pub target: IrSourceMapTarget,
    /// Stable Phase 3 origin label, such as `hir:expr:0`.
    pub origin: String,
    /// Source span in the IR file table.
    pub span: IrSpan,
}

/// Source-map table for an IR unit.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct IrSourceMap {
    entries: Vec<IrSourceMapEntry>,
}

impl IrSourceMap {
    /// Creates an empty IR source map.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Adds a source-map entry.
    pub fn push(&mut self, target: IrSourceMapTarget, origin: impl Into<String>, span: IrSpan) {
        self.entries.push(IrSourceMapEntry {
            target,
            origin: origin.into(),
            span,
        });
    }

    /// Returns entries in insertion order.
    #[must_use]
    pub fn entries(&self) -> &[IrSourceMapEntry] {
        &self.entries
    }

    /// Returns true when no entries are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
