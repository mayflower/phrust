//! PHP import table for one namespace block.

use std::collections::HashMap;

use php_ast::{AstNode, Name, UseDecl};
use php_source::TextRange;
use php_syntax::SyntaxElement;

use crate::hir::{FullyQualifiedName, NameKind, QualifiedName};

/// PHP import namespace.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImportKind {
    /// Class-like/default import: `use Foo\Bar;`.
    ClassLike,
    /// Function import: `use function Foo\bar;`.
    Function,
    /// Constant import: `use const Foo\BAR;`.
    Const,
}

impl ImportKind {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClassLike => "class_like",
            Self::Function => "function",
            Self::Const => "const",
        }
    }

    /// Returns the name canonicalization category for aliases of this import.
    #[must_use]
    pub const fn alias_name_kind(self) -> NameKind {
        match self {
            Self::ClassLike => NameKind::ClassLike,
            Self::Function => NameKind::Function,
            Self::Const => NameKind::Constant,
        }
    }
}

/// One import entry collected from a `use` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportEntry {
    kind: ImportKind,
    name: FullyQualifiedName,
    alias: String,
    alias_canonical: String,
    explicit_alias: bool,
    span: TextRange,
}

impl ImportEntry {
    /// Creates an import entry.
    #[must_use]
    pub fn new(
        kind: ImportKind,
        name: FullyQualifiedName,
        alias: impl Into<String>,
        explicit_alias: bool,
        span: TextRange,
    ) -> Self {
        let alias = alias.into();
        let alias_canonical = if kind.alias_name_kind().is_case_insensitive() {
            alias.to_ascii_lowercase()
        } else {
            alias.clone()
        };
        Self {
            kind,
            name,
            alias,
            alias_canonical,
            explicit_alias,
            span,
        }
    }

    /// Returns import kind.
    #[must_use]
    pub const fn kind(&self) -> ImportKind {
        self.kind
    }

    /// Returns imported FQN.
    #[must_use]
    pub const fn name(&self) -> &FullyQualifiedName {
        &self.name
    }

    /// Returns source alias spelling.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Returns canonical alias spelling for duplicate checks and lookup.
    #[must_use]
    pub fn alias_canonical(&self) -> &str {
        &self.alias_canonical
    }

    /// Returns true if the alias came from `as`.
    #[must_use]
    pub const fn is_explicit_alias(&self) -> bool {
        self.explicit_alias
    }

    /// Returns source span for the imported clause.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Duplicate import alias details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateImportAlias {
    kind: ImportKind,
    alias: String,
    previous_span: TextRange,
    duplicate_span: TextRange,
}

impl DuplicateImportAlias {
    /// Returns duplicate import kind.
    #[must_use]
    pub const fn kind(&self) -> ImportKind {
        self.kind
    }

    /// Returns duplicate alias spelling.
    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// Returns previous alias span.
    #[must_use]
    pub const fn previous_span(&self) -> TextRange {
        self.previous_span
    }

    /// Returns duplicate alias span.
    #[must_use]
    pub const fn duplicate_span(&self) -> TextRange {
        self.duplicate_span
    }
}

/// Import table scoped to one namespace block.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImportTable {
    entries: Vec<ImportEntry>,
    class_like_aliases: HashMap<String, usize>,
    function_aliases: HashMap<String, usize>,
    const_aliases: HashMap<String, usize>,
}

impl ImportTable {
    /// Creates an empty import table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an import entry or returns duplicate-alias details.
    pub fn insert(&mut self, entry: ImportEntry) -> Result<(), DuplicateImportAlias> {
        let aliases = match entry.kind {
            ImportKind::ClassLike => &mut self.class_like_aliases,
            ImportKind::Function => &mut self.function_aliases,
            ImportKind::Const => &mut self.const_aliases,
        };

        if let Some(previous_index) = aliases.get(entry.alias_canonical()) {
            let previous = &self.entries[*previous_index];
            return Err(DuplicateImportAlias {
                kind: entry.kind,
                alias: entry.alias.clone(),
                previous_span: previous.span(),
                duplicate_span: entry.span(),
            });
        }

        let index = self.entries.len();
        aliases.insert(entry.alias_canonical.clone(), index);
        self.entries.push(entry);
        Ok(())
    }

    /// Looks up an import by source alias.
    #[must_use]
    pub fn lookup(&self, kind: ImportKind, alias: &str) -> Option<&ImportEntry> {
        let canonical = if kind.alias_name_kind().is_case_insensitive() {
            alias.to_ascii_lowercase()
        } else {
            alias.to_owned()
        };
        let aliases = match kind {
            ImportKind::ClassLike => &self.class_like_aliases,
            ImportKind::Function => &self.function_aliases,
            ImportKind::Const => &self.const_aliases,
        };
        aliases
            .get(&canonical)
            .and_then(|index| self.entries.get(*index))
    }

