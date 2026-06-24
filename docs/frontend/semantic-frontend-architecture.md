# Semantic Frontend Architecture

Semantic frontend is a read-only consumer of the existing source, token, and CST layers.
The pipeline is:

```text
SourceText
  -> php_lexer tokens
  -> php_syntax lossless CST
  -> php_ast typed CST views
  -> declaration collection
  -> namespace and import tables
  -> name resolution
  -> php_semantics HIR
  -> type lowering
  -> constant-expression validation
  -> attribute lowering
  -> semantic diagnostics
  -> FrontendResult
```

## Layer Responsibilities

`php_ast` provides typed views over CST nodes and tokens. Views must not own a
second syntax tree, re-tokenize source, or reinterpret byte spans. They expose
structured access to declarations, members, statements, expressions, type
syntax, attributes, names, and PHP 8.5 syntax forms.

`php_semantics` owns semantic IDs, HIR arenas, declaration tables, scopes, name
resolution output, type lowering, constant-expression metadata, attribute
metadata, compile-time diagnostics, and stable JSON/debug summaries.

`php_frontend_cli` exposes deterministic diagnostics and inspection commands.
It calls `php_semantics`; it must not implement an alternate parser or execute
PHP code.

`php_semantics::query::frontend` is the public query-shaped entry point for
hosts that need cache-friendly analysis. It exposes `FileId`, `SourceHash`,
`ParseCacheKey`, `FrontendOptions`, and stage-named functions while keeping the
current implementation vertical. See `docs/frontend/incremental-frontend-api.md`.

`php_frontend_cli symbols <file>` prints the Semantic frontend declaration table. The
table records stable `DeclId`/`SymbolId` pairs, declaration kind, FQN, source
span, and whether the declaration is conditional.

## Conditional Declarations

PHP permits declarations inside statement contexts such as `if` blocks. Semantic frontend
records those as conditional declarations because their existence depends on
runtime execution. Conditional declarations are included in symbol output but
are not treated as always-available globals and are excluded from the safe
single-file duplicate-declaration heuristic.

## Frontend Result Shape

The main public analysis result should include:

- parser diagnostics from `php_syntax`
- semantic diagnostics from `php_semantics`
- a `SemanticModule`
- symbol and declaration tables
- scopes
- import tables
- HIR arenas
- type and constant-expression tables
- source maps from HIR IDs back to CST/source byte ranges
- target PHP version and analysis options

## Diagnostic Separation

Parser diagnostics remain syntax diagnostics. Semantic diagnostics use their
own stable IDs, severity, phase, labels, and byte spans. PHP reference message
text may inform tests, but exact wording is not a compatibility requirement for
Semantic frontend unless a fixture explicitly asserts it.

## Layer Boundary

HIR may model deferred runtime effects such as include, require, eval, dynamic
function lookup, and autoload-sensitive names. It must not execute those
effects. Runtime values, zvals, copy-on-write, object semantics, opcodes, VM
dispatch, and extension behavior are Runtime or later concerns.

PHP 8.5 syntax forms such as pipe, clone-with, first-class callables in
constant expressions, and reference-observed `(void)` rejection are documented
in `docs/frontend/php85-semantics.md`.
