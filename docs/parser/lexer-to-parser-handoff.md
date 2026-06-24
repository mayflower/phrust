# Lexer to Parser Handoff

Parser should consume `php_lexer::lex_all` and the public token types exported
from `php_lexer`.

## Token API

- `Token.kind`: normalized `TokenKind`, either PHP `T_*`, symbol, or synthetic
  EOF when requested.
- `Token.range`: byte-oriented half-open source range.
- `Token.line`: one-based start line from the lexer.
- `Token::text(source)`: original token text slice.
- `LexResult.diagnostics`: recoverable lexer diagnostics with stable IDs.

## Parser-Relevant Modes

The parser should be aware that the lexer has explicit modes for inline HTML,
scripting, double-quoted strings, backticks, heredoc, nowdoc, and interpolation
substates. Parser work should not reinterpret scanner state from raw source
unless a lexer gap is documented.

## TOKEN_PARSE and Context

Strict `TOKEN_PARSE` parity is deferred to Parser because contextual keyword
relaxation depends on grammar context. Parser must decide how to represent:

- parser-contextual keyword relaxation;
- names that become reserved only in specific grammar positions;
- class constant and property lookups inside interpolated strings;
- grammar recovery after lexer diagnostics.

## Reusable Fixtures

Reuse `tests/fixtures/lexer/*.php` for parser smoke tests where tokens are the
input boundary. The strict command `just lexer-diff` currently passes for the
curated Lexer fixtures.

## Gaps to Close Before Parser Reliance

- Audit contextual token normalization beyond the curated Lexer fixtures.
- Decide on byte-exact non-UTF-8 source handling.
- Keep generated large reports and extracted php-src corpus files under
  `target/`, not in the repository.
