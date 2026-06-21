//! PHP source-name resolution over collected imports.

use php_source::TextRange;

use crate::hir::{FullyQualifiedName, NameKind, NamePart, NamespaceName, QualifiedName};
use crate::symbols::imports::{ImportKind, ImportTable};

/// Context in which a source name is resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolveContext {
    /// Class-like name.
    ClassLike,
    /// Function call name.
    FunctionCall,
    /// Constant fetch name.
    ConstantFetch,
    /// Namespace name.
    NamespaceName,
    /// Attribute class name.
    AttributeClass,
    /// Type annotation name.
    TypeName,
}

impl ResolveContext {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClassLike => "class_like",
            Self::FunctionCall => "function_call",
            Self::ConstantFetch => "constant_fetch",
            Self::NamespaceName => "namespace_name",
            Self::AttributeClass => "attribute_class",
            Self::TypeName => "type_name",
        }
    }

    fn import_kind(self) -> Option<ImportKind> {
        match self {
            Self::ClassLike | Self::AttributeClass | Self::TypeName => Some(ImportKind::ClassLike),
            Self::FunctionCall => Some(ImportKind::Function),
            Self::ConstantFetch => Some(ImportKind::Const),
            Self::NamespaceName => None,
        }
    }

    fn name_kind(self) -> NameKind {
        match self {
            Self::ClassLike | Self::AttributeClass | Self::TypeName => NameKind::ClassLike,
            Self::FunctionCall => NameKind::Function,
            Self::ConstantFetch => NameKind::Constant,
            Self::NamespaceName => NameKind::Namespace,
        }
    }
}

/// Resolution result for a source name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedName {
    /// Statically resolved FQN.
    FullyQualified(FullyQualifiedName),
    /// Namespaced function/constant lookup may fall back at runtime.
    MaybeRuntimeFallback {
        /// Namespaced candidate checked first.
        namespaced: FullyQualifiedName,
        /// Global fallback candidate.
        fallback: FullyQualifiedName,
    },
    /// Dynamic name expression.
    Dynamic,
    /// Name was not statically bound by this pass.
    Unresolved,
}

impl ResolvedName {
    /// Returns stable classification text.
    #[must_use]
    pub const fn classification(&self) -> &'static str {
        match self {
            Self::FullyQualified(_) => "fully_qualified",
            Self::MaybeRuntimeFallback { .. } => "maybe_runtime_fallback",
            Self::Dynamic => "dynamic",
            Self::Unresolved => "unresolved",
        }
    }
}

/// Resolved source-name record emitted for fixtures and debug JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedNameRecord {
    source: QualifiedName,
    context: ResolveContext,
    result: ResolvedName,
    span: TextRange,
}

impl ResolvedNameRecord {
    /// Creates a resolved-name record.
    #[must_use]
    pub const fn new(
        source: QualifiedName,
        context: ResolveContext,
        result: ResolvedName,
        span: TextRange,
    ) -> Self {
        Self {
            source,
            context,
            result,
            span,
        }
    }

    /// Returns source name.
    #[must_use]
    pub const fn source(&self) -> &QualifiedName {
        &self.source
    }

    /// Returns resolution context.
    #[must_use]
    pub const fn context(&self) -> ResolveContext {
        self.context
    }

    /// Returns resolution result.
    #[must_use]
    pub const fn result(&self) -> &ResolvedName {
        &self.result
    }

    /// Returns source span.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Resolves names in one namespace block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameResolver<'a> {
    current_namespace: Option<&'a NamespaceName>,
    imports: &'a ImportTable,
}

impl<'a> NameResolver<'a> {
    /// Creates a resolver for one namespace block.
    #[must_use]
    pub const fn new(
        current_namespace: Option<&'a NamespaceName>,
        imports: &'a ImportTable,
    ) -> Self {
        Self {
            current_namespace,
            imports,
        }
    }

    /// Resolves a source name in a specific context.
    #[must_use]
    pub fn resolve(&self, name: &QualifiedName, context: ResolveContext) -> ResolvedName {
        if name.parts().is_empty() {
            return ResolvedName::Dynamic;
        }

        if name.is_fully_qualified() {
            return ResolvedName::FullyQualified(FullyQualifiedName::from_parts(
                name.parts().to_vec(),
            ));
        }

        if name.is_namespace_relative() {
            return ResolvedName::FullyQualified(self.with_current_namespace(name.parts()));
        }

        let import_kind = context.import_kind();
        if let Some(first) = name.parts().first()
            && let Some(kind) = import_kind
            && let Some(import) = self.imports.lookup(kind, first.original())
        {
            let mut parts = import.name().parts().to_vec();
            parts.extend_from_slice(&name.parts()[1..]);
            return ResolvedName::FullyQualified(FullyQualifiedName::from_parts(parts));
        }

        match context {
            ResolveContext::FunctionCall | ResolveContext::ConstantFetch
                if name.parts().len() == 1 && self.current_namespace.is_some() =>
            {
                ResolvedName::MaybeRuntimeFallback {
                    namespaced: self.with_current_namespace(name.parts()),
                    fallback: FullyQualifiedName::from_parts(name.parts().to_vec()),
                }
            }
            ResolveContext::FunctionCall | ResolveContext::ConstantFetch => {
                ResolvedName::FullyQualified(FullyQualifiedName::from_parts(name.parts().to_vec()))
            }
            ResolveContext::ClassLike
            | ResolveContext::AttributeClass
            | ResolveContext::TypeName
            | ResolveContext::NamespaceName => {
                ResolvedName::FullyQualified(self.with_current_namespace(name.parts()))
            }
        }
    }

