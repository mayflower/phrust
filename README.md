# phrust

`phrust` is a Rust project intended to become a PHP 8.5 compatible core
engine.

The repository currently contains the foundation tooling, byte-oriented lexer,
lossless parser/CST layer, typed AST views, Phase 3 semantic frontend crates,
Phase 4 IR/runtime/VM crates, fixture harnesses, and PHP reference comparison
scripts. It does not implement JIT, extensions, or Zend ABI emulation.

## Reference Target

- PHP series: `8.5`
- PHP version: `8.5.7`
- Git tag: `php-8.5.7`
- Repository: `https://github.com/php/php-src.git`

The reference version is fixed by ADR. It must not be automatically advanced to
a newer patch release without a new ADR.

## Development Environment

### Quickstart

1. Install Nix with Flake support.

2. Enter the development shell:

```bash
nix develop
```

3. List available commands:

```bash
just help
```

4. Run the central parser/CST verification gate:

```bash
just verify-phase2
```

5. Fetch and pin the PHP reference:

```bash
just bootstrap-ref
```

6. Extract reference metadata:

```bash
just extract-ref-metadata
```

7. Optionally build the reference PHP CLI:

```bash
just build-ref-php
```

The same validation command can also be run without entering the shell first:

```bash
nix develop -c just verify-phase2
```

Useful parser commands:

```bash
just parser-diff
cargo run -p php_parser_cli -- --debug-tree file.php
```

Phase 3 semantic frontend work is validated by the documented contract:

```bash
nix develop -c just verify-phase0
nix develop -c just verify-phase1
nix develop -c just verify-phase2
nix develop -c just verify-phase3
```

The Phase 3 gate checks typed AST views, semantic HIR, diagnostics, semantic
fixtures, reference acceptance diffing, and frontend CLI snapshots.

Phase 4 starts the executable core: HIR to IR/bytecode, interpreter VM,
minimal runtime values, CLI execution, and differential runtime tests against
the pinned PHP 8.5.7 reference. It must use the existing Phase 3 frontend as
input and must not add a second syntax or semantic pipeline.

Phase 4 contracts:

- [Runtime contract](docs/phase4-runtime-contract.md)
- [Runtime values](docs/phase4-runtime-values.md)
- [VM model](docs/phase4-vm.md)
- [Supported subset](docs/phase4-supported-subset.md)
- [Known gaps](docs/phase4-known-gaps.md)
- [Runtime reference oracle](docs/phase4-runtime-reference.md)
- [Reference diff](docs/phase4-reference-diff.md)
- [IR model](docs/phase4-ir.md)

Phase 4 commands:

```bash
nix develop -c just verify-phase4
nix develop -c just runtime-fixtures
nix develop -c just runtime-diff
```

## CI

CI uses Nix. The parser and semantic frontend workflows run the same commands
used locally:

```bash
nix develop -c just verify-phase2
nix develop -c just verify-phase3
```

Required CI does not clone or build `php-src`. Reference-dependent fixture
checks skip clearly if a PHP reference binary is unavailable, and fail when an
explicit `REFERENCE_PHP` is configured but unusable.

Phase 3 hard gates are formatting, Clippy, workspace tests, semantic fixtures,
reference acceptance diffing, and frontend snapshots through:

```bash
nix develop -c just verify-phase3
```

Long-running or currently undefined work is kept behind explicit soft targets
and is not run by required CI:

```bash
nix develop -c just semantic-corpus-smoke
nix develop -c just fuzz-frontend-smoke
nix develop -c just bench-frontend
```

## Foundation Scope

The foundation establishes:

- A pinned PHP 8.5.7 reference contract.
- Documentation for the authoritative PHP syntax and runtime sources.
- A Rust workspace foundation with shared source/testkit crates:
  - `crates/php_source`
  - `crates/php_testkit`
- Scripts to fetch and verify the PHP reference.
- A test-oracle plan for lexer, parser, runtime, and framework compatibility.
- CI preparation around `nix develop -c just verify-phase0`.

The foundation does not build the engine.

It also does not implement VM, runtime value model, JIT, extensions, or Zend
ABI emulation.

## Lexer Scope

