//! HIR module container.

use crate::hir::arena::Arena;
use crate::hir::attributes::HirAttribute;
use crate::hir::class_like::{
    HirClassConst, HirClassLike, HirEnumCase, HirMethod, HirProperty, HirTraitUse,
};
use crate::hir::const_expr::ConstExpr;
use crate::hir::decl::HirDecl;
use crate::hir::declare::{FileDirectives, HirDeclare};
use crate::hir::expr::HirExpr;
use crate::hir::ids::ClassLikeId;
use crate::hir::ids::{
    AttributeId, ConstExprId, ConstId, DeclId, EnumCaseId, ExprId, MethodId, NamespaceId,
    PropertyId, StmtId, TraitUseId, TypeId,
};
use crate::hir::names::HirName;
use crate::hir::signatures::FunctionSignature;
use crate::hir::stmt::HirStmt;
use crate::hir::types::HirType;
use crate::scopes::ScopeArena;
use crate::symbols::declarations::DeclarationTable;
use crate::symbols::imports::ImportTable;
use crate::symbols::resolution::ResolvedNameRecord;
use php_source::TextRange;

/// HIR for one PHP source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirModule {
    root_kind: String,
    source_bytes: usize,
    namespaces: Arena<HirNamespaceBlock, NamespaceId>,
    scopes: ScopeArena,
    declarations: Arena<HirDecl, DeclId>,
    class_likes: Arena<HirClassLike, ClassLikeId>,
    trait_uses: Arena<HirTraitUse, TraitUseId>,
    enum_cases: Arena<HirEnumCase, EnumCaseId>,
    methods: Arena<HirMethod, MethodId>,
    properties: Arena<HirProperty, PropertyId>,
    class_consts: Arena<HirClassConst, ConstId>,
    declaration_table: DeclarationTable,
    statements: Arena<HirStmt, StmtId>,
    expressions: Arena<HirExpr, ExprId>,
    const_exprs: Arena<ConstExpr, ConstExprId>,
    types: Arena<HirType, TypeId>,
    signatures: Vec<FunctionSignature>,
    attributes: Arena<HirAttribute, AttributeId>,
    declares: Vec<HirDeclare>,
    file_directives: FileDirectives,
}

impl HirModule {
    /// Creates an empty HIR module anchored to the parsed source file.
    #[must_use]
    pub fn new(root_kind: impl Into<String>, source_bytes: usize) -> Self {
        Self {
            root_kind: root_kind.into(),
            source_bytes,
            namespaces: Arena::new(),
            scopes: ScopeArena::new(),
            declarations: Arena::new(),
            class_likes: Arena::new(),
            trait_uses: Arena::new(),
            enum_cases: Arena::new(),
            methods: Arena::new(),
            properties: Arena::new(),
            class_consts: Arena::new(),
            declaration_table: DeclarationTable::new(),
            statements: Arena::new(),
            expressions: Arena::new(),
            const_exprs: Arena::new(),
            types: Arena::new(),
            signatures: Vec::new(),
            attributes: Arena::new(),
            declares: Vec::new(),
            file_directives: FileDirectives::default(),
        }
    }

    /// Returns the CST root kind used as the module anchor.
    #[must_use]
    pub fn root_kind(&self) -> &str {
        &self.root_kind
    }

    /// Returns source size in bytes.
    #[must_use]
    pub const fn source_bytes(&self) -> usize {
        self.source_bytes
    }

    /// Returns namespace-block arena.
    #[must_use]
    pub const fn namespaces(&self) -> &Arena<HirNamespaceBlock, NamespaceId> {
        &self.namespaces
    }

    /// Returns mutable namespace-block arena.
    #[must_use]
    pub fn namespaces_mut(&mut self) -> &mut Arena<HirNamespaceBlock, NamespaceId> {
        &mut self.namespaces
    }

    /// Returns the lexical scope arena.
    #[must_use]
    pub const fn scopes(&self) -> &ScopeArena {
        &self.scopes
    }

    /// Returns the mutable lexical scope arena.
    #[must_use]
    pub fn scopes_mut(&mut self) -> &mut ScopeArena {
        &mut self.scopes
    }

    /// Returns declaration arena.
    #[must_use]
    pub const fn declarations(&self) -> &Arena<HirDecl, DeclId> {
        &self.declarations
    }

    /// Returns mutable declaration arena.
    #[must_use]
    pub fn declarations_mut(&mut self) -> &mut Arena<HirDecl, DeclId> {
        &mut self.declarations
    }

    /// Returns class-like arena.
    #[must_use]
    pub const fn class_likes(&self) -> &Arena<HirClassLike, ClassLikeId> {
        &self.class_likes
    }

    /// Returns mutable class-like arena.
    #[must_use]
    pub fn class_likes_mut(&mut self) -> &mut Arena<HirClassLike, ClassLikeId> {
        &mut self.class_likes
    }