    /// Returns a dynamic marker.
    #[must_use]
    pub const fn resolve_dynamic(&self) -> ResolvedName {
        ResolvedName::Dynamic
    }

    /// Returns the name kind for canonical output in this context.
    #[must_use]
    pub fn name_kind(context: ResolveContext) -> NameKind {
        context.name_kind()
    }

    fn with_current_namespace(&self, parts: &[NamePart]) -> FullyQualifiedName {
        let mut full_parts = self
            .current_namespace
            .map(|namespace| QualifiedName::parse(namespace.text()).parts().to_vec())
            .unwrap_or_default();
        full_parts.extend_from_slice(parts);
        FullyQualifiedName::from_parts(full_parts)
    }
}

#[cfg(test)]
mod tests {
    use super::{NameResolver, ResolveContext, ResolvedName};
    use crate::hir::{FullyQualifiedName, NamespaceName, QualifiedName};
    use crate::symbols::imports::{ImportEntry, ImportKind, ImportTable};
    use php_source::TextRange;

    #[test]
    fn resolves_class_import_aliases() {
        let namespace = NamespaceName::new("App");
        let mut imports = ImportTable::new();
        imports
            .insert(ImportEntry::new(
                ImportKind::ClassLike,
                FullyQualifiedName::parse("Vendor\\Thing"),
                "Thing",
                false,
                TextRange::new(0, 12),
            ))
            .expect("insert import");
        let resolver = NameResolver::new(Some(&namespace), &imports);

        let resolved = resolver.resolve(&QualifiedName::parse("Thing"), ResolveContext::ClassLike);

        assert_eq!(
            resolved,
            ResolvedName::FullyQualified(FullyQualifiedName::parse("Vendor\\Thing"))
        );
    }

    #[test]
    fn replaces_first_segment_of_qualified_class_import_aliases() {
        let namespace = NamespaceName::new("App");
        let mut imports = ImportTable::new();
        imports
            .insert(ImportEntry::new(
                ImportKind::ClassLike,
                FullyQualifiedName::parse("Vendor\\Package"),
                "Package",
                false,
                TextRange::new(0, 14),
            ))
            .expect("insert import");
        let resolver = NameResolver::new(Some(&namespace), &imports);

        let resolved = resolver.resolve(
            &QualifiedName::parse("Package\\Thing"),
            ResolveContext::ClassLike,
        );

        assert_eq!(
            resolved,
            ResolvedName::FullyQualified(FullyQualifiedName::parse("Vendor\\Package\\Thing"))
        );
    }

    #[test]
    fn resolves_function_and_const_import_aliases() {
        let namespace = NamespaceName::new("App");
        let mut imports = ImportTable::new();
        imports
            .insert(ImportEntry::new(
                ImportKind::Function,
                FullyQualifiedName::parse("Vendor\\Package\\helper"),
                "helper",
                false,
                TextRange::new(0, 22),
            ))
            .expect("insert function import");
        imports
            .insert(ImportEntry::new(
                ImportKind::Const,
                FullyQualifiedName::parse("Vendor\\Package\\VALUE"),
                "VALUE",
                false,
                TextRange::new(23, 44),
            ))
            .expect("insert const import");
        let resolver = NameResolver::new(Some(&namespace), &imports);

        assert_eq!(
            resolver.resolve(
                &QualifiedName::parse("helper"),
                ResolveContext::FunctionCall
            ),
            ResolvedName::FullyQualified(FullyQualifiedName::parse("Vendor\\Package\\helper"))
        );
        assert_eq!(
            resolver.resolve(
                &QualifiedName::parse("VALUE"),
                ResolveContext::ConstantFetch
            ),
            ResolvedName::FullyQualified(FullyQualifiedName::parse("Vendor\\Package\\VALUE"))
        );
    }

    #[test]
    fn marks_unqualified_namespaced_function_as_runtime_fallback() {
        let namespace = NamespaceName::new("App");
        let imports = ImportTable::new();
        let resolver = NameResolver::new(Some(&namespace), &imports);

        let resolved = resolver.resolve(
            &QualifiedName::parse("helper"),
            ResolveContext::FunctionCall,
        );

        assert_eq!(
            resolved,
            ResolvedName::MaybeRuntimeFallback {
                namespaced: FullyQualifiedName::parse("App\\helper"),
                fallback: FullyQualifiedName::parse("helper"),
            }
        );
    }

    #[test]
    fn handles_fully_qualified_namespace_relative_and_dynamic_names() {
        let namespace = NamespaceName::new("App");
        let imports = ImportTable::new();
        let resolver = NameResolver::new(Some(&namespace), &imports);

        assert_eq!(
            resolver.resolve(
                &QualifiedName::parse("\\Vendor\\Thing"),
                ResolveContext::ClassLike,
            ),
            ResolvedName::FullyQualified(FullyQualifiedName::parse("Vendor\\Thing"))
        );
        assert_eq!(
            resolver.resolve(
                &QualifiedName::parse("namespace\\Thing"),
                ResolveContext::ClassLike,
            ),
            ResolvedName::FullyQualified(FullyQualifiedName::parse("App\\Thing"))
        );
        assert_eq!(resolver.resolve_dynamic(), ResolvedName::Dynamic);
    }
}
