# Semantic frontend Final Audit

Date: 2026-06-20

## Target State

- PHP series: 8.5
- PHP version: 8.5.7
- PHP git tag: `php-8.5.7`
- Reference checkout: `third_party/php-src`
- Reference commit observed by `verify-foundation`:
  `35eab8c08bc590758d05813b0ff7a3d8c3e67b79`
- Project base commit during audit: `0146f39`
- Worktree state: dirty, with Parser and Semantic frontend implementation files staged
  only by later user action if requested.

## Implemented Crates and APIs

### `php_ast`

Implemented typed CST views over the existing `php_syntax` parser output. The
crate exposes AST node/token wrappers, typed declarations, statements,
expressions, class-like views, attribute views, name/type helpers, source-local
AST pointers, and validation helpers. It does not introduce a second lexer or a
second parser.

### `php_semantics`

Implemented the Semantic frontend semantic frontend layer:

- semantic database and module model
- typed HIR IDs and arenas
- declaration, import, symbol, scope, type, attribute, and constant-expression
  metadata
- name-resolution records with deferred runtime fallback metadata
- semantic diagnostics with stable IDs, spans, notes, deduplication, and a
  diagnostic cap
- HIR lowering with missing-node recovery
- query-shaped frontend API in `php_semantics::query::frontend`
- compatibility entry point `analyze_source`

The high-level API is `php_semantics::query::frontend::analyze_file`, returning
`FrontendResult`.

### `php_frontend_cli`

Implemented the semantic frontend CLI as a consumer of `php_semantics`:

- `analyze`
- `diagnostics`
- `symbols`
- `scopes`
- `hir`
- `snapshot`

Supported options include `--format text|json`, `--php-version-target 8.5`,
`--show-spans`, `--show-source-map`, `--show-deferred`,
`--fail-on-diagnostics`, and `--pretty`.

Exit codes are:

- 0: success
- 1: I/O error
- 2: usage error
- 3: diagnostics with `--fail-on-diagnostics`

### `php_testkit`

Extended with semantic fixture discovery, semantic reference result parsing,
and semantic acceptance diff status handling. The semantic diff tool compares
PHP lint acceptance against the Rust semantic frontend and treats only explicit
known-gap fixtures as allowed mismatches.

## Definition of Done

Met:

- Semantic frontend has a dedicated `php_ast` layer instead of adding typed views to
  `php_syntax`.
- Semantic frontend has a dedicated `php_semantics` layer for HIR, declarations, scopes,
  names, types, constant-expression checks, attributes, semantic diagnostics,
  query API, and deferred runtime metadata.
- The CLI consumes `php_semantics`; it does not parse independently.
- Include, require, eval, runtime fallback, and autoload-sensitive behavior are
  represented as deferred metadata or known gaps, not executed.
- Semantic diagnostic IDs are documented in
  `docs/frontend/compile-time-diagnostics.md`.
- Foundation, Lexer, and Parser verification commands remain available and pass.
- `nix develop -c just verify-frontend` is the local and CI Semantic frontend hard gate.
- Semantic frontend snapshots and semantic reference diff are deterministic over the
  curated fixture set.

Not met or intentionally deferred:

- Full cross-file symbol linking and autoload-aware resolution are deferred to
  Runtime or later.
- Full CFG-level `goto` boundary validation is a known gap.
- Exact PHP error message wording compatibility is incomplete; Semantic frontend compares
  acceptance, IDs, spans, and structured diagnostics.
- Include/require/eval execution effects are not modeled.
- Runtime value behavior, references, copy-on-write, VM, bytecode, JIT,
  extensions, and Zend ABI emulation are outside Semantic frontend scope.
- Optional `semantic-corpus-smoke`, `fuzz-frontend-smoke`, and
  `bench-frontend` targets exist as explicit soft gates but are not configured
  to run heavy jobs by default.

## Test Status

All requested final-audit commands were executed through `nix develop` and
passed:

