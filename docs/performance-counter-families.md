# App-Flow Overhead Counter Families

The app-flow matrix (`just app-flow-smoke`, `just app-flow-matrix`) captures one
dedicated instrumented run per Phrust row and folds its raw VM counters into
named overhead families. The per-scenario attribution is written to the
local-only artifacts `target/performance/app-flows/overhead.{json,md}` and to
the `overhead_families` field of each row in `matrix.json`. Raw run data is
never committed; this document only defines what each family means.

Family folding is implemented in `scripts/performance/overhead_attribution.py`.
All values are deterministic event counts from `--counters-json`, not time.
Rich-IR and dense-bytecode interpreters record call/property/dim traffic
through disjoint counters, so a family sums both sides without double
counting; cache/fast-path hit counters that observe the same underlying events
are reported as non-summed `details`.

| Family | Meaning | Summed components |
| --- | --- | --- |
| `value_clones` | Runtime `Value` copies observed by the layout recorders. Cheap per event but the highest-volume family; the primary target for clone-elimination work. | `value_clones` |
| `string_allocations` | New string buffer allocations (concats, casts, escapes). | `string_allocations` |
| `array_handle_clones` | Shared array handle clones (COW share bumps) from by-value array flow. | `array_handle_clones` |
| `cow_separations` | Copy-on-write storage deep copies triggered by a write to shared storage. Few events, high per-event cost. | `cow_separations` |
| `object_allocations` | Object storage allocations. | `object_allocations` |
| `function_calls` | User function/closure calls on the rich interpreter plus dense `call_function` executions. | `function_calls`, `opcodes.bytecode_call_function` |
| `method_calls` | Instance/static method calls across both interpreters. | `method_calls`, `opcodes.bytecode_call_method`, `opcodes.bytecode_call_static_method` |
| `property_accesses` | Property fetch/assign/isset traffic across both interpreters. | `property_accesses`, `opcodes.bytecode_fetch_property`, `opcodes.bytecode_assign_property` |
| `array_dim_ops` | Array dimension fetch/assign/append/isset traffic across both interpreters. | `array_dim_fetches`, dense `*_dim` opcode executions |
| `packed_record_arrays` | Packed/record array fast-path hits and the fallbacks that bounced to the generic array path (reasons kept per event). | `array_fast_path_hits_by_family`, `array_fast_path_fallback_by_reason` |
| `builtin_dispatch` | Builtin/internal function dispatches; cache, fast-stub, and intrinsic behavior appears in `details`. | `internal_function_dispatches` |
| `adaptive_bookkeeping` | Quickening and inline-cache observation events on the dispatch hot path — the cost of the adaptive machinery itself. Observations are limited to instruction kinds that can actually specialize or cache. | `quickening_attempts`, `inline_cache_observations` |
| `dense_fallbacks` | Work pushed back to the rich interpreter because dense lowering rejected an instruction/function; reason strings identify the exact gap. | dense/bytecode fallback reason maps |
| `native_execution` | Native/JIT tier executions and side exits (zero in default builds without the JIT feature). | `jit_executed`, `jit_side_exits`, side-exit reason maps |

How to use the report:

1. Run `just app-flow-smoke` (or the full matrix) and open
   `target/performance/app-flows/overhead.md`.
2. The summary table ranks the top three families per scenario; per-scenario
   sections list every family with its components and top fallback reasons.
3. An optimization prompt should name the family it targets, land the change,
   rerun the matrix, and show that family's event count dropping on at least
   one scenario while correctness rows stay green.

`dense_fallbacks` reasons are the highest-leverage entries: a single rejected
instruction keeps an entire function on the rich interpreter, so one reason
string with a hot call count usually explains a scenario-wide gap in the
allocation-side families.