    /// Returns trait-use arena.
    #[must_use]
    pub const fn trait_uses(&self) -> &Arena<HirTraitUse, TraitUseId> {
        &self.trait_uses
    }

    /// Returns mutable trait-use arena.
    #[must_use]
    pub fn trait_uses_mut(&mut self) -> &mut Arena<HirTraitUse, TraitUseId> {
        &mut self.trait_uses
    }

    /// Returns enum-case arena.
    #[must_use]
    pub const fn enum_cases(&self) -> &Arena<HirEnumCase, EnumCaseId> {
        &self.enum_cases
    }

    /// Returns mutable enum-case arena.
    #[must_use]
    pub fn enum_cases_mut(&mut self) -> &mut Arena<HirEnumCase, EnumCaseId> {
        &mut self.enum_cases
    }

    /// Returns method arena.
    #[must_use]
    pub const fn methods(&self) -> &Arena<HirMethod, MethodId> {
        &self.methods
    }

    /// Returns mutable method arena.
    #[must_use]
    pub fn methods_mut(&mut self) -> &mut Arena<HirMethod, MethodId> {
        &mut self.methods
    }

    /// Returns property arena.
    #[must_use]
    pub const fn properties(&self) -> &Arena<HirProperty, PropertyId> {
        &self.properties
    }

    /// Returns mutable property arena.
    #[must_use]
    pub fn properties_mut(&mut self) -> &mut Arena<HirProperty, PropertyId> {
        &mut self.properties
    }

    /// Returns class-constant arena.
    #[must_use]
    pub const fn class_consts(&self) -> &Arena<HirClassConst, ConstId> {
        &self.class_consts
    }

    /// Returns mutable class-constant arena.
    #[must_use]
    pub fn class_consts_mut(&mut self) -> &mut Arena<HirClassConst, ConstId> {
        &mut self.class_consts
    }

    /// Returns the source-file declaration table.
    #[must_use]
    pub const fn declaration_table(&self) -> &DeclarationTable {
        &self.declaration_table
    }

    /// Returns the mutable source-file declaration table.
    #[must_use]
    pub fn declaration_table_mut(&mut self) -> &mut DeclarationTable {
        &mut self.declaration_table
    }

    /// Returns statement arena.
    #[must_use]
    pub const fn statements(&self) -> &Arena<HirStmt, StmtId> {
        &self.statements
    }

    /// Returns mutable statement arena.
    #[must_use]
    pub fn statements_mut(&mut self) -> &mut Arena<HirStmt, StmtId> {
        &mut self.statements
    }

    /// Returns expression arena.
    #[must_use]
    pub const fn expressions(&self) -> &Arena<HirExpr, ExprId> {
        &self.expressions
    }

    /// Returns mutable expression arena.
    #[must_use]
    pub fn expressions_mut(&mut self) -> &mut Arena<HirExpr, ExprId> {
        &mut self.expressions
    }

    /// Returns constant-expression candidate arena.
    #[must_use]
    pub const fn const_exprs(&self) -> &Arena<ConstExpr, ConstExprId> {
        &self.const_exprs
    }

    /// Returns mutable constant-expression candidate arena.
    #[must_use]
    pub fn const_exprs_mut(&mut self) -> &mut Arena<ConstExpr, ConstExprId> {
        &mut self.const_exprs
    }

    /// Returns type arena.
    #[must_use]
    pub const fn types(&self) -> &Arena<HirType, TypeId> {
        &self.types
    }

    /// Returns mutable type arena.
    #[must_use]
    pub fn types_mut(&mut self) -> &mut Arena<HirType, TypeId> {
        &mut self.types
    }

    /// Returns lowered function-like signatures.
    #[must_use]
    pub fn signatures(&self) -> &[FunctionSignature] {
        &self.signatures
    }

    /// Appends a lowered function-like signature.
    pub fn push_signature(&mut self, signature: FunctionSignature) {
        self.signatures.push(signature);
    }

    /// Returns attribute arena.
    #[must_use]
    pub const fn attributes(&self) -> &Arena<HirAttribute, AttributeId> {
        &self.attributes
    }

    /// Returns mutable attribute arena.
    #[must_use]
    pub fn attributes_mut(&mut self) -> &mut Arena<HirAttribute, AttributeId> {
        &mut self.attributes
    }

    /// Returns lowered declare-statement metadata.
    #[must_use]
    pub fn declares(&self) -> &[HirDeclare] {
        &self.declares
    }

    /// Appends declare-statement metadata.
    pub fn push_declare(&mut self, declare: HirDeclare) {
        self.declares.push(declare);
    }

    /// Returns file-level directive summary.
    #[must_use]
    pub const fn file_directives(&self) -> &FileDirectives {
        &self.file_directives
    }

    /// Returns mutable file-level directive summary.
    #[must_use]
    pub fn file_directives_mut(&mut self) -> &mut FileDirectives {
        &mut self.file_directives
    }
}