```bash
nix develop -c just verify-foundation
nix develop -c just verify-lexer
nix develop -c just verify-frontend
nix develop -c just verify-frontend
nix develop -c cargo fmt --all --check
nix develop -c cargo clippy --workspace --all-targets -- -D warnings
nix develop -c cargo test --workspace
nix develop -c just semantic-fixtures
nix develop -c just semantic-diff
nix develop -c just frontend-snapshots
```

Local tests:

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass
- `just verify-foundation`: pass
- `just verify-lexer`: pass
- `just verify-frontend`: pass
- `just verify-frontend`: pass

Reference tests:

- PHP reference binary: `third_party/php-src/sapi/cli/php`
- Reference version: 8.5.7
- Parser acceptance diff: 65 fixtures, 0 allowed gaps
- Semantic acceptance diff: 149 fixtures, 147 matches, 0 unexpected
  mismatches, 2 known gaps, 0 skips

Snapshot tests:

- Semantic snapshots: 149 generated snapshots under
  `fixtures/semantic/snapshots`
- Frontend CLI snapshot smoke: pass
- Parser CST/diagnostic snapshots remain covered by Parser gates

Known gaps:

- `fixtures/semantic/control_flow/goto-invalid-known-gap.php`: full PHP `goto`
  boundary validation deferred until a CFG pass.
- `fixtures/semantic/declarations/duplicate-class-invalid.php`: Rust performs a
  deterministic same-file duplicate class declaration check that PHP lint
  defers until compile/runtime loading.

## Diagnostic ID Inventory

- `PHS0000`
- `E_PHP_DUPLICATE_PARAMETER`
- `E_PHP_VARIADIC_PARAMETER_NOT_LAST`
- `E_PHP_INVALID_PARAMETER_DEFAULT`
- `E_PHP_INVALID_PROPERTY_PROMOTION`
- `E_PHP_CLOSURE_USE_DUPLICATES_PARAMETER`
- `E_PHP_DUPLICATE_CLOSURE_USE_VARIABLE`
- `E_PHP_HIR_MISSING_CHILD`
- `E_PHP_DUPLICATE_USE_ALIAS`
- `E_PHP_DUPLICATE_DECLARATION`
- `E_PHP_MIXED_NAMESPACE_DECLARATIONS`
- `E_PHP_NAMESPACE_MUST_BE_FIRST_STATEMENT`
- `E_PHP_INVALID_TYPE_VOID_CONTEXT`
- `E_PHP_INVALID_TYPE_NEVER_CONTEXT`
- `E_PHP_INVALID_TYPE_STATIC_CONTEXT`
- `E_PHP_INVALID_TYPE_SELF_CONTEXT`
- `E_PHP_INVALID_TYPE_PARENT_CONTEXT`
- `E_PHP_INVALID_TYPE_CALLABLE_CONTEXT`
- `E_PHP_DUPLICATE_TYPE_ALTERNATIVE`
- `E_PHP_DUPLICATE_MODIFIER`
- `E_PHP_INCOMPATIBLE_MODIFIERS`
- `E_PHP_BREAK_NOT_IN_LOOP_OR_SWITCH`
- `E_PHP_CONTINUE_NOT_IN_LOOP_OR_SWITCH`
- `E_PHP_INVALID_BREAK_CONTINUE_LEVEL`
- `E_PHP_RETURN_OUTSIDE_ALLOWED_CONTEXT`
- `E_PHP_RETURN_VALUE_FROM_VOID_FUNCTION`
- `E_PHP_RETURN_FROM_NEVER_FUNCTION`
- `E_PHP_YIELD_OUTSIDE_FUNCTION`
- `E_PHP_GOTO_LABEL_NOT_FOUND`
- `E_PHP_INVALID_CONST_EXPR`
- `E_PHP_ATTRIBUTE_ARGUMENT_NOT_CONST_EXPR`
- `E_PHP_DUPLICATE_CLASS_MEMBER`
- `E_PHP_ENUM_CASE_VALUE_ON_UNIT_ENUM`
- `E_PHP_ENUM_CASE_MISSING_VALUE_ON_BACKED_ENUM`
- `E_PHP_TRAIT_ADAPTATION_INVALID_SHAPE`
- `E_PHP_INVALID_CLASS_CONTEXT_NAME`
- `E_PHP_INVALID_MAGIC_METHOD_SIGNATURE`
- `E_PHP_INVALID_STRICT_TYPES_DECLARE`
- `E_PHP_STRICT_TYPES_DECLARE_NOT_FIRST`
- `E_PHP_INVALID_VOID_CAST`
- `W_PHP_REFERENCE_BEHAVIOR_DEFERRED`
- `N_PHP_RUNTIME_CHECK_DEFERRED`