The lexer/tokenization layer targets curated fixture compatibility with:

```php
token_get_all($code, 0)
```

The central lexer validation command is:

```bash
nix develop -c just verify-phase1
```

The lexer layer does not implement parser semantics, AST/CST lowering, VM,
runtime, JIT, extensions, or Zend ABI emulation.

Useful lexer commands:

```bash
export REFERENCE_PHP="$PWD/third_party/php-src/sapi/cli/php"
nix develop -c just lex tests/fixtures/lexer/010-tags.php
nix develop -c just lexer-fixtures
nix develop -c just lexer-diff
nix develop -c just lexer-diff-report
nix develop -c just fuzz-lexer-smoke
nix develop -c just bench-lexer
nix develop -c just lexer-corpus-smoke
```

`just lexer-fixtures` and `just lexer-diff` both run strict comparison for the
curated fixtures. `docs/phase-1/known-lexer-differences.md` records that no
curated fixture differences are currently accepted.

## Parser and CST Scope

The parser consumes `php_lexer` tokens and builds a lossless CST. It preserves
PHP tags, inline HTML, trivia, strings, heredoc/nowdoc structures, byte spans,
diagnostics, and error nodes. It compares curated fixture acceptance with the
pinned PHP 8.5.7 `php -l` oracle.

The central parser/CST validation command is:

```bash
nix develop -c just verify-phase2
```

Useful parser commands:

```bash
nix develop -c just parser-fixtures
nix develop -c just parser-diff
nix develop -c just cst-roundtrip
nix develop -c cargo run -p php_parser_cli -- --debug-tree file.php
```

Parser/CST work does not perform name resolution, compile-time semantic checks,
typed AST/HIR lowering, bytecode/IR generation, execution, runtime values, JIT,
extensions, or Zend ABI emulation.

## Phase 3 Semantic Frontend Scope

Phase 3 will consume the existing CST and build:

- `php_ast`: typed, read-only AST views over CST nodes and tokens.
- `php_semantics`: HIR, declaration tables, symbol IDs, scopes, name
  resolution, type lowering, constant-expression metadata, attribute metadata,
  and compile-time diagnostics.
- `php_frontend_cli`: deterministic analysis, diagnostics, symbols, scopes,
  HIR, and snapshot output.

Semantic diagnostics remain separate from parser diagnostics. Phase 3 does not
introduce a new lexer or parser, does not execute PHP files, does not run
include/require/eval, does not instantiate attributes, and does not implement
runtime values, opcodes, VM dispatch, JIT, extensions, or Zend ABI emulation.

Current Phase 3 smoke commands:

```bash
nix develop -c cargo run -p php_frontend_cli -- --help
nix develop -c cargo run -p php_frontend_cli -- analyze fixtures/parser/valid/basic_echo.php --format json
nix develop -c just semantic-fixtures
nix develop -c just semantic-diff
nix develop -c just frontend-snapshots
```

## Rust Workspace

The workspace uses Cargo resolver `3` and Rust edition `2024`. The current
crates are:

- `php_source`: byte-oriented source maps and spans.
- `php_lexer`: PHP lexer/tokenization library.
- `php_lexer_cli`: JSON output CLI for differential testing.
- `php_syntax`: PHP parser and lossless CST library.
- `php_parser_cli`: parser diagnostics, JSON, debug tree, and roundtrip CLI.
- `php_ast`: typed AST views over the lossless CST.
- `php_semantics`: semantic frontend API and HIR/diagnostic home.
- `php_frontend_cli`: semantic frontend inspection CLI.
- `php_testkit`: reference testing helpers.
- `php_ir`: Phase 4 IR/bytecode boundary.
- `php_runtime`: Phase 4 runtime value and diagnostic boundary.
- `php_vm`: Phase 4 interpreter VM boundary.
- `php_vm_cli`: Phase 4 VM command-line boundary.

## Reference Source Policy

The `php-src` checkout is local only and will live under `third_party/php-src`.
It must not be committed. Reference metadata and lockfiles belong under
`references/`.

## Documentation

