//! HIR attribute metadata records.

use crate::hir::ExprId;
use crate::hir::expr::HirNameResolution;

/// Attribute metadata record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirAttribute {
    target: AttributeTarget,
    name: HirNameResolution,
    args: Vec<ExprId>,
    repeated_on_target: bool,
}

impl HirAttribute {
    /// Creates an attribute metadata record.
    #[must_use]
    pub fn new(
        target: AttributeTarget,
        name: HirNameResolution,
        args: Vec<ExprId>,
        repeated_on_target: bool,
    ) -> Self {
        Self {
            target,
            name,
            args,
            repeated_on_target,
        }
    }

    /// Returns the syntactic attribute target.
    #[must_use]
    pub const fn target(&self) -> AttributeTarget {
        self.target
    }

    /// Returns the lowered attribute name.
    #[must_use]
    pub const fn name(&self) -> &HirNameResolution {
        &self.name
    }

    /// Returns argument expression IDs.
    #[must_use]
    pub fn args(&self) -> &[ExprId] {
        &self.args
    }

    /// Returns whether this target has another attribute with the same name.
    #[must_use]
    pub const fn is_repeated_on_target(&self) -> bool {
        self.repeated_on_target
    }
}

/// Syntactic target an attribute is attached to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeTarget {
    /// Class declaration.
    Class,
    /// Interface declaration.
    Interface,
    /// Trait declaration.
    Trait,
    /// Enum declaration.
    Enum,
    /// Function declaration.
    Function,
    /// Method declaration.
    Method,
    /// Property declaration.
    Property,
    /// Class constant declaration.
    ClassConstant,
    /// Enum case declaration.
    EnumCase,
    /// Function-like parameter.
    Parameter,
    /// Closure expression.
    Closure,
    /// Anonymous class expression.
    AnonymousClass,
    /// File-level or namespace-level attribute.
    FileOrNamespace,
    /// Attribute target not yet classified.
    Unknown,
}

impl AttributeTarget {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::Function => "function",
            Self::Method => "method",
            Self::Property => "property",
            Self::ClassConstant => "class_constant",
            Self::EnumCase => "enum_case",
            Self::Parameter => "parameter",
            Self::Closure => "closure",
            Self::AnonymousClass => "anonymous_class",
            Self::FileOrNamespace => "file_or_namespace",
            Self::Unknown => "unknown",
        }
    }
}
