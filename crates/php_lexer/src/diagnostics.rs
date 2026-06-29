use crate::{LexerMode, SourceText, TextRange};
use php_diagnostics::{
    DiagnosticEnvelope, DiagnosticLayer, DiagnosticLocation, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticSpan, DiagnosticSuggestion,
};
use std::collections::BTreeMap;

/// Diagnostic categories emitted by the lexer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LexDiagnosticKind {
    /// Input was malformed for the active scanner mode.
    InvalidInput,
    /// A block comment reached EOF before `*/`.
    UnterminatedBlockComment,
    /// A quoted string reached EOF before its closing delimiter.
    UnterminatedString,
    /// A heredoc or nowdoc block reached EOF before its closing label.
    UnterminatedHeredoc,
    /// A bad control byte was emitted as `T_BAD_CHARACTER`.
    BadCharacter,
}

impl LexDiagnosticKind {
    /// Returns the stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "E_PHP_LEXER_INVALID_INPUT",
            Self::UnterminatedBlockComment => "E_PHP_LEXER_UNTERMINATED_BLOCK_COMMENT",
            Self::UnterminatedString => "E_PHP_LEXER_UNTERMINATED_STRING",
            Self::UnterminatedHeredoc => "E_PHP_LEXER_UNTERMINATED_HEREDOC",
            Self::BadCharacter => "E_PHP_LEXER_BAD_CHARACTER",
        }
    }

    /// Returns the previous in-crate diagnostic kind name for compatibility.
    #[must_use]
    pub const fn legacy_id(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::UnterminatedBlockComment => "unterminated_block_comment",
            Self::UnterminatedString => "unterminated_string",
            Self::UnterminatedHeredoc => "unterminated_heredoc",
            Self::BadCharacter => "bad_character",
        }
    }

    /// Returns a short actionable suggestion when one is known.
    #[must_use]
    pub const fn suggestion(self) -> Option<&'static str> {
        match self {
            Self::InvalidInput => Some("check the input bytes for the active scanner mode"),
            Self::UnterminatedBlockComment => Some("close the block comment with */"),
            Self::UnterminatedString => Some("close the string with the matching quote"),
            Self::UnterminatedHeredoc => Some("add the matching heredoc or nowdoc closing label"),
            Self::BadCharacter => Some("remove the control character or escape it in a string"),
        }
    }
}

/// A recoverable lexer diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexDiagnostic {
    /// Diagnostic category.
    pub kind: LexDiagnosticKind,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Byte span associated with the diagnostic.
    pub span: TextRange,
    /// One-based source line associated with the diagnostic.
    pub line: usize,
}

impl LexDiagnostic {
    /// Creates a diagnostic.
    #[must_use]
    pub fn new(
        kind: LexDiagnosticKind,
        message: impl Into<String>,
        span: TextRange,
        line: usize,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            line,
        }
    }

    /// Returns a structured diagnostic envelope for this lexer diagnostic.
    #[must_use]
    pub fn to_diagnostic_envelope(
        &self,
        source: Option<&SourceText>,
        path: Option<&str>,
        scanner_mode: Option<LexerMode>,
    ) -> DiagnosticEnvelope {
        let mut context = BTreeMap::new();
        context.insert("line".to_owned(), self.line.to_string());
        if let Some(scanner_mode) = scanner_mode {
            context.insert("scanner_mode".to_owned(), scanner_mode.as_str().to_owned());
        }

        let mut envelope = DiagnosticEnvelope::new(
            self.kind.code(),
            DiagnosticLayer::lexer(),
            DiagnosticPhase::new("scan"),
            DiagnosticSeverity::Error,
            self.message.clone(),
        )
        .with_context(context);

        envelope.legacy_id = Some(self.kind.legacy_id().to_owned());
        envelope.php_visible = true;
        envelope.location = Some(match source {
            Some(source) => DiagnosticLocation::from_source_range(
                path.map(str::to_owned),
                None::<String>,
                source,
                self.span,
            ),
            None => DiagnosticLocation::new(
                path.map(str::to_owned),
                None::<String>,
                Some(DiagnosticSpan::from_range(self.span)),
            ),
        });
        envelope.suggestion = self.kind.suggestion().map(DiagnosticSuggestion::new);
        envelope
    }
}
