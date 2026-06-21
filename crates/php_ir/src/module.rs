//! IR unit and top-level tables.

use crate::constants::IrConstant;
use crate::function::{IrFunction, IrReturnType};
use crate::ids::{ClassId, ConstId, FileId, FunctionId, UnitId};
use crate::source_map::{IrSourceMap, IrSpan};
use serde::{Deserialize, Serialize};

/// Version marker for the Phase 4 IR snapshot shape.
pub const IR_VERSION: u32 = 1;

/// Source file table entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileEntry {
    /// File ID.
    pub id: FileId,
    /// Display path.
    pub path: String,
}

/// Class table entry used by the object/runtime prompts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassEntry {
    /// Class ID.
    pub id: ClassId,
    /// Resolved class name.
    pub name: String,
    /// Method entries in source order.
    pub methods: Vec<ClassMethodEntry>,
    /// Declared instance properties in source order.
    pub properties: Vec<ClassPropertyEntry>,
    /// Constructor method function ID, when present.
    pub constructor: Option<FunctionId>,
    /// Class flags captured from Phase 3.
    pub flags: ClassFlags,
    /// Source span for the class declaration.
    pub span: IrSpan,
}

/// Class declaration flags.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassFlags {
    /// `abstract class`.
    pub is_abstract: bool,
    /// `final class`.
    pub is_final: bool,
    /// `readonly class`.
    pub is_readonly: bool,
}

/// Class method table entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassMethodEntry {
    /// Normalized method lookup name.
    pub name: String,
    /// Method implementation function.
    pub function: FunctionId,
    /// Method flags captured from Phase 3.
    pub flags: ClassMethodFlags,
}

/// Class method flags.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassMethodFlags {
    /// `static`.
    pub is_static: bool,
    /// `private`.
    pub is_private: bool,
    /// `protected`.
    pub is_protected: bool,
    /// `abstract`.
    pub is_abstract: bool,
}

/// Class property table entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassPropertyEntry {
    /// Property name without `$`.
    pub name: String,
    /// Constant-pool default when the MVP can lower it.
    pub default: Option<ConstId>,
    /// Optional Phase-3 lowered runtime type enforced by the VM MVP.
    pub type_: Option<IrReturnType>,
    /// Property flags captured from Phase 3.
    pub flags: ClassPropertyFlags,
}

/// Class property flags.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClassPropertyFlags {
    /// `static`.
    pub is_static: bool,
    /// `private`.
    pub is_private: bool,
    /// `protected`.
    pub is_protected: bool,
    /// `readonly`.
    pub is_readonly: bool,
    /// Has a declared type.
    pub is_typed: bool,
}

/// Named function lookup entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FunctionEntry {
    /// Normalized lookup name.
    pub name: String,
    /// Function table ID.
    pub function: FunctionId,
}

/// Runtime-visible constant lookup entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GlobalConstantEntry {
    /// Canonical runtime lookup name.
    pub name: String,
    /// Constant-pool value.
    pub value: crate::ids::ConstId,
    /// Source span for the constant declaration.
    pub span: IrSpan,
}

/// Compiled IR unit.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct IrUnit {
    /// IR version.
    pub version: u32,
    /// Unit ID.
    pub id: UnitId,
    /// Constant pool.
    pub constants: Vec<IrConstant>,
    /// Function table.
    pub functions: Vec<IrFunction>,
    /// Deterministic normalized function-name lookup table.
    pub function_table: Vec<FunctionEntry>,
    /// Deterministic runtime constant lookup table.
    pub constant_table: Vec<GlobalConstantEntry>,
    /// Class skeleton table.
    pub classes: Vec<ClassEntry>,
    /// File/source table.
    pub files: Vec<FileEntry>,
    /// Entry function.
    pub entry: FunctionId,
    /// IR-to-HIR/source mapping.
    pub source_map: IrSourceMap,
}

impl IrUnit {
    /// Creates an empty unit.
    #[must_use]
    pub fn new(id: UnitId) -> Self {
        Self {
            version: IR_VERSION,
            id,
            constants: Vec::new(),
            functions: Vec::new(),
            function_table: Vec::new(),
            constant_table: Vec::new(),
            classes: Vec::new(),
            files: Vec::new(),
            entry: FunctionId::new(0),
            source_map: IrSourceMap::new(),
        }
    }
}
