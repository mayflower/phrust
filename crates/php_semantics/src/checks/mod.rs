//! Semantic validation pass module boundary.
//!
//! Prompt 01 creates this namespace so later Phase 3 checks can be added
//! without putting validation logic in the parser or AST view layers.

pub mod class_context;
pub mod modifiers;
