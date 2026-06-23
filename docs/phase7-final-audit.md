# Phase 7 Final Audit

Date: 2026-06-23.

Reference target: PHP 8.5.7 (`php-8.5.7`).

Phase 7 closes the first performance layer. It keeps the existing pipeline:

```text
php_lexer -> php_syntax -> php_ast -> php_semantics/HIR -> php_ir -> php_runtime -> php_vm -> php_vm_cli
```

No second lexer, parser, AST, semantic frontend, runtime executor, or
source-string execution path was introduced. Phase 7 performance features are
allowed to change internal representation, caching, dispatch, and measurement,
but must not change PHP-visible stdout, exit status, diagnostics, or side-effect
order.

## Required Gates

Run these before closing Phase 7:

```bash
nix develop -c just verify-phase7
nix develop -c just perf-report
```

`verify-phase7` expands to:

```bash
nix develop -c just test-phase7
nix develop -c just regression-phase7
nix develop -c just cache-roundtrip
nix develop -c just optimizer-diff
nix develop -c just quickening-smoke
nix develop -c just inline-cache-smoke
nix develop -c just bench-phase7-callgrind-smoke
nix develop -c just jit-smoke
nix develop -c just phase7-safety-audit-smoke
nix develop -c just bench-phase7-smoke
nix develop -c just hotpaths-phase7
nix develop -c just perf-report
```

The required CI workflow also runs:

```bash
nix flake check
nix develop -c just verify-phase7
```

Long benchmark suites remain optional and are not pull-request gates:

```bash
nix develop -c just bench-phase7
```

## Evidence Map

| Area | Status | Evidence |
| --- | --- | --- |
| Phase scope and principles | Implemented as performance-only internal layers with behavior preservation requirements. | `docs/adr/0070-phase7-performance-scope.md`, `docs/phase7-performance.md` |
| Nix/devshell performance tooling | Implemented with cache/linker optimization where supported and skip-safe optional tooling. | `flake.nix`, `docs/performance-methodology-phase7.md` |
| Phase 7 command gates | Implemented through concrete `just` recipes; no placeholder Phase 7 gate remains. | `justfile`, `docs/phase7-gate-todos.md` |
| Benchmark corpus and runner | Implemented deterministic smoke corpus, JSON runner, baseline/compare/report flow, and hot-path inventory. | `tests/fixtures/phase7/perf_smoke/`, `scripts/phase7/bench_matrix.py`, `scripts/phase7/compare_perf_json.py`, `scripts/phase7/perf_report.py`, `docs/benchmark-corpus-phase7.md`, `docs/perf-results-phase7.md` |
| Bytecode cache | Implemented local disk cache artifacts with fingerprints, target/version/config metadata, verified IR payloads, corrupt-cache fallback, CLI read/write modes, and path-component hardening. | `crates/php_bytecode_cache`, `crates/php_vm_cli/src/main.rs`, `docs/adr/0072-bytecode-cache-format.md`, `docs/bytecode-cache-phase7.md`, `just cache-roundtrip` |
| Optimizer passes | Implemented pass framework, opt levels 0/1/2, safe constant folding, peepholes, branch simplification, CFG verifier checks, and differential output comparison. | `crates/php_optimizer`, `docs/optimizer-passes-phase7.md`, `just optimizer-diff` |
| IR/bytecode invariants | Extended verifier coverage protects optimizer and cache boundaries. | `crates/php_ir/src/verify.rs`, `docs/phase7-ir-verifier.md`, `just ir-verify-phase7` |
| Quickening | Implemented request-local quickening framework and concrete specializations for int add, string concat, and packed-array dimension fetch with guard/fallback counters. | `crates/php_vm/src/quickening.rs`, `docs/quickening-inline-caches-phase7.md`, `just quickening-smoke` |
| Inline caches | Implemented monomorphic function, class/static, include-path, and autoload/class-lookup caches plus fixed-size method/property polymorphic caches with epoch invalidation, megamorphic fallback, and stats. | `crates/php_vm/src/inline_cache.rs`, `docs/adr/0074-quickening-inline-cache-model.md`, `docs/include-path-cache-phase7.md`, `docs/autoload-lookup-cache-phase7.md`, `just inline-cache-smoke`, `just polymorphic-inline-cache-smoke` |
| Runtime fast paths | Implemented local-slot/frame reuse, packed-array, numeric-string, typecheck/prologue, internal-dispatch, `count(array)`, and output-buffer fast-path coverage with counters. | `docs/local-slot-layout-phase7.md`, `docs/array-fast-paths-phase7.md`, `docs/numeric-string-cache-phase7.md`, `docs/typecheck-fast-paths-phase7.md`, `docs/internal-function-dispatch-cache-phase7.md`, `docs/output-buffer-fast-paths-phase7.md` |
| Deopt/fallback protocol | Implemented unified fallback/counter surface for optimized paths. | `crates/php_vm/src/fallback.rs`, `docs/quickening-inline-caches-phase7.md` |
| Stress regressions | Implemented regression fixtures for exceptions, destructors, generators, fibers, references, COW, and autoload invalidation around optimized paths. | `tests/fixtures/phase7/regressions/`, `scripts/phase7/regression_smoke.sh`, `docs/phase7-regressions.md` |
| Full performance-flag A/B matrix | Implemented baseline versus opt1, opt2, quickening, inline caches, bytecode cache, and all-non-JIT optimization combinations across Phase 7 and selected Phase 4-6 fixtures. | `scripts/phase7/perf_flag_matrix.py`, `just perf-flag-matrix` |
| Optional instruction-count smoke | Implemented skip-safe Callgrind smoke on Linux with explicit skip reasons elsewhere. | `scripts/phase7/callgrind_smoke.sh`, `just bench-phase7-callgrind-smoke` |
| Rust hot-path benchmarks | Implemented optional Criterion benchmark crate excluded from the main workspace. | `crates/php_bench`, `just bench-rust-phase7` |
| JIT experiment | Implemented default-off API, eligibility analyzer, ABI handle model, optional Cranelift lowering, guarded int-leaf execution through the VM tiering path, and fallback counters. It is not production native JIT. | `crates/php_jit`, `docs/adr/0076-cranelift-jit-experiment.md`, `docs/jit-experiment-phase7.md`, `just jit-smoke` |
| Optional W^X/mprotect | Reviewed 07.F precondition and documented that Phase 7 does not allocate executable memory or enter native code, so no placeholder mprotect wrapper is valid yet. | `docs/jit-experiment-phase7.md`, `docs/safety-audit-phase7.md`, `docs/known-gaps-phase7.md` |
| Tiering | Implemented interpreter to quickened to JIT policy counters and CLI stats output with default-safe thresholds. | `crates/php_vm/src/tiering.rs`, `docs/quickening-inline-caches-phase7.md` |
| Safety audit | Implemented cache/JIT/adaptive-surface unsafe scan, corrupt-cache tests, path traversal hardening, Miri skip policy, and executable-memory status documentation. | `docs/safety-audit-phase7.md`, `just phase7-safety-audit-smoke` |
| CI/Nix hardening | Implemented Phase 7 workflow with required verify job, manual/scheduled long benchmark job, flake check, report artifacts, and skip policy. | `.github/workflows/phase7.yml`, `docs/phase7-ci-policy.md` |
| Optional profiling | Implemented maintainer-only recipes that skip by default and write profiler artifacts under `target/phase7/profiles/`. | `docs/profiling-workflow-phase7.md`, `scripts/phase7/profile_smoke.sh` |
| Optional release profiles | Documented LTO/PGO experiments without changing default build profiles. | `docs/release-build-profile-phase7.md`, `scripts/phase7/release_profile_plan.sh` |
| Shared-cache research | Compared disk cache, mmap, process-local cache, and future shared memory with security/invalidation risks. | `docs/research-shared-cache-phase7.md` |
| Framework micro-smokes | Implemented offline router, DI-container, attribute/reflection, and template-output smokes with opt-off/on counter comparison and perf-report integration. | `tests/fixtures/phase7/framework_smoke/`, `scripts/phase7/framework_micro_smoke.py`, `just framework-smoke-phase7`, `just perf-report` |
| Known gaps | Current and explicit; no performance claim depends on unstated gaps. | `docs/known-gaps-phase7.md` |

