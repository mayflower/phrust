use php_diagnostics::{
    DiagnosticEnvelope, DiagnosticLayer, DiagnosticLocation, DiagnosticPhase,
    DiagnosticSeverity as EnvelopeSeverity, DiagnosticSpan, DiagnosticSuggestion,
};
use php_source::TextRange;
use std::collections::BTreeMap;

/// Stable parser diagnostic identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseDiagnosticId {
    /// Diagnostic forwarded from the lexer during stub parsing.
    LexerDiagnostic,
    /// Parser encountered an unexpected token.
    UnexpectedToken,
    /// Parser expected a specific token.
    ExpectedToken,
    /// Parser expected an expression.
    ExpectedExpression,
    /// Parser expected a statement.
    ExpectedStatement,
    /// Parser expected a type.
    ExpectedType,
    /// Parser expected an identifier or name.
    ExpectedIdentifier,
    /// Parser reached a recovery point before a delimiter closed.
    UnclosedDelimiter,
}

impl ParseDiagnosticId {
    /// Returns the stable external diagnostic identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LexerDiagnostic => "lexer_diagnostic",
            Self::UnexpectedToken => "unexpected_token",
            Self::ExpectedToken => "expected_token",
            Self::ExpectedExpression => "expected_expression",
            Self::ExpectedStatement => "expected_statement",
            Self::ExpectedType => "expected_type",
            Self::ExpectedIdentifier => "expected_identifier",
            Self::UnclosedDelimiter => "unclosed_delimiter",
        }
    }

    /// Returns the stable machine-readable PHP parser diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::LexerDiagnostic => "E_PHP_PARSE_LEXER_DIAGNOSTIC",
            Self::UnexpectedToken => "E_PHP_PARSE_UNEXPECTED_TOKEN",
            Self::ExpectedToken => "E_PHP_PARSE_EXPECTED_TOKEN",
            Self::ExpectedExpression => "E_PHP_PARSE_EXPECTED_EXPRESSION",
            Self::ExpectedStatement => "E_PHP_PARSE_EXPECTED_STATEMENT",
            Self::ExpectedType => "E_PHP_PARSE_EXPECTED_TYPE",
            Self::ExpectedIdentifier => "E_PHP_PARSE_EXPECTED_IDENTIFIER",
            Self::UnclosedDelimiter => "E_PHP_PARSE_UNCLOSED_DELIMITER",
        }
    }

    /// Returns a concrete suggestion when the parser knows an actionable shape.
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
            Self::LexerDiagnostic => None,
            Self::UnexpectedToken => Some("remove the unexpected token or add the expected syntax"),
            Self::ExpectedToken => Some("insert the expected token"),
            Self::ExpectedExpression => Some("insert a valid expression"),
            Self::ExpectedStatement => Some("insert a valid statement"),
            Self::ExpectedType => Some("insert a valid type"),
            Self::ExpectedIdentifier => Some("insert a valid identifier or name"),
            Self::UnclosedDelimiter => Some("add the matching closing delimiter"),
        }
    }
}

/// Parser diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseSeverity {
    /// Syntax or tokenization error.
    Error,
}

impl ParseSeverity {
    /// Returns the shared diagnostic envelope severity.
    #[must_use]
    pub const fn envelope_severity(self) -> EnvelopeSeverity {
        match self {
            Self::Error => EnvelopeSeverity::Error,
        }
    }
}

/// A recoverable parse diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseDiagnostic {
    /// Stable diagnostic identifier.
    pub id: ParseDiagnosticId,
    /// Human-readable message.
    pub message: String,
    /// Byte span in the original source.
    pub span: TextRange,
    /// Expected syntax names, empty for forwarded lexer diagnostics.
    pub expected: Vec<String>,
    /// Diagnostic severity.
    pub severity: ParseSeverity,
}

impl ParseDiagnostic {
    /// Creates a parse diagnostic.
    #[must_use]
    pub fn new(id: ParseDiagnosticId, message: impl Into<String>, span: TextRange) -> Self {
        Self {
            id,
            message: message.into(),
            span,
            expected: Vec::new(),
            severity: ParseSeverity::Error,
        }
    }

    /// Adds expected syntax names to the diagnostic.
    #[must_use]
    pub fn with_expected(mut self, expected: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.expected = expected.into_iter().map(Into::into).collect();
        self
    }

    /// Returns a structured diagnostic envelope for this parser diagnostic.
    #[must_use]
    pub fn to_diagnostic_envelope(
        &self,
        source: Option<&php_source::SourceText>,
        source_id: Option<&str>,
        path: Option<&str>,
    ) -> DiagnosticEnvelope {
        let mut context = BTreeMap::new();
        if !self.expected.is_empty() {
            context.insert("expected".to_owned(), self.expected.join(","));
        }

        let mut envelope = DiagnosticEnvelope::new(
            self.id.code(),
            DiagnosticLayer::parser(),
            DiagnosticPhase::new("parse"),
            self.severity.envelope_severity(),
            self.message.clone(),
        )
        .with_context(context);

        envelope.legacy_id = Some(self.id.as_str().to_owned());
        envelope.php_visible = true;
        envelope.location = Some(match source {
            Some(source) => DiagnosticLocation::from_source_range(
                path.map(str::to_owned),
                source_id.map(str::to_owned),
                source,
                self.span,
            ),
            None => DiagnosticLocation::new(
                path.map(str::to_owned),
                source_id.map(str::to_owned),
                Some(DiagnosticSpan::from_range(self.span)),
            ),
        });
        envelope.suggestion = self.id.suggestion().map(DiagnosticSuggestion::new);
        envelope
    }
}
