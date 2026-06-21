//! Declaration table for one analyzed source file.

use std::collections::HashMap;

use php_source::TextRange;

use crate::hir::{DeclId, FullyQualifiedName, NameKind, SymbolId};

/// Declaration family registered by declaration collection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclarationKind {
    /// Top-level function declaration.
    Function,
    /// Top-level constant declaration.
    Constant,
    /// Class declaration.
    Class,
    /// Interface declaration.
    Interface,
    /// Trait declaration.
    Trait,
    /// Enum declaration.
    Enum,
    /// Function declaration nested in statement context.
    ConditionalFunction,
    /// Class-like declaration nested in statement context.
    ConditionalClassLike,
}

impl DeclarationKind {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Constant => "constant",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::ConditionalFunction => "conditional_function",
            Self::ConditionalClassLike => "conditional_class_like",
        }
    }

    /// Returns true for declarations whose existence depends on execution.
    #[must_use]
    pub const fn is_conditional(self) -> bool {
        matches!(self, Self::ConditionalFunction | Self::ConditionalClassLike)
    }

    /// Returns the duplicate-detection namespace for this declaration.
    #[must_use]
    pub const fn duplicate_name_kind(self) -> NameKind {
        match self {
            Self::Function | Self::ConditionalFunction => NameKind::Function,
            Self::Constant => NameKind::Constant,
            Self::Class
            | Self::Interface
            | Self::Trait
            | Self::Enum
            | Self::ConditionalClassLike => NameKind::ClassLike,
        }
    }
}

/// Declaration table entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationEntry {
    decl_id: DeclId,
    symbol_id: SymbolId,
    kind: DeclarationKind,
    name: String,
    fqn: FullyQualifiedName,
    span: TextRange,
}

impl DeclarationEntry {
    /// Creates a declaration entry.
    #[must_use]
    pub fn new(
        decl_id: DeclId,
        symbol_id: SymbolId,
        kind: DeclarationKind,
        name: impl Into<String>,
        fqn: FullyQualifiedName,
        span: TextRange,
    ) -> Self {
        Self {
            decl_id,
            symbol_id,
            kind,
            name: name.into(),
            fqn,
            span,
        }
    }

    /// Returns declaration ID.
    #[must_use]
    pub const fn decl_id(&self) -> DeclId {
        self.decl_id
    }

    /// Returns symbol ID.
    #[must_use]
    pub const fn symbol_id(&self) -> SymbolId {
        self.symbol_id
    }

    /// Returns declaration kind.
    #[must_use]
    pub const fn kind(&self) -> DeclarationKind {
        self.kind
    }

    /// Returns short source name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns declaration FQN.
    #[must_use]
    pub const fn fqn(&self) -> &FullyQualifiedName {
        &self.fqn
    }

    /// Returns source span.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Duplicate declaration details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateDeclaration {
    name: String,
    kind: DeclarationKind,
    previous_span: TextRange,
    duplicate_span: TextRange,
}

impl DuplicateDeclaration {
    /// Returns duplicate declaration name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns duplicate declaration kind.
    #[must_use]
    pub const fn kind(&self) -> DeclarationKind {
        self.kind
    }

    /// Returns previous declaration span.
    #[must_use]
    pub const fn previous_span(&self) -> TextRange {
        self.previous_span
    }

    /// Returns duplicate declaration span.
    #[must_use]
    pub const fn duplicate_span(&self) -> TextRange {
        self.duplicate_span
    }
}

/// Source-file declaration table in deterministic registration order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DeclarationTable {
    entries: Vec<DeclarationEntry>,
    unconditional_lookup: HashMap<(NameKind, String), usize>,
}

impl DeclarationTable {
    /// Creates an empty declaration table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a declaration and returns duplicate details for safe local
    /// duplicate cases.
    pub fn insert(&mut self, entry: DeclarationEntry) -> Result<(), DuplicateDeclaration> {
        let duplicate = if !entry.kind().is_conditional() {
            let name_kind = entry.kind().duplicate_name_kind();
            let canonical = entry.fqn().canonical(name_kind);
            let key = (name_kind, canonical);
            if let Some(previous_index) = self.unconditional_lookup.get(&key) {
                let previous = &self.entries[*previous_index];
                Some(DuplicateDeclaration {
                    name: entry.name.clone(),
                    kind: entry.kind,
                    previous_span: previous.span(),
                    duplicate_span: entry.span(),
                })
            } else {
                self.unconditional_lookup.insert(key, self.entries.len());
                None
            }
        } else {
            None
        };

        self.entries.push(entry);
        if let Some(duplicate) = duplicate {
            Err(duplicate)
        } else {
            Ok(())
        }
    }

    /// Returns declarations in registration order.
    #[must_use]
    pub fn entries(&self) -> &[DeclarationEntry] {
        &self.entries
    }

    /// Returns true if no declarations are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
