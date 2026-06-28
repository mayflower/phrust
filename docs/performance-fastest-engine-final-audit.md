# Fastest Engine Final Audit

Date: 2026-06-28.

This audit records the fastest-engine work, FPE-00 through FPE-30, against the
current source tree. It is an engine/runtime performance sweep for
the existing PHP 8.5.7 target (`php-8.5.7`). It does not introduce a web server,
production SAPI, Zend ABI compatibility, extension ABI compatibility, OPcache
replacement, or a pinned PHP target update.

## Feature Status

| Feature | Status | Evidence |
| --- | --- | --- |
| FPE-00 through FPE-13 | Implemented | Prior committed slices establish the plan, hotpath reports, byte-kernel/frontend integration, dense fallback accounting, superinstruction mining, polymorphic call ICs, property/method/array/string fast paths, allocation counters, optimizer guidance, and explicit fast preset policy. |
| FPE-14 | Implemented | Selective Cranelift expansion remains default-off and limited to proven regions; unsupported object, dynamic, and reference-heavy opcodes are conservatively rejected. See `docs/performance-selective-cranelift-regions.md`. |
| FPE-15 | Implemented | Baseline native tier remains no-exec research only, with `php-vm dump-baseline-native-stencil`, `just baseline-native-stencil-smoke`, and ADR prerequisites. See `docs/adr/fast-baseline-native-tier-prerequisites.md`. |
| FPE-16 | Implemented | VM-owned deopt/live-state metadata is report-only and rejects unsupported rich state before any resume guess. See `docs/performance-deopt-live-state-osr-metadata.md`. |
| FPE-17 | Implemented | `just fastest-engine-matrix` writes correctness-first matrix artifacts and committed summary results. See `docs/performance-fastest-engine-results.md`. |
| FPE-18 | Implemented | This final audit records compatibility, safety, fallback, counter, artifact, gap, and no-regression evidence. |
| FPE-19 | Implemented | Request-local allocation and persistent immutable engine-data boundaries are documented, with allocation counters and narrow frame/register reuse evidence. See `docs/research/request-local-arenas.md`. |
| FPE-20 | Implemented | Persistent feedback remains advisory/default-off, rejects stale or corrupt metadata, and exposes stats outside PHP stdout. See `docs/research/persistent-type-feedback.md`. |
| FPE-21 | Implemented | Numeric-string classification is centralized for conversion-sensitive paths, with cache/specialization counters and guarded arithmetic specialization. See `docs/performance-numeric-string-cache.md`. |
| FPE-22 | Implemented | Alias/reference state is classified fail-closed and poisons optimized paths through counters and fallback policy. See `docs/performance-reference-aliasing-deopt.md`. |
| FPE-23 | Implemented | Record-like and small-map array shape metadata feeds guarded helper reads without changing PHP array representation or ordering. See `docs/performance-array-shapes.md`. |
| FPE-24 | Implemented | Include/autoload graph evidence records hit/miss, invalidation, and negative lookup behavior while preserving include order and side effects. See `docs/research/include-autoload-dependency-graph.md`. |
| FPE-25 | Implemented | The builtin intrinsic ladder adds exact interpreter intrinsics for the safe top string/builtin cases beyond the initial `strlen`/`count` stubs. See `docs/performance-builtin-intrinsics.md`. |
| FPE-26 | Implemented | Specialized call-frame layout metadata and a narrow tiny-leaf executable fast path reduce generic frame pressure without retaining userland state. See `docs/performance-specialized-call-frames.md`. |
| FPE-27 | Implemented | Hot/cold slow-path attribution and conservative outlining keep shared semantic helpers while reporting slow-path reasons. See `docs/performance-hot-cold-splitting.md`. |
| FPE-28 | Implemented | Region profiling is opt-in, metadata-only, file-output only, and classifies bounded framework-like traces. See `docs/performance-region-profiling.md`. |
| FPE-29 | Implemented | Copy-and-patch stencil research is no-exec, maps a tiny quickening-compatible dense subset to textual stencils, and writes local reports under `target/performance/stencils/`. See `docs/research/copy-and-patch-stencil-tier.md`. |
| FPE-30 | Implemented | PHP-aware mid-tier research is metadata-only, classifies dense functions/regions, and reports guards, helpers, deopt points, and rejection reasons under `target/performance/mid-tier/`. See `docs/research/php-mid-tier-compiler.md`. |

No FPE-00 through FPE-30 item is intentionally skipped.
Optional tools and rows that are unavailable locally are classified below.

## Defaults And Off Switches