- [Phase 0 Definition of Done](docs/phase-0/phase-0-definition-of-done.md)
- [Phase 0 Risk Register](docs/phase-0/risk-register.md)
- [ADR 0001: Target PHP Version](docs/adr/0001-target-php-version.md)
- [ADR 0002: Nix Development Environment](docs/adr/0002-nix-dev-environment.md)
- [ADR 0003: Reference Oracle](docs/adr/0003-reference-oracle.md)
- [ADR 0004: No Vendored php-src](docs/adr/0004-no-vendored-php-src.md)
- [ADR 0005: Phase Boundaries](docs/adr/0005-phase-boundaries.md)
- [ADR 0006: Byte-Oriented Lossless Lexer](docs/adr/0006-byte-oriented-lossless-lexer.md)
- [ADR 0007: Token Oracle Normalization](docs/adr/0007-token-oracle-normalization.md)
- [ADR 0008: Lexer Parser Boundary](docs/adr/0008-lexer-parser-boundary.md)
- [Phase 1 Definition of Done](docs/phase-1/phase-1-definition-of-done.md)
- [Phase 1 Final Audit](docs/phase-1/final-audit.md)
- [Known Lexer Differences](docs/phase-1/known-lexer-differences.md)
- [Lexer to Parser Handoff](docs/phase-2/lexer-to-parser-handoff.md)
- [Parser and CST Definition of Done](docs/phase-2/phase-2-definition-of-done.md)
- [Parser Architecture](docs/phase-2/parser-architecture.md)
- [CST Model](docs/phase-2/cst-model.md)
- [PHP Lint Oracle](docs/phase-2/php-lint-oracle.md)
- [Parser Known Gaps](docs/phase-2/parser-known-gaps.md)
- [Handoff to Semantic Layers](docs/phase-2/handoff-to-phase-3.md)
- [Phase 3 Definition of Done](docs/phase-3/phase-3-definition-of-done.md)
- [Semantic Frontend Architecture](docs/phase-3/semantic-frontend-architecture.md)
- [AST View Model](docs/phase-3/ast-view-model.md)
- [HIR Model](docs/phase-3/hir-model.md)
- [Declaration Model](docs/phase-3/declaration-model.md)
- [Symbol Table Model](docs/phase-3/symbol-table-model.md)
- [PHP Name Resolution](docs/phase-3/php-name-resolution.md)
- [Scope Model](docs/phase-3/scope-model.md)
- [Type Lowering](docs/phase-3/type-lowering.md)
- [Constant Expressions](docs/phase-3/constant-expressions.md)
- [Attributes](docs/phase-3/attributes.md)
- [Compile-Time Diagnostics](docs/phase-3/compile-time-diagnostics.md)
- [Semantic Reference Oracle](docs/phase-3/semantic-reference-oracle.md)
- [Semantic Fixtures](docs/phase-3/semantic-fixtures.md)
- [Semantic Known Gaps](docs/phase-3/semantic-known-gaps.md)
- [Phase 4 Handoff](docs/phase-3/handoff-to-phase-4.md)
- [Phase 4 Runtime Contract](docs/phase4-runtime-contract.md)
- [Phase 4 Runtime Values](docs/phase4-runtime-values.md)
- [Phase 4 VM](docs/phase4-vm.md)
- [Phase 4 Runtime Reference Oracle](docs/phase4-runtime-reference.md)
- [Phase 4 Reference Diff](docs/phase4-reference-diff.md)
- [Phase 4 Supported Subset](docs/phase4-supported-subset.md)
- [Phase 4 Known Gaps](docs/phase4-known-gaps.md)
- [ADR 0011: Typed AST Views over CST](docs/adr/0011-typed-ast-views.md)
- [ADR 0012: HIR, Symbol IDs, and Interning](docs/adr/0012-hir-symbol-ids-and-interning.md)
- [ADR 0013: PHP Name Resolution](docs/adr/0013-php-name-resolution.md)
- [ADR 0014: Compile-Time Diagnostics](docs/adr/0014-compile-time-diagnostics.md)
- [ADR 0015: Constant Expression Lowering](docs/adr/0015-constant-expression-lowering.md)
- [ADR 0016: Phase 3 Runtime Boundary](docs/adr/0016-phase3-no-runtime-boundary.md)
