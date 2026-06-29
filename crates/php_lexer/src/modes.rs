/// Scanner modes required for PHP tokenization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LexerMode {
    /// Bytes outside a PHP opening tag.
    #[default]
    InlineHtml,
    /// Normal PHP script tokenization.
    Scripting,
    /// Interpolated double-quoted string body.
    DoubleQuote,
    /// Interpolated shell execution string body.
    Backtick,
    /// Heredoc body.
    Heredoc,
    /// Nowdoc body.
    Nowdoc,
    /// Variable offset inside an interpolated string.
    StringVarOffset,
    /// Scanner is resolving a variable name in interpolation.
    LookingForVarName,
}

impl LexerMode {
    /// Returns the stable scanner mode name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InlineHtml => "inline_html",
            Self::Scripting => "scripting",
            Self::DoubleQuote => "double_quote",
            Self::Backtick => "backtick",
            Self::Heredoc => "heredoc",
            Self::Nowdoc => "nowdoc",
            Self::StringVarOffset => "string_var_offset",
            Self::LookingForVarName => "looking_for_var_name",
        }
    }
}