    /// Returns imports in source order.
    #[must_use]
    pub fn entries(&self) -> &[ImportEntry] {
        &self.entries
    }

    /// Returns true if no imports are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Creates an FQN by appending a relative name to a prefix name.
#[must_use]
pub fn append_import_name(prefix: &QualifiedName, suffix: &QualifiedName) -> FullyQualifiedName {
    let mut parts = prefix.parts().to_vec();
    parts.extend_from_slice(suffix.parts());
    FullyQualifiedName::from_parts(parts)
}

/// Collects import entries from one CST-backed `use` declaration.
#[must_use]
pub fn collect_use_imports(use_decl: UseDecl<'_>) -> Vec<ImportEntry> {
    let mut cursor = UseCursor::new(use_decl);
    cursor.skip_token("T_USE");
    let default_kind = cursor.take_import_kind().unwrap_or(ImportKind::ClassLike);
    let mut entries = Vec::new();

    while !cursor.is_done() && !cursor.at_token(";") {
        cursor.skip_token(",");
        if cursor.is_done() || cursor.at_token(";") {
            break;
        }
        entries.extend(cursor.parse_clause(default_kind, false, None));
        if !cursor.at_token(",") {
            break;
        }
    }

    entries
}

#[derive(Clone, Debug)]
struct ParsedImport {
    kind: ImportKind,
    name: QualifiedName,
    alias: String,
    explicit_alias: bool,
    span: TextRange,
}

#[derive(Clone, Debug)]
enum UseElement {
    Name(QualifiedName, TextRange),
    Token(String, String, TextRange),
}

struct UseCursor {
    elements: Vec<UseElement>,
    index: usize,
}

impl UseCursor {
    fn new(use_decl: UseDecl<'_>) -> Self {
        let mut elements = Vec::new();
        for child in use_decl.syntax().children() {
            match child {
                SyntaxElement::Node(node) => {
                    if let Some(name) = Name::cast(node) {
                        elements.push(UseElement::Name(
                            QualifiedName::from_ast_name(name),
                            name.text_range(),
                        ));
                    }
                }
                SyntaxElement::Token(token) if !token.kind().is_trivia() => {
                    elements.push(UseElement::Token(
                        token.kind().name(),
                        token.text().to_owned(),
                        token.text_range(),
                    ));
                }
                SyntaxElement::Token(_) => {}
            }
        }
        Self { elements, index: 0 }
    }

    fn is_done(&self) -> bool {
        self.index >= self.elements.len()
    }

    fn at_token(&self, text_or_kind: &str) -> bool {
        matches!(
            self.elements.get(self.index),
            Some(UseElement::Token(kind, text, _)) if kind == text_or_kind || text == text_or_kind
        )
    }

    fn peek_token(&self, offset: usize, text_or_kind: &str) -> bool {
        matches!(
            self.elements.get(self.index + offset),
            Some(UseElement::Token(kind, text, _)) if kind == text_or_kind || text == text_or_kind
        )
    }

