# Phase 7 Known Gaps

Phase 7 known gaps are tracked here as performance work lands. Prompt 07.58
reviewed this catalog against the developer workflow docs and current gates.
Generated artifacts under `target/phase7` are evidence for local runs, not
portable hard truth, and are intentionally not committed.

| Gap ID | Layer | Evidence/Test | Risk | Planned handoff |
| --- | --- | --- | --- | --- |
| PHASE7-GAP-STDLIB-PER-BUILTIN-COUNTERS | Runtime counters | `docs/hotpaths-phase7.md` now reports `internal_function_dispatches`, `internal_function_dispatch_cache_hits`, and `internal_function_dispatch_cache_misses` for `stdlib_dispatch.php`, but counters are not yet broken down per builtin name. | Standard-library priorities can still be under- or over-attributed when several builtins share one fixture. | Add optional per-builtin attribution in a later reporting pass before making per-function performance claims |
| PHASE7-GAP-HOTPATH-TYPE_CHECKS-NO-EVENTS | Benchmark corpus | Unit tests cover typecheck fast-path hits, misses, and A/B correctness, but `docs/hotpaths-phase7.md` lists Type Checks with zero counter events in the tiny benchmark smoke corpus. | Type-check caches cannot be prioritized from smoke-corpus counter data yet, even though their correctness path is covered separately. | Add an `instanceof`/typed catch performance fixture before making typecheck throughput claims |
| PHASE7-GAP-HOTPATH-CORPUS-REPRESENTATIVENESS | Benchmark corpus | `target/phase7/hotpaths.json` is derived from `tests/fixtures/phase7/perf_smoke`, which is deterministic but intentionally tiny. | Hot-path ranking is useful for local smoke prioritization, not real Composer/framework workload claims. | Expand corpus before promoting hard performance claims |
| PHASE7-GAP-PERF-BUDGET-CALIBRATION | Benchmark budgets | `docs/perf-results-phase7.md` records a fresh local snapshot where the default benchmark smoke has no optimization flags and wall-clock medians are noisy across different warmup/repetition settings. | Phase 7 can prove behavior preservation and counter activity, but cannot yet use host wall-clock deltas as a hard performance budget or per-optimization speedup claim. | Add calibrated optimized-flag benchmark suites and stable Linux Callgrind budgets before hard performance gates |
| PHASE7-GAP-CACHE-INCLUDE-INVALIDATION | Bytecode cache | `docs/bytecode-cache-phase7.md` documents include/require dependency rules, but Phase 7 cache artifacts do not yet record a complete include dependency graph for dynamic include paths, symlinks, working-directory changes, or failed include diagnostics. | A cache hit could become stale if include dependencies were reused without complete fingerprints, so include-sensitive programs must miss until this is implemented. | Phase 8 SAPI/cache lifecycle or a later explicit cache-dependency prompt |
| PHASE7-GAP-CACHE-COMPOSER-AUTOLOAD-DEPS | Bytecode cache | Composer/autoload interaction is documented in `docs/bytecode-cache-phase7.md`; no gate yet fingerprints `vendor/autoload.php`, generated class maps, PSR-4 prefix maps, or autoload registration order. | Composer fixture cache hits cannot be claimed safe without metadata for generated autoload files and runtime autoload behavior. | Phase 8 dependency model before Composer cache-hit claims |
| PHASE7-GAP-CACHE-SHARED-MEMORY-OPCACHE | Bytecode cache / SAPI | `docs/bytecode-cache-phase7.md` states that Phase 7 uses optional local disk artifacts and does not implement a shared-memory OPcache manager. | Later daemon or FPM-like workers need a separate shared-memory ownership, eviction, and invalidation design. | Phase 8 SAPI/daemon design |
| PHASE7-GAP-CACHE-PRELOAD | Bytecode cache / SAPI | `docs/bytecode-cache-phase7.md` states that PHP OPcache-style preloading is not a Phase 7 requirement. | Preloaded declarations and persistent runtime state can change request isolation semantics if added without a dedicated model. | Phase 8-or-later preload ADR |
| PHASE7-GAP-CACHE-PRODUCTION-SAPI-LIFECYCLE | Bytecode cache / SAPI | `docs/bytecode-cache-phase7.md` documents only CLI and future handoff diagrams; no FPM, production SAPI, or worker-pool lifecycle is implemented. | Cross-request cache sharing, worker recycling, config reloads, and request cleanup remain undefined for production use. | Phase 8 SAPI/daemon design |
| PHASE7-GAP-JIT-NATIVE-EXECUTION | JIT | `docs/adr/0076-cranelift-jit-experiment.md` accepts only a default-off experiment; `crates/php_jit` and `jit-smoke` now cover the API skeleton, conservative eligibility analyzer, handle-based ABI, optional Cranelift IR lowering, guarded safe int-leaf execution under `--jit=on`, Prompt 07.54 tiering/stats, Prompt 07.55 safety audit coverage, and Optional 07.F precondition review, but do not allocate executable memory or enter native machine code. | A production native execution claim would be unsafe without W^X, executable-memory ownership, native entry/exit ABI proof, and native execution smoke gates. | Phase 8+ native-code handoff; add W^X/mprotect owner only when native machine-code execution exists |

## Entry Format

- `Gap ID`: stable identifier such as `PHASE7-GAP-BYTECODE-CACHE-CORRUPT`.
- `Layer`: bytecode cache, optimizer, quickening, inline cache, specialization,
  JIT, benchmark infrastructure, or runtime fast path.
- `Evidence/Test`: fixture, gate, report, or command proving the gap exists.
- `Risk`: correctness, security, invalidation, portability, or measurement risk.
- `Planned handoff`: later Phase 7 prompt, optional prompt, or future phase.
