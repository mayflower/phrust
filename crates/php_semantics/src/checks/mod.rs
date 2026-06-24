//! Semantic validation pass module boundary.
//!
//! Semantic checks live here as the frontend grows beyond structural lowering.
//! without putting validation logic in the parser or AST view layers.

pub mod class_context;
pub mod modifiers;