    fn skip_token(&mut self, text_or_kind: &str) -> bool {
        if self.at_token(text_or_kind) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn take_import_kind(&mut self) -> Option<ImportKind> {
        if self.skip_token("T_FUNCTION") {
            Some(ImportKind::Function)
        } else if self.skip_token("T_CONST") {
            Some(ImportKind::Const)
        } else {
            None
        }
    }

    fn take_name(&mut self) -> Option<(QualifiedName, TextRange)> {
        match self.elements.get(self.index) {
            Some(UseElement::Name(name, span)) => {
                self.index += 1;
                Some((name.clone(), *span))
            }
            _ => None,
        }
    }

    fn take_alias(&mut self, span: TextRange) -> (Option<String>, bool, TextRange) {
        if !self.skip_token("T_AS") {
            return (None, false, span);
        }

        match self.elements.get(self.index) {
            Some(UseElement::Token(kind, text, alias_span)) if kind == "T_STRING" => {
                self.index += 1;
                (Some(text.clone()), true, cover_ranges(span, *alias_span))
            }
            _ => (None, true, span),
        }
    }

    fn parse_clause(
        &mut self,
        default_kind: ImportKind,
        allow_item_kind: bool,
        prefix: Option<&QualifiedName>,
    ) -> Vec<ImportEntry> {
        let parsed = match self.parse_single_clause(default_kind, allow_item_kind) {
            Some(parsed) => parsed,
            None => return Vec::new(),
        };

        if prefix.is_none() && self.at_token("\\") && self.peek_token(1, "{") {
            self.index += 2;
            let mut entries = Vec::new();
            while !self.is_done() && !self.at_token("}") && !self.at_token(";") {
                self.skip_token(",");
                if self.at_token("}") {
                    break;
                }
                entries.extend(self.parse_clause(default_kind, true, Some(&parsed.name)));
                self.skip_token(",");
            }
            self.skip_token("}");
            return entries;
        }

        let name = prefix
            .map(|base| append_import_name(base, &parsed.name))
            .unwrap_or_else(|| FullyQualifiedName::from_parts(parsed.name.parts().to_vec()));
        vec![ImportEntry::new(
            parsed.kind,
            name,
            parsed.alias,
            parsed.explicit_alias,
            parsed.span,
        )]
    }

    fn parse_single_clause(
        &mut self,
        default_kind: ImportKind,
        allow_item_kind: bool,
    ) -> Option<ParsedImport> {
        let kind = if allow_item_kind {
            self.take_import_kind().unwrap_or(default_kind)
        } else {
            default_kind
        };
        let (name, span) = self.take_name()?;
        let (explicit_alias, alias, span) = {
            let (alias, explicit_alias, span) = self.take_alias(span);
            (explicit_alias, alias, span)
        };
        let alias = alias.unwrap_or_else(|| {
            name.parts()
                .last()
                .map(|part| part.original().to_owned())
                .unwrap_or_default()
        });

        Some(ParsedImport {
            kind,
            name,
            alias,
            explicit_alias,
            span,
        })
    }
}

fn cover_ranges(left: TextRange, right: TextRange) -> TextRange {
    let start = left.start().to_usize().min(right.start().to_usize());
    let end = left.end().to_usize().max(right.end().to_usize());
    TextRange::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::{ImportKind, ImportTable, collect_use_imports};
    use php_ast::{UseDecl, descendant_nodes};
    use php_syntax::parse_source_file;

    fn imports(source: &str) -> Vec<super::ImportEntry> {
        let parse = parse_source_file(source);
        let use_decl = descendant_nodes::<UseDecl<'_>>(parse.root())
            .next()
            .expect("use declaration");
        collect_use_imports(use_decl)
    }

    #[test]
    fn collects_class_import_with_explicit_alias() {
        let imports = imports("<?php use Vendor\\Package\\ClassName as Alias;");

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].kind(), ImportKind::ClassLike);
        assert_eq!(
            imports[0].name().canonical(crate::hir::NameKind::ClassLike),
            "vendor\\package\\classname"
        );
        assert_eq!(imports[0].alias(), "Alias");
        assert!(imports[0].is_explicit_alias());
    }

    #[test]
    fn collects_function_and_const_import_kinds() {
        let function = imports("<?php use function Vendor\\Package\\helper;");
        let konst = imports("<?php use const Vendor\\Package\\VALUE;");

        assert_eq!(function[0].kind(), ImportKind::Function);
        assert_eq!(function[0].alias(), "helper");
        assert_eq!(konst[0].kind(), ImportKind::Const);
        assert_eq!(konst[0].alias(), "VALUE");
    }

    #[test]
    fn collects_group_and_mixed_group_imports() {
        let grouped = imports("<?php use Vendor\\Package\\{ClassA, ClassB as B};");
        let mixed = imports("<?php use Vendor\\Package\\{function helper, const VALUE};");

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].alias(), "ClassA");
        assert_eq!(grouped[1].alias(), "B");
        assert_eq!(mixed.len(), 2);
        assert_eq!(mixed[0].kind(), ImportKind::Function);
        assert_eq!(mixed[0].alias(), "helper");
        assert_eq!(mixed[1].kind(), ImportKind::Const);
        assert_eq!(mixed[1].alias(), "VALUE");
    }

    #[test]
    fn duplicate_aliases_are_tracked_per_import_kind() {
        let class_imports = imports("<?php use Vendor\\One\\Thing, Vendor\\Two\\Thing as Thing;");
        let function_imports =
            imports("<?php use function Vendor\\One\\thing, Vendor\\Two\\thing;");
        let const_imports = imports("<?php use const Vendor\\One\\THING, Vendor\\Two\\THING;");

        let mut table = ImportTable::new();
        table
            .insert(class_imports[0].clone())
            .expect("first class import");
        assert!(table.insert(class_imports[1].clone()).is_err());

        let mut table = ImportTable::new();
        table
            .insert(function_imports[0].clone())
            .expect("first function import");
        assert!(table.insert(function_imports[1].clone()).is_err());

        let mut table = ImportTable::new();
        table
            .insert(const_imports[0].clone())
            .expect("first const import");
        assert!(table.insert(const_imports[1].clone()).is_err());
    }
}
