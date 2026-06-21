//! Deterministic source-name interner.

use std::collections::HashMap;

use crate::hir::{NameId, NameKind, QualifiedName};

/// Interned source-level name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InternedName {
    id: NameId,
    kind: NameKind,
    original: String,
    canonical: String,
}

impl InternedName {
    /// Returns the typed name ID.
    #[must_use]
    pub const fn id(&self) -> NameId {
        self.id
    }

    /// Returns the semantic name category.
    #[must_use]
    pub const fn kind(&self) -> NameKind {
        self.kind
    }

    /// Returns the first spelling interned for this canonical name.
    #[must_use]
    pub fn original(&self) -> &str {
        &self.original
    }

    /// Returns the canonical lookup key without the category discriminator.
    #[must_use]
    pub fn canonical(&self) -> &str {
        &self.canonical
    }
}

/// Interns names by kind-aware canonical spelling.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NameInterner {
    names: Vec<InternedName>,
    lookup: HashMap<(NameKind, String), NameId>,
}

impl NameInterner {
    /// Creates an empty name interner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a raw source-level name.
    pub fn intern_raw(&mut self, raw: impl AsRef<str>, kind: NameKind) -> NameId {
        let name = QualifiedName::parse(raw.as_ref());
        self.intern_qualified(&name, kind)
    }

    /// Interns a parsed source-level name.
    pub fn intern_qualified(&mut self, name: &QualifiedName, kind: NameKind) -> NameId {
        let canonical = name.canonical(kind);
        let key = (kind, canonical.clone());
        if let Some(id) = self.lookup.get(&key) {
            return *id;
        }

        let id = NameId::from_raw(self.names.len());
        self.lookup.insert(key, id);
        self.names.push(InternedName {
            id,
            kind,
            original: name.original().to_owned(),
            canonical,
        });
        id
    }

    /// Returns an interned name by ID.
    #[must_use]
    pub fn get(&self, id: NameId) -> Option<&InternedName> {
        self.names.get(id.raw())
    }

    /// Returns all interned names in deterministic allocation order.
    #[must_use]
    pub fn names(&self) -> &[InternedName] {
        &self.names
    }

    /// Returns the number of interned names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Returns true if no names have been interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::NameInterner;
    use crate::hir::{NameId, NameKind, QualifiedName};

    #[test]
    fn class_like_names_share_ascii_case_insensitive_id() {
        let mut interner = NameInterner::new();

        let upper = interner.intern_raw("Foo", NameKind::ClassLike);
        let lower = interner.intern_raw("foo", NameKind::ClassLike);
        let shouting = interner.intern_raw("FOO", NameKind::ClassLike);

        assert_eq!(upper, NameId::from_raw(0));
        assert_eq!(upper, lower);
        assert_eq!(lower, shouting);
        assert_eq!(interner.len(), 1);
        assert_eq!(interner.get(upper).expect("name").original(), "Foo");
        assert_eq!(interner.get(upper).expect("name").canonical(), "foo");
    }

    #[test]
    fn variable_names_remain_case_sensitive() {
        let mut interner = NameInterner::new();

        let upper = interner.intern_raw("Foo", NameKind::VariableName);
        let lower = interner.intern_raw("foo", NameKind::VariableName);

        assert_ne!(upper, lower);
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn namespace_relative_and_fully_qualified_names_have_distinct_keys() {
        let mut interner = NameInterner::new();

        let relative = interner.intern_raw("Foo\\Bar", NameKind::ClassLike);
        let fqn = interner.intern_raw("\\Foo\\Bar", NameKind::ClassLike);
        let namespace_relative = interner.intern_raw("namespace\\Foo\\Bar", NameKind::ClassLike);

        assert_ne!(relative, fqn);
        assert_ne!(relative, namespace_relative);
        assert_ne!(fqn, namespace_relative);
    }

    #[test]
    fn qualified_name_input_uses_same_key() {
        let mut interner = NameInterner::new();
        let name = QualifiedName::parse("Foo\\Bar");

        let first = interner.intern_qualified(&name, NameKind::Function);
        let second = interner.intern_raw("foo\\bar", NameKind::Function);

        assert_eq!(first, second);
    }

    #[test]
    fn constants_keep_original_case_until_resolution_rules_are_added() {
        let mut interner = NameInterner::new();

        let upper = interner.intern_raw("FOO", NameKind::Constant);
        let lower = interner.intern_raw("foo", NameKind::Constant);

        assert_ne!(upper, lower);
        assert_eq!(interner.len(), 2);
    }
}
