# Fastest Engine Hotpaths

This report ranks engine work from VM counters and existing performance artifacts. Wall-clock timings are not used for priority.

## Inputs

| Input | Status | Records | Reason |
| --- | --- | ---: | --- |
| `benchmark_smoke` | `ok` | 15 |  |
| `framework_smoke` | `ok` | 9 |  |
| `acceleration_matrix` | `ok` | 127 |  |
| `counter_json` | `ok` | 286 |  |

## Ranked Areas

| Rank | Area | Counter events | Class | Top evidence | Next evidence |
| ---: | --- | ---: | --- | --- | --- |
| 1 | Optimizer And Runtime Allocation | 189725 | `very_high` | `tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php` via `benchmark-smoke` (1787) | Destructor, reference, COW, output-order, and verifier-bracketed optimizer fixtures. |
| 2 | Dispatch | 90989 | `very_high` | `target/performance/array-fast-paths-v2.counters.json` via `counter-json` (924) | Dense opcode, quickening, and superinstruction A/B fixtures. |
| 3 | Strings And Output | 33465 | `very_high` | `tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php` via `benchmark-smoke` (346) | Output-buffer callback, object conversion, binary string, and diagnostic-order fixtures. |
| 4 | Calls And Builtins | 3558 | `high` | `target/performance/inline-cache-smoke/inline_cache-builtin-fast-stubs.on.counters.json` via `counter-json` (272) | Call-shape, by-reference, named-argument, method visibility, and stdlib diffs. |
| 5 | Arrays And Foreach | 3068 | `high` | `target/performance/array-fast-paths-v2.counters.json` via `counter-json` (125) | Packed, mixed, numeric-string key, by-ref foreach, COW, mutation, and order fixtures. |
| 6 | Properties And Methods | 2691 | `high` | `target/performance/inline-cache-smoke/inline_cache-property-shape-guards.on.counters.json` via `counter-json` (113) | Visibility, typed/readonly properties, magic, hooks, dynamic properties, and override fixtures. |
| 7 | Native And JIT Candidates | 20 | `low` | `target/performance/jit-cranelift-counters.json` via `counter-json` (17) | Feature-gated JIT rows with interpreter fallback, compile-budget, and side-exit reports. |
| 8 | Frontend Byte Scanning | 0 | `no_current_counter_evidence` | No current counter events | Lexer/parser/CST parity plus byte-kernel tests before call-site replacement. |

## Optional Profilers

- `callgrind`: `skipped` at `target/performance/callgrind/summary.json`: Callgrind is only supported by this gate on Linux; host is Darwin
- `linux-perf`: `skipped` at `target/performance/perf*.json`: no Linux perf artifact found

## Correctness Policy

The report is advisory for prioritization only; any optimization must still prove stdout, stderr/runtime diagnostics, exit status, fallback counters, and focused fixture parity.
