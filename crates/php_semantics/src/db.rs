//! Semantic frontend database and source maps.

use crate::hir::{Arena, HirModule, ModuleId};
use crate::hir::{
    AttributeId, ClassLikeId, ConstExprId, ConstId, DeclId, EnumCaseId, ExprId, FunctionId,
    MethodId, NameId, NamespaceId, ParamId, PropertyId, ScopeId, StmtId, SymbolId, TraitUseId,
    TypeId,
};
use php_source::TextRange;

/// A typed HIR or semantic ID that can be mapped back to source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceMappedId {
    /// Module ID.
    Module(ModuleId),
    /// Namespace ID.
    Namespace(NamespaceId),
    /// Declaration ID.
    Decl(DeclId),
    /// Function ID.
    Function(FunctionId),
    /// Class-like ID.
    ClassLike(ClassLikeId),
    /// Trait-use declaration ID.
    TraitUse(TraitUseId),
    /// Enum case ID.
    EnumCase(EnumCaseId),
    /// Method ID.
    Method(MethodId),
    /// Property ID.
    Property(PropertyId),
    /// Constant ID.
    Const(ConstId),
    /// Parameter ID.
    Param(ParamId),
    /// Expression ID.
    Expr(ExprId),
    /// Statement ID.
    Stmt(StmtId),
    /// Type ID.
    Type(TypeId),
    /// Constant-expression candidate ID.
    ConstExpr(ConstExprId),
    /// Attribute ID.
    Attribute(AttributeId),
    /// Scope ID.
    Scope(ScopeId),
    /// Symbol ID.
    Symbol(SymbolId),
    /// Source-level name ID.
    Name(NameId),
}

impl From<ModuleId> for SourceMappedId {
    fn from(id: ModuleId) -> Self {
        Self::Module(id)
    }
}

impl From<NamespaceId> for SourceMappedId {
    fn from(id: NamespaceId) -> Self {
        Self::Namespace(id)
    }
}

impl From<ExprId> for SourceMappedId {
    fn from(id: ExprId) -> Self {
        Self::Expr(id)
    }
}

impl From<StmtId> for SourceMappedId {
    fn from(id: StmtId) -> Self {
        Self::Stmt(id)
    }
}

impl From<DeclId> for SourceMappedId {
    fn from(id: DeclId) -> Self {
        Self::Decl(id)
    }
}

impl From<FunctionId> for SourceMappedId {
    fn from(id: FunctionId) -> Self {
        Self::Function(id)
    }
}

impl From<ClassLikeId> for SourceMappedId {
    fn from(id: ClassLikeId) -> Self {
        Self::ClassLike(id)
    }
}

impl From<TraitUseId> for SourceMappedId {
    fn from(id: TraitUseId) -> Self {
        Self::TraitUse(id)
    }
}

impl From<EnumCaseId> for SourceMappedId {
    fn from(id: EnumCaseId) -> Self {
        Self::EnumCase(id)
    }
}

impl From<MethodId> for SourceMappedId {
    fn from(id: MethodId) -> Self {
        Self::Method(id)
    }
}

impl From<PropertyId> for SourceMappedId {
    fn from(id: PropertyId) -> Self {
        Self::Property(id)
    }
}

impl From<ConstId> for SourceMappedId {
    fn from(id: ConstId) -> Self {
        Self::Const(id)
    }
}

impl From<ParamId> for SourceMappedId {
    fn from(id: ParamId) -> Self {
        Self::Param(id)
    }
}

impl From<TypeId> for SourceMappedId {
    fn from(id: TypeId) -> Self {
        Self::Type(id)
    }
}

impl From<ConstExprId> for SourceMappedId {
    fn from(id: ConstExprId) -> Self {
        Self::ConstExpr(id)
    }
}

impl From<AttributeId> for SourceMappedId {
    fn from(id: AttributeId) -> Self {
        Self::Attribute(id)
    }
}

impl From<ScopeId> for SourceMappedId {
    fn from(id: ScopeId) -> Self {
        Self::Scope(id)
    }
}

impl From<SymbolId> for SourceMappedId {
    fn from(id: SymbolId) -> Self {
        Self::Symbol(id)
    }
}

impl From<NameId> for SourceMappedId {
    fn from(id: NameId) -> Self {
        Self::Name(id)
    }
}

/// Source ranges for HIR IDs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceMap {
    spans: Vec<(SourceMappedId, TextRange)>,
}

impl SourceMap {
    /// Records a source span for an ID.
    pub fn insert(&mut self, id: impl Into<SourceMappedId>, range: TextRange) {
        self.spans.push((id.into(), range));
    }

    /// Returns the latest span recorded for an ID.
    #[must_use]
    pub fn span(&self, id: impl Into<SourceMappedId>) -> Option<TextRange> {
        let id = id.into();
        self.spans
            .iter()
            .rev()
            .find_map(|(candidate, range)| (*candidate == id).then_some(*range))
    }

    /// Returns true when no spans are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }
}

/// Container for semantic frontend state produced for one analysis result.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrontendDatabase {
    modules: Arena<HirModule, ModuleId>,
    source_map: SourceMap,
}

impl FrontendDatabase {
    /// Creates an empty frontend database.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates a HIR module.
    pub fn add_module(&mut self, module: HirModule) -> ModuleId {
        self.modules.alloc(module)
    }

    /// Returns a module by ID.
    #[must_use]
    pub fn module(&self, id: ModuleId) -> Option<&HirModule> {
        self.modules.get(id)
    }

    /// Returns a mutable module by ID.
    #[must_use]
    pub fn module_mut(&mut self, id: ModuleId) -> Option<&mut HirModule> {
        self.modules.get_mut(id)
    }

    /// Returns all modules.
    #[must_use]
    pub const fn modules(&self) -> &Arena<HirModule, ModuleId> {
        &self.modules
    }

    /// Returns source map.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns mutable source map.
    #[must_use]
    pub fn source_map_mut(&mut self) -> &mut SourceMap {
        &mut self.source_map
    }
}
