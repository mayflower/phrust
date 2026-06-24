//! Lexical scope model for Semantic frontend compile-time analysis.

pub mod control_context;

use crate::hir::{Arena, ScopeId};
use php_source::TextRange;

/// Append-only arena for lexical scopes in one source file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopeArena {
    scopes: Arena<Scope, ScopeId>,
    root: Option<ScopeId>,
}

impl ScopeArena {
    /// Creates an empty scope arena.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates the file scope. A module has at most one root scope.
    pub fn alloc_root(
        &mut self,
        kind: ScopeKind,
        name: Option<String>,
        span: TextRange,
    ) -> ScopeId {
        let id = self.scopes.alloc(Scope::new(kind, None, name, span));
        self.root = Some(id);
        id
    }

    /// Allocates a child scope and updates the parent-child relation.
    pub fn alloc_child(
        &mut self,
        parent: ScopeId,
        kind: ScopeKind,
        name: Option<String>,
        span: TextRange,
    ) -> ScopeId {
        let id = self
            .scopes
            .alloc(Scope::new(kind, Some(parent), name, span));
        self.scopes[parent].push_child(id);
        id
    }

    /// Returns the root file scope.
    #[must_use]
    pub const fn root(&self) -> Option<ScopeId> {
        self.root
    }

    /// Returns a scope by ID.
    #[must_use]
    pub fn get(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Returns a mutable scope by ID.
    #[must_use]
    pub fn get_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id)
    }

    /// Iterates over scopes in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (ScopeId, &Scope)> {
        self.scopes.iter()
    }

    /// Returns true when no scopes are allocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }
}

/// Lexical scope families tracked by Semantic frontend.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeKind {
    /// Whole source file.
    File,
    /// Namespace block, including synthetic global namespace.
    Namespace,
    /// Named function declaration.
    Function,
    /// Closure expression.
    Closure,
    /// Arrow function expression.
    ArrowFunction,
    /// Class method declaration.
    Method,
    /// Class declaration.
    Class,
    /// Trait declaration.
    Trait,
    /// Interface declaration.
    Interface,
    /// Enum declaration.
    Enum,
}

impl ScopeKind {
    /// Returns the stable lowercase JSON/text spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Namespace => "namespace",
            Self::Function => "function",
            Self::Closure => "closure",
            Self::ArrowFunction => "arrow_function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Trait => "trait",
            Self::Interface => "interface",
            Self::Enum => "enum",
        }
    }
}

/// One lexical scope node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    kind: ScopeKind,
    parent: Option<ScopeId>,
    children: Vec<ScopeId>,
    name: Option<String>,
    span: TextRange,
    function_like: Option<FunctionLikeContext>,
    globals: Vec<VariableBinding>,
    statics: Vec<StaticLocalBinding>,
}

impl Scope {
    fn new(
        kind: ScopeKind,
        parent: Option<ScopeId>,
        name: Option<String>,
        span: TextRange,
    ) -> Self {
        Self {
            kind,
            parent,
            children: Vec::new(),
            name,
            span,
            function_like: None,
            globals: Vec::new(),
            statics: Vec::new(),
        }
    }

    /// Returns the scope kind.
    #[must_use]
    pub const fn kind(&self) -> ScopeKind {
        self.kind
    }

    /// Returns the parent scope ID.
    #[must_use]
    pub const fn parent(&self) -> Option<ScopeId> {
        self.parent
    }

    /// Returns child scope IDs in source/allocation order.
    #[must_use]
    pub fn children(&self) -> &[ScopeId] {
        &self.children
    }

    /// Returns the optional source name for named scopes.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the source span covered by the scope anchor.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }

    /// Returns function-like metadata for function, method, closure, or arrow scopes.
    #[must_use]
    pub const fn function_like(&self) -> Option<&FunctionLikeContext> {
        self.function_like.as_ref()
    }

    /// Returns `global` declarations recorded in this function-like scope.
    #[must_use]
    pub fn globals(&self) -> &[VariableBinding] {
        &self.globals
    }

    /// Returns `static` local declarations recorded in this function-like scope.
    #[must_use]
    pub fn statics(&self) -> &[StaticLocalBinding] {
        &self.statics
    }

    /// Sets function-like metadata.
    pub fn set_function_like(&mut self, context: FunctionLikeContext) {
        self.function_like = Some(context);
    }

    /// Records a `global` variable declaration.
    pub fn push_global(&mut self, binding: VariableBinding) {
        self.globals.push(binding);
    }

    /// Records a `static` local declaration.
    pub fn push_static(&mut self, binding: StaticLocalBinding) {
        self.statics.push(binding);
    }

    fn push_child(&mut self, child: ScopeId) {
        self.children.push(child);
    }
}

