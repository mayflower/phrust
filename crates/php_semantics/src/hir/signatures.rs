//! Function-like signature HIR records.

use crate::hir::{StmtId, TypeId};
use php_source::TextRange;

/// Function-like signature lowered from PHP syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignature {
    kind: SignatureKind,
    name: Option<String>,
    parameters: Vec<Parameter>,
    return_type: Option<ReturnType>,
    by_ref_return: bool,
    flags: FunctionLikeFlags,
    arrow_body: Option<TextRange>,
    body: Vec<StmtId>,
    span: TextRange,
}

impl FunctionSignature {
    /// Creates a function-like signature.
    #[must_use]
    pub fn new(
        kind: SignatureKind,
        name: Option<String>,
        parameters: Vec<Parameter>,
        return_type: Option<ReturnType>,
        by_ref_return: bool,
        span: TextRange,
    ) -> Self {
        Self {
            kind,
            name,
            parameters,
            return_type,
            by_ref_return,
            flags: FunctionLikeFlags::new(by_ref_return),
            arrow_body: None,
            body: Vec::new(),
            span,
        }
    }

    /// Replaces inferred function-like flags.
    #[must_use]
    pub fn with_flags(mut self, flags: FunctionLikeFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Attaches the arrow-function expression body span.
    #[must_use]
    pub fn with_arrow_body(mut self, span: Option<TextRange>) -> Self {
        self.arrow_body = span;
        self
    }

    /// Attaches statement IDs for a named function body.
    #[must_use]
    pub fn with_body(mut self, body: Vec<StmtId>) -> Self {
        self.body = body;
        self
    }

    /// Returns the function-like kind.
    #[must_use]
    pub const fn kind(&self) -> SignatureKind {
        self.kind
    }

    /// Returns the optional source name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns parameters in source order.
    #[must_use]
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Returns the optional return type.
    #[must_use]
    pub const fn return_type(&self) -> Option<&ReturnType> {
        self.return_type.as_ref()
    }

    /// Returns true for `function &name()` or `function & (...)`.
    #[must_use]
    pub const fn by_ref_return(&self) -> bool {
        self.by_ref_return
    }

    /// Returns compile-time function-like flags.
    #[must_use]
    pub const fn flags(&self) -> &FunctionLikeFlags {
        &self.flags
    }

    /// Returns the arrow-function body expression span, when this signature is
    /// an arrow function and the parser exposed the body expression.
    #[must_use]
    pub const fn arrow_body(&self) -> Option<TextRange> {
        self.arrow_body
    }

    /// Returns lowered statement IDs for a named function body.
    #[must_use]
    pub fn body(&self) -> &[StmtId] {
        &self.body
    }

    /// Returns the source span for the signature owner.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Compile-time function-like metadata.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FunctionLikeFlags {
    returns_by_ref: bool,
    is_static: bool,
    is_generator: bool,
    has_return_type_void: bool,
    has_return_type_never: bool,
    has_tentative_or_deferred_info: bool,
    this_available: bool,
}

impl FunctionLikeFlags {
    /// Creates function-like flags with the by-reference return bit.
    #[must_use]
    pub const fn new(returns_by_ref: bool) -> Self {
        Self {
            returns_by_ref,
            is_static: false,
            is_generator: false,
            has_return_type_void: false,
            has_return_type_never: false,
            has_tentative_or_deferred_info: false,
            this_available: false,
        }
    }

    /// Marks whether the function-like declaration is static.
    pub const fn set_static(&mut self, is_static: bool) {
        self.is_static = is_static;
    }

    /// Marks whether the function-like contains `yield` or `yield from`.
    pub const fn set_generator(&mut self, is_generator: bool) {
        self.is_generator = is_generator;
    }

    /// Marks whether the return type is `void`.
    pub const fn set_return_type_void(&mut self, has_return_type_void: bool) {
        self.has_return_type_void = has_return_type_void;
    }

    /// Marks whether the return type is `never`.
    pub const fn set_return_type_never(&mut self, has_return_type_never: bool) {
        self.has_return_type_never = has_return_type_never;
    }

    /// Marks whether later phases must fill or validate deferred metadata.
    pub const fn set_tentative_or_deferred_info(&mut self, has_deferred_info: bool) {
        self.has_tentative_or_deferred_info = has_deferred_info;
    }

    /// Marks whether `$this` can be available in this function-like context.
    pub const fn set_this_available(&mut self, this_available: bool) {
        self.this_available = this_available;
    }

    /// Returns true for by-reference returns.
    #[must_use]
    pub const fn returns_by_ref(self) -> bool {
        self.returns_by_ref
    }

    /// Returns true for static methods, static closures, and static arrows.
    #[must_use]
    pub const fn is_static(self) -> bool {
        self.is_static
    }

    /// Returns true when `yield` or `yield from` appears in this function-like.
    #[must_use]
    pub const fn is_generator(self) -> bool {
        self.is_generator
    }

    /// Returns true when the declared return type is `void`.
    #[must_use]
    pub const fn has_return_type_void(self) -> bool {
        self.has_return_type_void
    }

    /// Returns true when the declared return type is `never`.
    #[must_use]
    pub const fn has_return_type_never(self) -> bool {
        self.has_return_type_never
    }