## Semantic Coverage

- Namespaces: braced/unbraced namespace blocks, namespace placement checks,
  mixed namespace form diagnostics.
- Imports: class, function, const, grouped imports, alias collision checks.
- Declarations: functions, constants, class-like declarations, conditional
  declaration metadata, same-file duplicate checks.
- Scopes: file, namespace, function, method, closure, arrow function,
  global/static statements, closure-use metadata.
- Types: unions, intersections, DNF types, nullable forms, contextual invalid
  `void`, `never`, `static`, `self`, `parent`, and `callable` cases.
- Constant expressions: scalar and array forms, class constant fetches,
  conservative literal folding, invalid variable/call checks, PHP 8.5 closure,
  new, cast, and first-class callable fixtures.
- Attributes: target metadata, argument constant-expression validation,
  repeated attributes, class/function/method/parameter/property/enum coverage.
- Class-like constructs: classes, interfaces, traits, enums, properties,
  methods, constants, modifiers, property hooks, constructor promotion, trait
  adaptations, enum cases, magic method diagnostics.
- PHP 8.5 features: pipe operator fixtures, clone-with fixtures, closure and
  first-class callable constant-expression fixtures, cast constant-expression
  fixtures, and pinned-reference rejection for `(void)` cast syntax.

## Risks for Runtime

- Runtime-dependent semantics: includes, requires, eval, runtime fallback, and
  dynamic symbol creation need a runtime or linker model.
- Include/eval: Semantic frontend records deferred metadata but does not execute files or
  parse runtime strings.
- Autoloading/cross-file: class and function lookup must become file-set aware
  before bytecode or IR linking.
- Exact error-message compatibility: Semantic frontend has stable IDs but does not
  guarantee byte-for-byte PHP fatal error messages.
- References/copy-on-write: not modeled in Semantic frontend and must not be inferred from
  HIR alone.
- CFG checks: full `goto` boundary validation and more precise control-flow
  legality belong in Runtime.

## Runtime Handoff

Start Runtime from `php_semantics::query::frontend::analyze_file`.

Recommended first steps:

1. Treat `semantic_diagnostics` error severity as a pre-bytecode gate.
2. Consume `FrontendResult::module()` and
   `FrontendResult::database().module(module_id)` instead of parser events.
3. Use HIR source maps for byte-span attribution in new diagnostics.
4. Build CFG-level validation for `goto`, branch targets, and function-like
   boundaries.
5. Introduce cross-file declaration linking and autoload-aware class lookup.
6. Define include/require/eval behavior as a separate bounded runtime/linking
   layer.
7. Keep parser diagnostics and semantic diagnostics separate.

## CI and Nix Status

- `flake.nix` provides Rust tooling, `just`, `sccache`, `ccache`, and
  `shellcheck` in the dev shell.
- Linux dev shells use `mold` through Rust link args.
- The Semantic frontend GitHub workflow runs the same command as local verification:
  `nix develop -c just verify-frontend`.
- `verify-frontend` does not run corpus, fuzz, or benchmark jobs by default.
- Soft targets are explicit:
  `semantic-corpus-smoke`, `fuzz-frontend-smoke`, and `bench-frontend`.

## Final Assessment

Semantic frontend is implemented and verified for the bounded semantic frontend scope.
The remaining gaps are documented, fixture-backed, and assigned to later layers
where they require CFG, cross-file, autoload, or runtime behavior.
