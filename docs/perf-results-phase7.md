# Phase 7 Performance Results

Phase 7 performance artifacts are generated under `target/phase7` and are not
committed because wall-clock timings are host-specific. This document records the
latest local snapshot and the conservative budgets used for Phase 7 gates.

## Latest Local Snapshot

| Field | Value |
| --- | --- |
| Date | 2026-06-23 |
| Commit | `81ac9ae6b32b` |
| Worktree | Dirty local Phase 7 implementation worktree; use as development evidence, not a release baseline. |
| Platform | macOS 26.5, Darwin 25.5.0, arm64 (`arm64-darwin`) |
| Commands | `nix develop -c just perf-baseline`, `nix develop -c just perf-compare`, `nix develop -c just perf-report` |
| Baseline artifact | `target/phase7/baseline.json` |
| Current artifact | `target/phase7/bench-phase7-smoke.json` |
| Comparison artifact | `target/phase7/perf-compare.md` and `target/phase7/perf-compare.json` |
| Report artifact | `target/phase7/perf-report.md` and `target/phase7/perf-report.json` |

The latest comparison covered 24 measurements, 168 comparable metrics, no
missing scenarios, no added scenarios, and 0 hard regressions. The current smoke
uses one measured repetition and no warmups; the baseline uses three measured
repetitions and one warmup. Treat wall-clock deltas below as advisory only.

## Scenario Coverage

The benchmark smoke covers these deterministic Phase 7 scenarios for both
`rust-vm` and `reference-php` when the reference binary is available:

- `arithmetic`
- `arrays_mixed`
- `arrays_packed`
- `autoload_smoke`
- `exceptions_no_throw`
- `function_calls`
- `loops`
- `method_calls`
- `output_writes`
- `properties`
- `stdlib_dispatch`
- `strings_concat`

The Prompt 07.56 correctness matrix separately compared 18 fixtures across 6
non-baseline variants, for 108 behavior comparisons:

- `opt1`
- `opt2`
- `quickening-on`
- `inline-caches-on`
- `bytecode-cache-read-write`
- `all-non-jit-on`

JIT matrix coverage is opt-in with `PHRUST_PHASE7_MATRIX_JIT=1` because native
JIT support remains platform/feature constrained and default-off.

## Relative Wall-Clock Changes

Rust VM median wall-clock changes from the latest `perf-compare` run:

| Scenario | Baseline median | Current median | Change |
| --- | ---: | ---: | ---: |
| `arithmetic` | 15.278 ms | 303.294 ms | +1885.13% |
| `arrays_mixed` | 13.581 ms | 41.112 ms | +202.72% |
| `arrays_packed` | 14.509 ms | 90.090 ms | +520.90% |
| `autoload_smoke` | 17.330 ms | 96.162 ms | +454.89% |
| `exceptions_no_throw` | 15.248 ms | 36.096 ms | +136.72% |
| `function_calls` | 15.378 ms | 67.108 ms | +336.39% |
| `loops` | 13.379 ms | 39.806 ms | +197.53% |
| `method_calls` | 18.496 ms | 28.950 ms | +56.52% |
| `output_writes` | 16.182 ms | 40.817 ms | +152.24% |
| `properties` | 15.573 ms | 63.674 ms | +308.88% |
| `stdlib_dispatch` | 19.094 ms | 86.006 ms | +350.44% |
| `strings_concat` | 16.056 ms | 43.862 ms | +173.18% |

This run does not prove a wall-clock improvement. The benchmark harness is still
useful as a deterministic smoke for output, exit status, stderr, counters, and
artifact generation, but it is not calibrated enough to be a hard performance
budget.

## Counter Snapshot

The latest `perf-report` aggregated these notable Rust VM counters from the
default benchmark smoke:

| Counter | Total |
| --- | ---: |
| `instructions_executed` | 1994 |
| `local_slot_fast_path_hits` | 706 |
| `literal_intern_misses` | 296 |
| `output_bytes` | 242 |
| `literal_intern_hits` | 128 |
| `output_buffer_appends` | 79 |
| `function_calls` | 70 |
| `internal_function_dispatches` | 59 |
| `internal_function_dispatch_cache_hits` | 51 |
| `property_accesses` | 26 |
| `array_count_fast_path_hits` | 17 |
| `internal_count_array_direct_fast_path_hits` | 17 |

The default benchmark command does not pass quickening, inline-cache, bytecode
cache, or JIT flags, so those counters are expected to be zero in
`perf-report`. Their correctness and counter evidence comes from dedicated gates
such as `quickening-smoke`, `inline-cache-smoke`, `cache-roundtrip`,
`jit-smoke`, and `perf-flag-matrix`.

## Budgets

Phase 7 budgets are intentionally conservative:

- Correctness is mandatory. `verify-phase7`, `regression-phase7`, and
  `perf-flag-matrix` must not show output, exit-status, stderr, or diagnostic
  differences between the baseline and enabled performance flags.
- Wall-clock is advisory. `perf-compare` may report large local changes without
  failing unless a caller explicitly uses `--fail-on-regression-percent`.
- Callgrind is optional and platform-gated. When `bench-phase7-callgrind-smoke`
  runs on a supported Linux host, a massive stable instruction-count regression
  should block promotion. On the current Darwin snapshot it skips cleanly.
- Counter activity must be interpreted by layer. Local-slot, output, literal,
  array, and internal-dispatch counters show active infrastructure in the
  default smoke; quickening, IC, cache, and JIT require their dedicated gates or
  an explicit optimized benchmark run.

## Optimization Readout

Evidence from this snapshot:

- Measurable counter activity exists for local-slot fast paths, literal pools,
  output buffering, array count fast paths, and internal function dispatch
  caching.
- Bytecode cache, optimizer, quickening, inline caches, tiering, and JIT have
  correctness gates and behavior-matrix coverage, but the default wall-clock
  smoke does not isolate their speed contribution.
- No optimization layer should claim a production speedup from this snapshot
  alone. The current value is regression protection and instrumentation.

## Reproducing

```bash
nix develop -c just perf-baseline
nix develop -c just perf-compare
nix develop -c just perf-report
```

Use `target/phase7/perf-compare.json` for machine-readable comparison data and
`target/phase7/perf-report.md` for the generated local report.
