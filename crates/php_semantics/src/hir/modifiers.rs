//! Declaration modifier and flag HIR records.

use crate::hir::Visibility;
use php_source::TextRange;

/// PHP declaration modifier token classified into a semantic flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Modifier {
    /// `abstract`.
    Abstract,
    /// `final`.
    Final,
    /// `static`.
    Static,
    /// `readonly`.
    Readonly,
    /// `public`.
    Public,
    /// `protected`.
    Protected,
    /// `private`.
    Private,
    /// `public(set)`.
    PublicSet,
    /// `protected(set)`.
    ProtectedSet,
    /// `private(set)`.
    PrivateSet,
    /// `var`.
    Var,
    /// `&`.
    ByRef,
    /// `...`.
    Variadic,
    /// Constructor property-promotion marker.
    Promoted,
    /// Property declaration with a hook body.
    HookRelated,
}

impl Modifier {
    /// Classifies a CST token name as a semantic modifier.
    #[must_use]
    pub fn from_token_name(token_name: &str) -> Option<Self> {
        match token_name {
            "T_ABSTRACT" => Some(Self::Abstract),
            "T_FINAL" => Some(Self::Final),
            "T_STATIC" => Some(Self::Static),
            "T_READONLY" => Some(Self::Readonly),
            "T_PUBLIC" => Some(Self::Public),
            "T_PROTECTED" => Some(Self::Protected),
            "T_PRIVATE" => Some(Self::Private),
            "T_PUBLIC_SET" => Some(Self::PublicSet),
            "T_PROTECTED_SET" => Some(Self::ProtectedSet),
            "T_PRIVATE_SET" => Some(Self::PrivateSet),
            "T_VAR" => Some(Self::Var),
            "&" => Some(Self::ByRef),
            "T_ELLIPSIS" => Some(Self::Variadic),
            _ => None,
        }
    }

    /// Returns stable lowercase text for JSON, docs, and diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Abstract => "abstract",
            Self::Final => "final",
            Self::Static => "static",
            Self::Readonly => "readonly",
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
            Self::PublicSet => "public(set)",
            Self::ProtectedSet => "protected(set)",
            Self::PrivateSet => "private(set)",
            Self::Var => "var",
            Self::ByRef => "by_ref",
            Self::Variadic => "variadic",
            Self::Promoted => "promoted",
            Self::HookRelated => "hook_related",
        }
    }

    /// Returns normal member visibility represented by this modifier.
    #[must_use]
    pub const fn visibility(self) -> Option<Visibility> {
        match self {
            Self::Public => Some(Visibility::Public),
            Self::Protected => Some(Visibility::Protected),
            Self::Private => Some(Visibility::Private),
            _ => None,
        }
    }

    /// Returns property-set visibility represented by this modifier.
    #[must_use]
    pub const fn set_visibility(self) -> Option<Visibility> {
        match self {
            Self::PublicSet => Some(Visibility::Public),
            Self::ProtectedSet => Some(Visibility::Protected),
            Self::PrivateSet => Some(Visibility::Private),
            _ => None,
        }
    }
}

/// One source modifier with its byte span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModifierOccurrence {
    modifier: Modifier,
    span: TextRange,
}

impl ModifierOccurrence {
    /// Creates a modifier occurrence.
    #[must_use]
    pub const fn new(modifier: Modifier, span: TextRange) -> Self {
        Self { modifier, span }
    }

    /// Returns the semantic modifier.
    #[must_use]
    pub const fn modifier(self) -> Modifier {
        self.modifier
    }

    /// Returns the source span.
    #[must_use]
    pub const fn span(self) -> TextRange {
        self.span
    }
}

/// Normalized modifier flags attached to one declaration-like target.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModifierSet {
    occurrences: Vec<ModifierOccurrence>,
    visibility: Option<Visibility>,
    set_visibility: Option<Visibility>,
    is_abstract: bool,
    is_final: bool,
    is_static: bool,
    is_readonly: bool,
    is_by_ref: bool,
    is_variadic: bool,
    is_promoted: bool,
    is_hook_related: bool,
    uses_var: bool,
}

impl ModifierSet {
    /// Creates an empty modifier set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            occurrences: Vec::new(),
            visibility: None,
            set_visibility: None,
            is_abstract: false,
            is_final: false,
            is_static: false,
            is_readonly: false,
            is_by_ref: false,
            is_variadic: false,
            is_promoted: false,
            is_hook_related: false,
            uses_var: false,
        }
    }

    /// Adds one modifier occurrence.
    pub fn push(&mut self, occurrence: ModifierOccurrence) {
        let modifier = occurrence.modifier();
        if let Some(visibility) = modifier.visibility() {
            self.visibility.get_or_insert(visibility);
        }
        if let Some(set_visibility) = modifier.set_visibility() {
            self.set_visibility.get_or_insert(set_visibility);
        }
        match modifier {
            Modifier::Abstract => self.is_abstract = true,
            Modifier::Final => self.is_final = true,
            Modifier::Static => self.is_static = true,
            Modifier::Readonly => self.is_readonly = true,
            Modifier::Var => self.uses_var = true,
            Modifier::ByRef => self.is_by_ref = true,
            Modifier::Variadic => self.is_variadic = true,
            Modifier::Promoted => self.is_promoted = true,
            Modifier::HookRelated => self.is_hook_related = true,
            Modifier::Public
            | Modifier::Protected
            | Modifier::Private
            | Modifier::PublicSet
            | Modifier::ProtectedSet
            | Modifier::PrivateSet => {}
        }
        self.occurrences.push(occurrence);
    }

    /// Returns occurrences in source order.
    #[must_use]
    pub fn occurrences(&self) -> &[ModifierOccurrence] {
        &self.occurrences
    }

    /// Returns normal visibility, if one was present.
    #[must_use]
    pub const fn visibility(&self) -> Option<Visibility> {
        self.visibility
    }

    /// Returns property-set visibility, if one was present.
    #[must_use]
    pub const fn set_visibility(&self) -> Option<Visibility> {
        self.set_visibility
    }

    /// Returns true for `abstract`.
    #[must_use]
    pub const fn is_abstract(&self) -> bool {
        self.is_abstract
    }

    /// Returns true for `final`.
    #[must_use]
    pub const fn is_final(&self) -> bool {
        self.is_final
    }

    /// Returns true for `static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.is_static
    }

    /// Returns true for `readonly`.
    #[must_use]
    pub const fn is_readonly(&self) -> bool {
        self.is_readonly
    }

    /// Returns true for `&`.
    #[must_use]
    pub const fn is_by_ref(&self) -> bool {
        self.is_by_ref
    }

    /// Returns true for `...`.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.is_variadic
    }

    /// Returns true for promoted constructor properties.
    #[must_use]
    pub const fn is_promoted(&self) -> bool {
        self.is_promoted
    }

    /// Returns true when a property has a hook body.
    #[must_use]
    pub const fn is_hook_related(&self) -> bool {
        self.is_hook_related
    }

    /// Returns true for legacy `var` properties.
    #[must_use]
    pub const fn uses_var(&self) -> bool {
        self.uses_var
    }
}