/// Namespace block collected from top-level PHP source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirNamespaceBlock {
    name: Option<NamespaceName>,
    form: NamespaceForm,
    span: TextRange,
    scope_id: Option<crate::hir::ScopeId>,
    items: Vec<TopLevelItem>,
    imports: ImportTable,
    resolved_names: Vec<ResolvedNameRecord>,
}

impl HirNamespaceBlock {
    /// Creates a namespace block.
    #[must_use]
    pub fn new(name: Option<NamespaceName>, form: NamespaceForm, span: TextRange) -> Self {
        Self {
            name,
            form,
            span,
            scope_id: None,
            items: Vec::new(),
            imports: ImportTable::new(),
            resolved_names: Vec::new(),
        }
    }

    /// Returns namespace name, or `None` for the global namespace.
    #[must_use]
    pub const fn name(&self) -> Option<&NamespaceName> {
        self.name.as_ref()
    }

    /// Returns namespace declaration form.
    #[must_use]
    pub const fn form(&self) -> NamespaceForm {
        self.form
    }

    /// Returns source span for the block.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }

    /// Returns the namespace scope ID, when collection has assigned one.
    #[must_use]
    pub const fn scope_id(&self) -> Option<crate::hir::ScopeId> {
        self.scope_id
    }

    /// Assigns the namespace scope ID.
    pub fn set_scope_id(&mut self, scope_id: crate::hir::ScopeId) {
        self.scope_id = Some(scope_id);
    }

    /// Returns collected top-level items in this namespace block.
    #[must_use]
    pub fn items(&self) -> &[TopLevelItem] {
        &self.items
    }

    /// Returns imports collected for this namespace block.
    #[must_use]
    pub const fn imports(&self) -> &ImportTable {
        &self.imports
    }

    /// Returns mutable imports collected for this namespace block.
    #[must_use]
    pub fn imports_mut(&mut self) -> &mut ImportTable {
        &mut self.imports
    }

    /// Returns resolved source-name records for this namespace block.
    #[must_use]
    pub fn resolved_names(&self) -> &[ResolvedNameRecord] {
        &self.resolved_names
    }

    /// Pushes a top-level item and expands the block span to include it.
    pub fn push_item(&mut self, item: TopLevelItem) {
        self.span = cover_ranges(self.span, item.span());
        self.items.push(item);
    }

    /// Records a resolved source name for debug JSON and fixtures.
    pub fn push_resolved_name(&mut self, resolved_name: ResolvedNameRecord) {
        self.resolved_names.push(resolved_name);
    }
}

/// Namespace declaration form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceForm {
    /// Synthetic global namespace for files without explicit namespace blocks.
    Global,
    /// Braced namespace syntax: `namespace A { ... }`.
    Braced,
    /// Unbraced namespace syntax: `namespace A;`.
    Unbraced,
}

impl NamespaceForm {
    /// Returns the stable lowercase JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Braced => "braced",
            Self::Unbraced => "unbraced",
        }
    }
}

/// Namespace name text as written in source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamespaceName {
    name: HirName,
}

impl NamespaceName {
    /// Creates a namespace name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: HirName::new(name),
        }
    }

    /// Returns the original namespace name text.
    #[must_use]
    pub fn text(&self) -> &str {
        self.name.text()
    }
}

/// Top-level item collected inside a namespace block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopLevelItem {
    kind: TopLevelItemKind,
    span: TextRange,
}

impl TopLevelItem {
    /// Creates a top-level item.
    #[must_use]
    pub const fn new(kind: TopLevelItemKind, span: TextRange) -> Self {
        Self { kind, span }
    }

    /// Returns item kind.
    #[must_use]
    pub const fn kind(&self) -> TopLevelItemKind {
        self.kind
    }

    /// Returns item span.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Coarse top-level item families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopLevelItemKind {
    /// Inline HTML segment.
    InlineHtml,
    /// `declare` statement.
    Declare,
    /// Import declaration.
    Use,
    /// Constant declaration.
    Const,
    /// Function declaration.
    Function,
    /// Class declaration.
    Class,
    /// Interface declaration.
    Interface,
    /// Trait declaration.
    Trait,
    /// Enum declaration.
    Enum,
    /// Other executable top-level statement.
    Statement,
    /// Parser recovery or unknown top-level node.
    Unknown,
}

impl TopLevelItemKind {
    /// Returns the stable lowercase JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineHtml => "inline_html",
            Self::Declare => "declare",
            Self::Use => "use",
            Self::Const => "const",
            Self::Function => "function",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::Statement => "statement",
            Self::Unknown => "unknown",
        }
    }
}

fn cover_ranges(left: TextRange, right: TextRange) -> TextRange {
    let start = left.start().to_usize().min(right.start().to_usize());
    let end = left.end().to_usize().max(right.end().to_usize());
    TextRange::new(start, end)
}