- `--exec-format=ir` remains the correctness baseline.
- Dense bytecode is explicit or `auto` fallback only; strict unsupported shapes
  report deterministic unsupported reasons.
- `--engine-preset=baseline` keeps experimental speed features off.
- `--engine-preset=fast` is explicit opt-in and still preserves fallback paths.
- `--engine-preset=experimental-jit` is explicit opt-in and requires the
  Cranelift feature/runtime selection path.
- Cranelift remains feature-gated and runtime-off by default.
- Quickening, inline caches, superinstructions, bytecode cache, dense execution,
  tiering, and JIT behavior remain controllable through CLI flags and preset
  policy.
- Generated reports remain under `target/` and are not committed.

## Fallback And Safety Policy

- Dense bytecode `auto` can fall back to rich IR for unsupported property,
  method, static, include/eval, exception, generator, fiber, reference, and
  object-heavy shapes.
- Quickening, inline caches, superinstructions, packed-array helpers, output
  batching, method/property caches, and builtin stubs keep generic helper
  fallback paths plus hit/miss or fallback counters.
- Cranelift side exits keep interpreter fallback, side-exit accounting, and
  blacklist counters; generic function JIT remains out of scope.
- Baseline native research is no-exec only; no executable memory policy or
  runtime code cache is introduced.
- Deopt metadata is report-only and rejects reference/COW identity, foreach
  state, try/finally, exceptions, generators/fibers, output-buffer state, and
  unsupported control flow until exact live-state proof exists.
- Runtime safety gates keep `unsafe` out of the runtime and VM crates. The
  broader performance safety audit runs under `just verify-performance`; Miri is
  skipped on this host because the installed toolchain cannot run the configured
  Miri smoke.

## Counters And Reports

The fastest-engine work exposes counters for dense bytecode lowering/execution
families, unsupported and auto-fallback reasons, superinstruction
candidate/emitted/executed/fallback maps, quickening hits and guard failures,
function/builtin/method/property inline-cache hits and misses, packed-array
fast paths, COW/reference/object allocation pressure, output batching and concat
fallbacks, optimizer attempted/applied/skipped passes, Cranelift side exits and
blacklists, baseline-native stencil estimates, and fastest-engine matrix row
classification.

Generated local artifacts include:

- `target/performance/acceleration/summary.json`
- `target/performance/acceleration/summary.md`
- `target/performance/fastest/matrix.json`
- `target/performance/fastest/matrix.md`
- `target/performance/fastest/runs/`
- `target/performance/fastest/compile/`
- `target/performance/fastest/hotpath-report.json`
- `target/performance/fastest/hotpath-report.md`
- `target/performance/benchmark-smoke.json`
- `target/performance/baseline-native-stencil/`
- `target/performance/stencils/`
- `target/performance/mid-tier/`
- `target/performance/bytecode-exec-smoke/summary.json`

Committed summaries and methodology docs include:

- `docs/performance-fastest-engine-plan.md`
- `docs/performance-fastest-engine-known-gaps.md`
- `docs/performance-fastest-engine-results.md`
- `docs/performance-acceleration-results.md`
- `docs/performance-fastest-hotpaths.md`
- `docs/performance-selective-cranelift-regions.md`
- `docs/performance-deopt-live-state-osr-metadata.md`
- `docs/adr/fast-baseline-native-tier-prerequisites.md`
- `docs/research/request-local-arenas.md`
- `docs/research/persistent-type-feedback.md`
- `docs/performance-numeric-string-cache.md`
- `docs/performance-reference-aliasing-deopt.md`
- `docs/performance-array-shapes.md`
- `docs/research/include-autoload-dependency-graph.md`
- `docs/performance-builtin-intrinsics.md`
- `docs/performance-specialized-call-frames.md`
- `docs/performance-hot-cold-splitting.md`
- `docs/performance-region-profiling.md`
- `docs/research/copy-and-patch-stencil-tier.md`
- `docs/research/php-mid-tier-compiler.md`

## Remaining Gaps

The remaining fastest-engine gaps are intentionally visible in
`docs/performance-fastest-engine-known-gaps.md`. The most important unresolved
areas are broader real-workload representativeness, default-on fast preset
promotion, dense object/method/include execution, broader builtin intrinsics,
executable native-tier prerequisites, production-grade OSR resume, broader
optional Cranelift regions, executable copy-and-patch stencils, and executable
PHP-aware mid-tier compilation.

The comparative fastest-engine matrix is correctness-first and host-local. It
does not make a global fastest-PHP claim.

## Validation Evidence

