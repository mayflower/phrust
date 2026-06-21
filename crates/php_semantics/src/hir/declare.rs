//! Declare directive metadata.

use php_source::TextRange;

/// Lowered `declare(...)` statement metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HirDeclare {
    directives: Vec<DeclareDirective>,
    span: TextRange,
}

impl HirDeclare {
    /// Creates a declare metadata record.
    #[must_use]
    pub fn new(directives: Vec<DeclareDirective>, span: TextRange) -> Self {
        Self { directives, span }
    }

    /// Returns directive records in source order.
    #[must_use]
    pub fn directives(&self) -> &[DeclareDirective] {
        &self.directives
    }

    /// Returns the source span of the declare statement.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }
}

/// One `declare(name=value)` entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclareDirective {
    name: String,
    value: DeclareValue,
    span: TextRange,
    value_span: Option<TextRange>,
}

impl DeclareDirective {
    /// Creates a directive entry.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        value: DeclareValue,
        span: TextRange,
        value_span: Option<TextRange>,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            span,
            value_span,
        }
    }

    /// Returns the directive name as written.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a lowercase key for directive comparisons.
    #[must_use]
    pub fn canonical_name(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    /// Returns the lowered literal directive value.
    #[must_use]
    pub const fn value(&self) -> &DeclareValue {
        &self.value
    }

    /// Returns the directive source span.
    #[must_use]
    pub const fn span(&self) -> TextRange {
        self.span
    }

    /// Returns the value source span, when a value token was visible.
    #[must_use]
    pub const fn value_span(&self) -> Option<TextRange> {
        self.value_span
    }
}

/// Conservative literal forms accepted in declare metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeclareValue {
    /// Integer literal value.
    Int(i64),
    /// String literal value with PHP quotes removed conservatively.
    String(String),
    /// Any unsupported or missing value token, preserved as source text.
    Unknown(String),
}

impl DeclareValue {
    /// Returns stable JSON text for the value family.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Int(_) => "int",
            Self::String(_) => "string",
            Self::Unknown(_) => "unknown",
        }
    }
}

/// File-level directive summary for downstream phases.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileDirectives {
    strict_types: Option<DeclareDirective>,
    encoding: Option<DeclareDirective>,
    ticks: Option<DeclareDirective>,
    unknown: Vec<DeclareDirective>,
}

impl FileDirectives {
    /// Records a directive into the file-level summary.
    pub fn record(&mut self, directive: DeclareDirective) {
        match directive.canonical_name().as_str() {
            "strict_types" => self.strict_types = Some(directive),
            "encoding" => self.encoding = Some(directive),
            "ticks" => self.ticks = Some(directive),
            _ => self.unknown.push(directive),
        }
    }

    /// Returns the last visible `strict_types` directive.
    #[must_use]
    pub const fn strict_types(&self) -> Option<&DeclareDirective> {
        self.strict_types.as_ref()
    }

    /// Returns the last visible `encoding` directive.
    #[must_use]
    pub const fn encoding(&self) -> Option<&DeclareDirective> {
        self.encoding.as_ref()
    }

    /// Returns the last visible `ticks` directive.
    #[must_use]
    pub const fn ticks(&self) -> Option<&DeclareDirective> {
        self.ticks.as_ref()
    }

    /// Returns unknown file-level directives.
    #[must_use]
    pub fn unknown(&self) -> &[DeclareDirective] {
        &self.unknown
    }
}