/// Metadata for a PHP function-like scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionLikeContext {
    kind: FunctionLikeKind,
    parameters: Vec<ParameterBinding>,
    captures: Vec<CaptureBinding>,
    capture_mode: Option<CaptureMode>,
}

impl FunctionLikeContext {
    /// Creates metadata for a named function or method.
    #[must_use]
    pub fn new(kind: FunctionLikeKind, parameters: Vec<ParameterBinding>) -> Self {
        Self {
            kind,
            parameters,
            captures: Vec::new(),
            capture_mode: None,
        }
    }

    /// Creates metadata for a closure with explicit `use` captures.
    #[must_use]
    pub fn closure(parameters: Vec<ParameterBinding>, captures: Vec<CaptureBinding>) -> Self {
        Self {
            kind: FunctionLikeKind::Closure,
            parameters,
            captures,
            capture_mode: None,
        }
    }

    /// Creates metadata for an arrow function with deferred implicit capture.
    #[must_use]
    pub fn arrow(parameters: Vec<ParameterBinding>) -> Self {
        Self {
            kind: FunctionLikeKind::ArrowFunction,
            parameters,
            captures: Vec::new(),
            capture_mode: Some(CaptureMode::ImplicitByValueDeferred),
        }
    }

    /// Returns the function-like kind.
    #[must_use]
    pub const fn kind(&self) -> FunctionLikeKind {
        self.kind
    }

    /// Returns registered parameters.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterBinding] {
        &self.parameters
    }

    /// Returns explicit closure-use captures.
    #[must_use]
    pub fn captures(&self) -> &[CaptureBinding] {
        &self.captures
    }

    /// Returns the aggregate capture mode, when one exists.
    #[must_use]
    pub const fn capture_mode(&self) -> Option<CaptureMode> {
        self.capture_mode
    }
}

/// Function-like scope families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionLikeKind {
    /// Named function.
    Function,
    /// Closure expression.
    Closure,
    /// Arrow function expression.
    ArrowFunction,
    /// Class method.
    Method,
}

impl FunctionLikeKind {
    /// Returns the stable lowercase JSON/text spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Closure => "closure",
            Self::ArrowFunction => "arrow_function",
            Self::Method => "method",
        }
    }
}

/// Capture mode marker. Semantic frontend records shape only, not runtime capture values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureMode {
    /// Explicit closure `use ($x)`.
    ExplicitByValue,
    /// Explicit closure `use (&$x)`.
    ExplicitByReference,
    /// Arrow-function implicit by-value capture, deferred to later checks.
    ImplicitByValueDeferred,
}

impl CaptureMode {
    /// Returns the stable lowercase JSON/text spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitByValue => "explicit_by_value",
            Self::ExplicitByReference => "explicit_by_reference",
            Self::ImplicitByValueDeferred => "implicit_by_value_deferred",
        }
    }
}

/// Function-like parameter declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterBinding {
    name: String,
    by_ref: bool,
    variadic: bool,
    span: TextRange,
}

impl ParameterBinding {
    /// Creates a parameter binding.
    #[must_use]
    pub fn new(name: impl Into<String>, by_ref: bool, variadic: bool, span: TextRange) -> Self {
        Self {
            name: name.into(),
            by_ref,
            variadic,
            span,
        }
    }

    /// Returns the parameter variable name including `$`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true when the parameter is by-reference.
    #[must_use]
    pub const fn is_by_ref(&self) -> bool {
        self.by_ref
    }

    /// Returns true when the parameter is variadic.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns the source span for the variable token.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Explicit closure-use capture declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureBinding {
    name: String,
    mode: CaptureMode,
    span: TextRange,
}

impl CaptureBinding {
    /// Creates a capture binding.
    #[must_use]
    pub fn new(name: impl Into<String>, mode: CaptureMode, span: TextRange) -> Self {
        Self {
            name: name.into(),
            mode,
            span,
        }
    }

    /// Returns the captured variable name including `$`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the explicit capture mode.
    #[must_use]
    pub const fn mode(&self) -> CaptureMode {
        self.mode
    }

    /// Returns the source span for the variable token.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Variable declaration recorded from a `global` statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariableBinding {
    name: String,
    span: TextRange,
}

impl VariableBinding {
    /// Creates a variable binding.
    #[must_use]
    pub fn new(name: impl Into<String>, span: TextRange) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    /// Returns the variable name including `$`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the source span for the variable token.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Static local declaration recorded from a `static` statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticLocalBinding {
    variable: VariableBinding,
}

impl StaticLocalBinding {
    /// Creates a static local binding.
    #[must_use]
    pub fn new(variable: VariableBinding) -> Self {
        Self { variable }
    }

    /// Returns the variable declaration.
    #[must_use]
    pub const fn variable(&self) -> &VariableBinding {
        &self.variable
    }
}