| Command | Result | Notes |
| --- | --- | --- |
| `nix develop -c cargo fmt --all --check` | Pass | Formatting was checked before this audit document was written. |
| `nix develop -c cargo clippy --workspace --all-targets -- -D warnings` | Pass | Workspace clippy was clean. |
| `nix develop -c cargo test --workspace` | Pass | Workspace tests were clean. |
| `nix develop -c cargo test -p php_vm_cli dump_ --bin php-vm` | Pass | CLI report commands are covered, including the baseline stencil, copy-and-patch stencil, and mid-tier metadata commands. |
| `nix develop -c cargo test -p php_jit -p php_vm` | Pass | `php_jit` unit/doc tests passed; `php_vm` ran 412 unit tests plus doc tests. |
| `nix develop -c python3 -m py_compile scripts/performance/fastest_engine_matrix.py` | Pass | Matrix script compiles. |
| `nix develop -c just fastest-engine-matrix` | Pass | Latest local sweep reported 60 enabled rows, 36 skipped optional rows, and 0 known-gap rows. |
| `nix develop -c just baseline-native-stencil-smoke` | Pass | Compared 4 fixtures and 156 dense instructions; wrote `target/performance/baseline-native-stencil/summary.json`. |
| `nix develop -c just copy-patch-stencil-smoke` | Pass | Compared 4 fixtures, 313 dense instructions, and 276 stencils; wrote `target/performance/stencils/summary.json`. |
| `nix develop -c just mid-tier-plan-smoke` | Pass | Classified 4 eligible and 3 rejected functions; wrote `target/performance/mid-tier/summary.json`. |
| `nix develop -c just safety-audit-smoke` | Pass with local skip | Bytecode-cache corruption tests passed; Miri portion skipped because `cargo-miri` is present but not usable for the active toolchain. |
| `nix develop -c just verify-performance` | Pass | Includes fastest-engine matrix, acceleration matrix, baseline/copy-patch/mid-tier report smokes, safety audit smoke, and performance report gates. Expected local skips: Callgrind on Darwin, JIT/Cranelift rows not requested, persistent-feedback rows default-off, Miri unusable on this host. |
| `nix develop -c just verify-frontend` | Pass | Lexer, parser/CST, semantic fixtures, reference comparisons, and frontend CLI gates passed with existing known semantic gaps classified. |
| `nix develop -c just verify-runtime` | Pass | Bytecode snapshots, VM smokes, runtime fixtures, runtime semantics diffs, known-gap catalog checks, and runtime hardening lints passed. |
| `nix develop -c just verify-stdlib` | Pass | Stdlib docs, coverage, unit tests, and differential reports passed; stdlib diff reported 36 pass and 6 known gaps in the core stdlib area. |
| `nix develop -c just verify-phpt` | Pass | PHPT foundation, committed full baseline verification, php-src source integrity, and `php_phpt_tools` tests passed. Baseline verification covered 21,548 corpus entries and 20,428 known non-green fingerprints. |
| `nix develop -c just phpt-verify-source-integrity` | Pass | Verified 24,475 pinned php-src manifest entries; skipped 0 host-generated entries. |
| `nix develop -c just phpt-module MODULE=operators.conversions` | Pass | Reference and target both ran 5 tests with 0 non-green outcomes. |
| `nix develop -c just phpt-module MODULE=standard.output` | Pass | Reference and target both ran 11 tests with 0 non-green outcomes. |
| `nix develop -c just phpt-module MODULE=standard.arrays` | Pass | Reference and target both ran 17 tests with 0 non-green outcomes. |
| `nix develop -c just phpt-module MODULE=standard.strings` | Pass | Reference and target both ran 16 tests with 0 non-green outcomes. |
| `nix develop -c just phpt-module MODULE=objects.classes` | Non-green target | Reference ran 200 tests with 0 non-green outcomes. Target ran 200 tests with 36 non-green outcomes in existing object-runtime gap areas including static property references, autoload, iterators, exceptions, eval declarations, serialization, and class constants. This remains visible and is not claimed as closed by fastest-engine work. |

The full PHPT regression command was not run in this final local sweep because
the project already treats the 21,548-test full corpus as a heavyweight
regression gate with committed baseline verification. This audit used
`just verify-phpt`, explicit source-integrity verification, and focused module
batches for the touched behavior areas instead.

## Boundary Confirmation

- PHP target remains `8.5.7` / `php-8.5.7`.
- No generated reports under `target/` are intended for commit.
- No extracted `php-src` corpus files or vendored php-src copies are intended
  for commit.
- No web server, production SAPI, Zend ABI emulation, extension ABI emulation,
  OPcache replacement, default-on JIT, executable baseline-native tier, or
  runtime VM semantic shortcut was added.
