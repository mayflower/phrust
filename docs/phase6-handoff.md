# Phase 6 Handoff

Phase 6 starts from the Phase 5 runtime semantics boundary and should consume
the existing frontend, HIR, IR, runtime, VM, fixture harnesses, and known-gap
catalog. It should not add a second lexer, parser, AST, semantic frontend, or
source-string execution path.

## Starting Point

- Final Phase 5 gate: `nix develop -c just verify-phase5`
- Reference-backed coverage snapshot: `docs/phase5-coverage-matrix.md`
- Known-gap catalog: `docs/phase5-known-gaps.md`
- Runtime contract: `docs/phase5-runtime-contract.md`
- Hardening audit: `docs/phase5-unsafe-audit.md`

## Phase 6 Topics

| Topic | Concrete next work | Starting evidence |
| --- | --- | --- |
| Standard library | Add Tier 1 builtin coverage for framework boot paths: array helpers, string helpers, `count`, `is_*`, `class_exists`/`interface_exists` edge cases, and argument/type diagnostics. Keep every unsupported builtin behind a specific diagnostic. | `E_PHP_RUNTIME_UNSUPPORTED_STDLIB`, `E_PHP_RUNTIME_BUILTIN_ARITY`, `fixtures/phase5/real_world/*.php` |
| SPL and Reflection expansion | Expand `Iterator`, `IteratorAggregate`, `ArrayAccess`, Reflection classes, ReflectionEnum APIs, callable reflection, constructor/new-instance paths, and attribute target/repetition enforcement. | `docs/phase5-reflection-attributes.md`, `fixtures/phase5/reflection/*.php`, `fixtures/phase5/foreach/arrayaccess-known-gap.php` |
| Streams | Introduce deterministic stream/file wrappers for local file reads, include path behavior, path normalization, and warning/fatal rendering. Do not implement network streams in the required gate. | `E_PHP_RUNTIME_UNSUPPORTED_STREAM_WRAPPER`, `fixtures/phase5/include_eval_autoload/*.php` |
| JSON, PCRE, Date | Add small but real extension-like surfaces for `json_encode`/`json_decode`, `preg_*` basics, and DateTime construction/formatting because Composer/framework smokes commonly need them. Keep extension breadth explicit. | `E_PHP_RUNTIME_UNSUPPORTED_STDLIB`, Composer-style known gaps |
| Composer smokes | Add local, offline Composer-subset fixtures that are checked into `fixtures/phase6/` or generated deterministically. Keep user-provided Composer projects opt-in and out of required CI. | `just phase5-local-composer-smoke <paths>`, `just phase5-composer-smoke`, `E_PHP_RUNTIME_COMPOSER_AUTOLOAD_MATRIX`, `E_PHP_RUNTIME_COMPOSER_STDLIB_MATRIX` |
| Performance Tier 1 | Add stable microbenchmarks for parse-to-run, function calls, array append/read, property access, method dispatch, generator resume, fiber suspend/resume, and Reflection metadata reads. Treat them as trend evidence, not compatibility gates. | `just phase5-bench-smoke`, `docs/phase5-generators-fibers.md`, `docs/phase5-array-semantics.md`, `docs/phase5-object-semantics.md` |
| Fuzz/property expansion | Promote the optional deterministic reference/COW/foreach fuzz smoke into a larger property suite with minimization, corpus promotion rules, and stable seeds per bug class. | `just phase5-fuzz-smoke`, `scripts/minimize_phase5_failure.py`, `fixtures/phase5/regressions/` |
| Bytecode cache | Define a versioned cache format for lowered IR/bytecode plus invalidation on source hash, PHP target version, feature flags, and semantic metadata version. | `php_ir`, `php_vm_cli`, Phase 4 IR docs |
| Extension API | Define a minimal Rust-native internal extension boundary for builtins and predefined classes before considering Zend ABI compatibility. Zend ABI emulation remains out of scope unless a later phase explicitly accepts it. | `E_PHP_RUNTIME_UNSUPPORTED_ZEND_ABI`, `crates/php_runtime/src/builtins.rs` |

## First Phase 6 Gate Shape

The first Phase 6 gate should preserve all earlier gates:

```bash
nix develop -c just verify-phase0
nix develop -c just verify-phase1
nix develop -c just verify-phase2
nix develop -c just verify-phase3
nix develop -c just verify-phase4
nix develop -c just verify-phase5
```

Then add a narrow `verify-phase6` target that initially runs only deterministic
Phase 6 fixtures and preserves clear skip behavior for reference-dependent
checks when `REFERENCE_PHP` is unavailable.

## Rules for Closing Gaps

- Move a known gap to implemented only when it has fixture evidence and passes
  against `REFERENCE_PHP` when reference behavior is observable.
- Split broad `UNSUPPORTED` IDs before adding executable paths beneath them.
- Keep generated reports under `target/`; do not commit reference output or
  vendored Composer/php-src trees.
- Preserve byte-based spans and structured diagnostics even when adding more
  PHP-compatible user-facing error text.
