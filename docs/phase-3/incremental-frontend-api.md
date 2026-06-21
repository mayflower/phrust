# Incremental Frontend API Preparation

Phase 3 does not implement a query engine. It exposes query-shaped APIs so a
future LSP, Salsa integration, or host cache can add incrementality without
changing the main frontend contract.

The public boundary is `php_semantics::query::frontend`.

## Pipeline Functions

- `parse_file(source, file_id)` parses a single source string and carries a
  caller-owned `FileId` through parser context metadata.
- `ast_root(parse)` returns the typed `SourceFile` AST view.
- `collect_declarations(root, database, module_id)` runs the current fused
  Phase 3 declaration, name-resolution, HIR-lowering, and check pass.
- `resolve_names()`, `lower_hir()`, and `run_checks()` are explicit future split
  points. They are currently no-op boundaries because the implementation is
  still intentionally vertical.
- `analyze_file(source, options)` is the stable high-level entry point.

## Cache-Friendly Inputs

- `FileId` is caller-owned and can be a path, URI, or host document ID.
- `SourceHash` is a deterministic FNV-1a hash over source bytes.
- `ParseCacheKey` combines `FileId`, `SourceHash`, and target PHP version.
- `FrontendOptions` carries:
  - `target_php_version`
  - `enable_reference_annotations`
  - `max_diagnostics`
  - `strict_known_gap_mode`

No global mutable singleton database is used. Every `analyze_file` call creates
and returns its own `FrontendDatabase`.

## Stable Cache Candidates

These Phase 3 data sets are stable for one file and can be cached by
`ParseCacheKey`:

- parse result and parser diagnostics
- typed AST root views
- source maps and byte spans
- file-local HIR arenas
- file-local declarations
- file-local scopes
- file-local semantic diagnostics that do not require autoload or include
  execution

## Later Invalidations

Phase 4 and later phases must invalidate or recompute data that depends on
cross-file or runtime-sensitive behavior:

- autoload-sensitive class/interface/trait/enum lookup
- include, require, and eval effects
- function and constant fallback that depends on loaded files
- inheritance checks that require resolved parent members
- bytecode-lowering decisions that depend on runtime values or zval behavior

## Phase 4 Consumption

Bytecode lowering should consume HIR through `FrontendResult::database()` and
the module ID from `FrontendResult::module()`. It should treat include/eval,
autoload, callable validation, object construction, clone-with property
semantics, and exact runtime error behavior as unresolved inputs rather than
assuming Phase 3 executed them.