## Current Known Gaps

The authoritative catalog is `docs/known-gaps-phase7.md`. The final Phase 7
carryovers are:

- calibrated performance budgets and optimized-flag benchmark suites;
- broader hot-path corpus representativeness;
- more granular standard-library call counters;
- complete bytecode-cache dependency invalidation for dynamic include paths,
  symlinks, failed include diagnostics, Composer autoload metadata, shared
  memory, preload, and production SAPI lifecycle;
- production native JIT execution, executable-memory ownership, W^X, and native
  entry/exit ABI proof.

These are not silent failures. They are explicit Phase 8-or-later handoffs.

## Phase 8 Handoff

Recommended Phase 8 entry points:

- production SAPI/FPM/daemon lifecycle, including request reset, worker
  recycling, config reloads, and cache lifetime ownership;
- persistent shared bytecode cache, preload semantics, and dependency
  invalidation for includes, Composer autoload metadata, generated class maps,
  symlinks, and working-directory changes;
- Zend ABI and extension strategy boundaries, including which extensions are
  native Rust implementations versus ABI-compatible bridges;
- calibrated performance budgets in a stable CI environment, ideally with
  Linux instruction-count budgets before wall-clock budgets;
- broader offline framework performance smokes for router dispatch,
  dependency-injection lookup, attribute/reflection warm paths, and template
  output;
- optional native JIT continuation only after W^X/executable-memory policy,
  native call ABI proof, deopt safety, and crash containment gates exist;
- packaging and distribution profiles for CLI, daemon, and future production
  deployment forms.

## Closure Criteria

Phase 7 is complete when `verify-phase7` and `perf-report` pass after this
audit is written. Any red gate must be classified as a new regression, an
environment/tool skip with explicit output, or an existing known gap before the
phase can be considered closed.