    /// Returns true when a later phase owns a precise check.
    #[must_use]
    pub const fn has_tentative_or_deferred_info(self) -> bool {
        self.has_tentative_or_deferred_info
    }

    /// Returns true when `$this` may be available.
    #[must_use]
    pub const fn this_available(self) -> bool {
        self.this_available
    }
}

/// Function-like signature families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureKind {
    /// Named function declaration.
    Function,
    /// Class method declaration.
    Method,
    /// Closure expression.
    Closure,
    /// Arrow function expression.
    ArrowFunction,
}

impl SignatureKind {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Closure => "closure",
            Self::ArrowFunction => "arrow_function",
        }
    }
}

/// Function-like parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    name: String,
    type_id: Option<TypeId>,
    flags: ParameterFlags,
    default: Option<DefaultValueRef>,
    attributes: Vec<ParameterAttribute>,
    span: TextRange,
}

impl Parameter {
    /// Creates a parameter record.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        type_id: Option<TypeId>,
        flags: ParameterFlags,
        default: Option<DefaultValueRef>,
        attributes: Vec<ParameterAttribute>,
        span: TextRange,
    ) -> Self {
        Self {
            name: name.into(),
            type_id,
            flags,
            default,
            attributes,
            span,
        }
    }

    /// Returns the variable name including `$`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the lowered parameter type.
    #[must_use]
    pub const fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    /// Returns parameter flags.
    #[must_use]
    pub const fn flags(&self) -> &ParameterFlags {
        &self.flags
    }

    /// Returns the default-value reference.
    #[must_use]
    pub const fn default(&self) -> Option<&DefaultValueRef> {
        self.default.as_ref()
    }

    /// Returns source attribute spans attached to the parameter.
    #[must_use]
    pub fn attributes(&self) -> &[ParameterAttribute] {
        &self.attributes
    }

    /// Returns the parameter source span.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Parameter flags.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParameterFlags {
    by_ref: bool,
    variadic: bool,
    promoted_property: Option<PromotedPropertyInfo>,
}

impl ParameterFlags {
    /// Creates parameter flags.
    #[must_use]
    pub const fn new(
        by_ref: bool,
        variadic: bool,
        promoted_property: Option<PromotedPropertyInfo>,
    ) -> Self {
        Self {
            by_ref,
            variadic,
            promoted_property,
        }
    }

    /// Returns true when the parameter is passed by reference.
    #[must_use]
    pub const fn is_by_ref(&self) -> bool {
        self.by_ref
    }

    /// Returns true when the parameter is variadic.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns constructor-promotion metadata.
    #[must_use]
    pub const fn promoted_property(&self) -> Option<&PromotedPropertyInfo> {
        self.promoted_property.as_ref()
    }
}

/// Return type reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReturnType {
    type_id: TypeId,
    span: TextRange,
}

impl ReturnType {
    /// Creates a return type reference.
    #[must_use]
    pub const fn new(type_id: TypeId, span: TextRange) -> Self {
        Self { type_id, span }
    }

    /// Returns the lowered type ID.
    #[must_use]
    pub const fn type_id(self) -> TypeId {
        self.type_id
    }

    /// Returns the source span.
    #[must_use]
    pub const fn span(self) -> TextRange {
        self.span
    }
}

/// Default value reference. The expression is not evaluated in Phase 3.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DefaultValueRef {
    span: TextRange,
    const_expr_candidate: bool,
}

impl DefaultValueRef {
    /// Creates a default-value reference.
    #[must_use]
    pub const fn new(span: TextRange, const_expr_candidate: bool) -> Self {
        Self {
            span,
            const_expr_candidate,
        }
    }

    /// Returns source span.
    #[must_use]
    pub const fn span(self) -> TextRange {
        self.span
    }

    /// Returns true when the default is a constant-expression candidate.
    #[must_use]
    pub const fn is_const_expr_candidate(self) -> bool {
        self.const_expr_candidate
    }
}

/// Source attribute span attached to a parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterAttribute {
    span: TextRange,
}

impl ParameterAttribute {
    /// Creates a parameter attribute record.
    #[must_use]
    pub const fn new(span: TextRange) -> Self {
        Self { span }
    }

    /// Returns source span.
    #[must_use]
    pub const fn span(self) -> TextRange {
        self.span
    }
}

/// Constructor property-promotion metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotedPropertyInfo {
    visibility: Visibility,
    readonly: bool,
    set_visibility: Option<Visibility>,
    span: TextRange,
}

impl PromotedPropertyInfo {
    /// Creates promoted-property metadata.
    #[must_use]
    pub const fn new(
        visibility: Visibility,
        readonly: bool,
        set_visibility: Option<Visibility>,
        span: TextRange,
    ) -> Self {
        Self {
            visibility,
            readonly,
            set_visibility,
            span,
        }
    }

    /// Returns property visibility.
    #[must_use]
    pub const fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Returns true when `readonly` is present.
    #[must_use]
    pub const fn is_readonly(&self) -> bool {
        self.readonly
    }

    /// Returns asymmetric set visibility.
    #[must_use]
    pub const fn set_visibility(&self) -> Option<Visibility> {
        self.set_visibility
    }

    /// Returns source span for promotion modifiers.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// Visibility flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    /// `public`.
    Public,
    /// `protected`.
    Protected,
    /// `private`.
    Private,
}

impl Visibility {
    /// Returns stable JSON text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
        }
    }
}
