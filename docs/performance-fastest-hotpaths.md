# Fastest Engine Hotpaths

This report ranks engine work from VM counters and existing performance artifacts. Wall-clock timings are not used for priority.

## Inputs

| Input | Status | Records | Reason |
| --- | --- | ---: | --- |
| `benchmark_smoke` | `ok` | 16 |  |
| `framework_smoke` | `ok` | 9 |  |
| `acceleration_matrix` | `ok` | 127 |  |
| `counter_json` | `ok` | 594 |  |

## Ranked Areas

| Rank | Area | Counter events | Class | Top evidence | Next evidence |
| ---: | --- | ---: | --- | --- | --- |
| 1 | Optimizer And Runtime Allocation | 2205747 | `very_high` | `target/performance/app-flows/runs/collection_transform_pagination/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (292632) | Destructor, reference, COW, output-order, and verifier-bracketed optimizer fixtures. |
| 2 | Dispatch | 640693 | `very_high` | `target/performance/app-flows/runs/collection_transform_pagination/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (53273) | Dense opcode, quickening, and superinstruction A/B fixtures. |
| 3 | Calls And Builtins | 103250 | `very_high` | `target/performance/app-flows/runs/collection_transform_pagination/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (12587) | Call-shape, by-reference, named-argument, method visibility, and stdlib diffs. |
| 4 | Strings And Output | 81617 | `very_high` | `target/performance/app-flows/runs/template_render_escape/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (4179) | Output-buffer callback, object conversion, binary string, and diagnostic-order fixtures. |
| 5 | Arrays And Foreach | 63950 | `very_high` | `target/performance/app-flows/runs/collection_transform_pagination/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (14028) | Packed, mixed, numeric-string key, by-ref foreach, COW, mutation, and order fixtures. |
| 6 | Properties And Methods | 12101 | `very_high` | `target/performance/app-flows/runs/model_hydration_json/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (2400) | Visibility, typed/readonly properties, magic, hooks, dynamic properties, and override fixtures. |
| 7 | Native And JIT Candidates | 1142 | `high` | `target/performance/app-flows/runs/dependency_container_resolution/phrust-release-fast/iter-instrumented.counters.json` via `counter-json` (237) | Feature-gated JIT rows with interpreter fallback, compile-budget, and side-exit reports. |
| 8 | Include And Autoload | 137 | `medium` | `target/performance/inline-cache-smoke/inline_cache-include-path-cache.on.counters.json` via `counter-json` (15) | Include/require warning order, stream-wrapper rejection, generated autoload, and invalidation fixtures. |

## Optional Profilers

- `callgrind`: `skipped` at `target/performance/callgrind/summary.json`: Callgrind is only supported by this gate on Linux; host is Darwin
- `linux-perf`: `skipped` at `target/performance/perf*.json`: no Linux perf artifact found

## Correctness Policy

The report is advisory for prioritization only; any optimization must still prove stdout, stderr/runtime diagnostics, exit status, fallback counters, and focused fixture parity.
