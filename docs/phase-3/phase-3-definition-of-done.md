# Phase 3 Definition of Done

Phase 3 builds a PHP 8.5 semantic frontend on top of the Phase 1 lexer and
Phase 2 lossless CST. It introduces typed AST views, semantic HIR, declaration
tables, scopes, name resolution, type lowering, constant-expression validation,
attribute metadata, and compile-time diagnostics.

The target remains PHP 8.5.7, git tag `php-8.5.7`.

## Goals

- Provide `php_ast` typed views over the existing CST without reparsing.
- Provide `php_semantics` with HIR, symbol IDs, declaration tables, scope
  arenas, name resolution results, type models, constant-expression metadata,
  attribute metadata, and semantic diagnostics.
- Provide `php_frontend_cli` for deterministic analysis, diagnostics, symbol,
  scope, HIR, and snapshot output.
- Keep parser diagnostics and semantic diagnostics separate.
- Preserve byte spans as the source of truth; line and column values remain
  derived display data.
- Compare semantic acceptance with the pinned PHP reference by diagnostics,
  token text, source spans, and structured frontend output, not numeric token
  IDs.
- Track every reference mismatch as an explicit known gap or a failing diff.
- Expose enough HIR and metadata for a later bytecode or IR lowering phase.

## Non-Goals

- No new lexer or parser.
- No semantic rules inside `php_syntax`.
- No bytecode, VM, runtime values, Zend ABI emulation, JIT, extensions, or
  standard-library implementation.
- No include, require, eval, or autoload execution.
- No attribute class instantiation.
- No cross-file runtime symbol loading.
- No vendored `php-src` checkout or generated reports under `target/`.

## Required Artifacts

- `crates/php_ast`
- `crates/php_semantics`
- `crates/php_frontend_cli`
- Semantic fixture categories under `fixtures/semantic`
- Semantic reference and diff harnesses
- Stable frontend snapshots
- Phase 3 documentation and ADRs
- Query-shaped frontend API documentation
- PHP 8.5 semantic focus documentation
- `nix develop -c just verify-phase3`

## Verification Contract

The Phase 3 gate must keep existing gates available and must not weaken Phase
0, Phase 1, or Phase 2 verification. Reference-dependent checks must skip
clearly when no PHP reference binary is available, and must be strict when
`REFERENCE_PHP` is explicitly set.

Hard gates:

```bash
nix develop -c just verify-phase0
nix develop -c just verify-phase1
nix develop -c just verify-phase2
nix develop -c just verify-phase3
nix develop -c cargo fmt --all --check
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo test --workspace
```

Reference and snapshot gates:

```bash
nix develop -c just semantic-fixtures
nix develop -c just semantic-diff
nix develop -c just frontend-snapshots
```

Optional heavy gates:

```bash
nix develop -c just semantic-corpus-smoke
nix develop -c just fuzz-frontend-smoke
nix develop -c just bench-frontend
```

## Completion Criteria

Phase 3 is complete when the semantic frontend can analyze curated PHP 8.5
fixtures deterministically, explain accepted and rejected files through stable
diagnostics and HIR output, compare acceptance with the pinned PHP reference,
and hand off structured HIR plus metadata to Phase 4 without relying on parser
internals or runtime execution.
