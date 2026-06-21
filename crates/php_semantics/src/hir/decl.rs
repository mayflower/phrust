//! HIR declaration placeholders.

use crate::hir::ids::{AttributeId, ExprId, TypeId};
use crate::hir::names::HirName;

/// Declaration record stored in the module declaration arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirDecl {
    kind: HirDeclKind,
    name: Option<HirName>,
}

impl HirDecl {
    /// Creates a declaration placeholder.
    #[must_use]
    pub const fn new(kind: HirDeclKind, name: Option<HirName>) -> Self {
        Self { kind, name }
    }

    /// Returns the declaration kind.
    #[must_use]
    pub const fn kind(&self) -> HirDeclKind {
        self.kind
    }

    /// Returns the optional declaration name.
    #[must_use]
    pub const fn name(&self) -> Option<&HirName> {
        self.name.as_ref()
    }
}

/// Declaration families used by later lowering prompts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HirDeclKind {
    /// Namespace declaration.
    Namespace,
    /// Use/import declaration.
    Use,
    /// Constant declaration.
    Const,
    /// Function declaration.
    Function,
    /// Class-like declaration.
    ClassLike,
    /// Method declaration.
    Method,
    /// Property declaration.
    Property,
    /// Parameter declaration.
    Parameter,
    /// Attribute metadata.
    Attribute(AttributeId),
    /// Type annotation.
    Type(TypeId),
    /// Default or constant expression.
    Expr(ExprId),
}
