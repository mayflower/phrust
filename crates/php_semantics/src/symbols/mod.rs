//! Symbol and name interning support.

pub mod declarations;
pub mod imports;
mod interner;
pub mod resolution;

pub use crate::hir::NameId;
pub use interner::{InternedName, NameInterner};
