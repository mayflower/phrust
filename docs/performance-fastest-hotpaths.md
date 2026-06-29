# Fastest Engine Hotpaths

This report ranks engine work from VM counters and existing performance artifacts. Wall-clock timings are not used for priority.

## Inputs

| Input | Status | Records | Reason |
| --- | --- | ---: | --- |
| `benchmark_smoke` | `ok` | 16 |  |
| `framework_smoke` | `ok` | 9 |  |
| `acceleration_matrix` | `ok` | 127 |  |
| `counter_json` | `ok` | 328 |  |

## Ranked Areas

| Rank | Area | Counter events | Class | Top evidence | Next evidence |
| ---: | --- | ---: | --- | --- | --- |
| 1 | Optimizer And Runtime Allocation | 263244 | `very_high` | `tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php` via `benchmark-smoke` (2274) | Destructor, reference, COW, output-order, and verifier-bracketed optimizer fixtures. |
| 2 | Dispatch | 103858 | `very_high` | `tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php` via `acceleration-matrix` (870) | Dense opcode, quickening, and superinstruction A/B fixtures. |
| 3 | Strings And Output | 38104 | `very_high` | `tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php` via `benchmark-smoke` (348) | Output-buffer callback, object conversion, binary string, and diagnostic-order fixtures. |
| 4 | Calls And Builtins | 7142 | `high` | `target/performance/inline-cache-smoke/perf_smoke-stdlib_dispatch.on.counters.json` via `counter-json` (360) | Call-shape, by-reference, named-argument, method visibility, and stdlib diffs. |
| 5 | Arrays And Foreach | 3754 | `high` | `target/performance/quickening-smoke/array_fast_paths_v2.on.counters.json` via `counter-json` (135) | Packed, mixed, numeric-string key, by-ref foreach, COW, mutation, and order fixtures. |
| 6 | Properties And Methods | 2967 | `high` | `target/performance/inline-cache-smoke/inline_cache-property-shape-guards.on.counters.json` via `counter-json` (113) | Visibility, typed/readonly properties, magic, hooks, dynamic properties, and override fixtures. |
| 7 | Include And Autoload | 91 | `low` | `target/performance/inline-cache-smoke/inline_cache-include-path-cache.on.counters.json` via `counter-json` (15) | Include/require warning order, stream-wrapper rejection, Composer autoload, and invalidation fixtures. |
| 8 | Native And JIT Candidates | 20 | `low` | `target/performance/jit-cranelift-counters.json` via `counter-json` (17) | Feature-gated JIT rows with interpreter fallback, compile-budget, and side-exit reports. |

## Optional Profilers

- `callgrind`: `skipped` at `target/performance/callgrind/summary.json`: Callgrind is only supported by this gate on Linux; host is Darwin
- `linux-perf`: `skipped` at `target/performance/perf*.json`: no Linux perf artifact found

## Correctness Policy

The report is advisory for prioritization only; any optimization must still prove stdout, stderr/runtime diagnostics, exit status, fallback counters, and focused fixture parity.
