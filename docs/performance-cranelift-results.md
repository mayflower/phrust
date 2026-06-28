# Performance Cranelift Results

Date: 2026-06-23.

Reference target: PHP 8.5.7 (`php-8.5.7`).

This document records the local Work item.16 through 07.CL.32 Cranelift big-win
matrix. The generated source report is:

```text
target/performance/cranelift/big_wins_report.json
```

Generated reports under `target/` are local artifacts and are not committed.

## Run Metadata

| Field | Value |
| --- | --- |
| Command | `nix develop -c just jit-cranelift-report` |
| Git commit | `c5931f2b7f68` |
| Rust target triple | `aarch64-apple-darwin` |
| Feature flags | `jit-cranelift` |
| CLI flags | `--jit=off`, `--jit=cranelift`, `--jit-eager`, threshold rows with `--jit-threshold=N`, `--jit-stats=json` |
| Warmups | 1 |
| Iterations per row | 3 |
| Timing policy | Wall-clock totals are informational only; there is no hard speedup gate. Compile, execution, and total costs are reported separately. |
| Report schema | `schema_version: 2`; rows include `matrix_family`, `matrix_family_label`, `path_kinds`, and `command`. |
| Reference PHP | Optional advisory orientation only. The local report skips it when `REFERENCE_PHP` is not set. |

## Work item.31 Consolidated Matrix

`just jit-cranelift-report` first runs the off-vs-Cranelift differential gate
and then writes a complete Big-Win overview to
`target/performance/cranelift/big_wins_report.json`. The report remains valid when
optional rows, such as `REFERENCE_PHP`, are skipped.

The schema-2 matrix summary records these required families:

| Family | Local scenario count | Observed path kinds |
| --- | ---: | --- |
| `int_leaf_calls` | 5 | `interpreter_baseline`, `jit_fast_path`, `jit_side_exit_resume` |
| `int_counted_loop` | 7 | `interpreter_baseline`, `jit_fast_path`, `jit_fallback_or_skip` |
| `packed_int_fetch` | 1 | `interpreter_baseline`, `jit_helper_call_path`, `jit_fast_path` |
| `packed_foreach_sum` | 3 | `interpreter_baseline`, `jit_fast_path`, `jit_side_exit_resume` |
| `known_strlen_count_call` | 5 | `interpreter_baseline`, `jit_helper_call_path`, `jit_fast_path`, `jit_side_exit_resume`, `jit_fallback_or_skip` |
| `string_concat` | 6 | `interpreter_baseline`, `jit_helper_call_path`, `jit_fast_path`, `jit_fallback_or_skip` |
| `property_read_loop` | 5 | `interpreter_baseline`, `jit_helper_call_path`, `jit_fast_path`, `jit_side_exit_resume`, `jit_fallback_or_skip` |
| `method_call_loop` | 1 | `interpreter_baseline`, `jit_fast_path`, `jit_fallback_or_skip` |

Coverage is enforced by `scripts/performance/cranelift/jit_bench_matrix.py`: each
required family must have an interpreter baseline and a Cranelift row, and the
overall report must include interpreter baseline, helper-call, and fast-path
coverage. `REFERENCE_PHP` rows are advisory only and never decide the gate.

## Work item.32 Guard Report

`just cranelift-guard-report` consumes the generated Big-Win report JSON and
writes:

```text
target/performance/cranelift/guard-report.json
target/performance/cranelift/guard-report.txt
```

The text report is intended for humans; the JSON report is stable enough for
follow-up scripts. It reports:

- top side-exit reasons;
- rows with the highest observed failure rates;
- blacklisted candidates and blacklist reasons;
- recommended next actions: `keep`, `specialize`, `blacklist`, or
  `unsupported`;
- optional minimizer commands that write only under
  `target/performance/cranelift/minimized/`.

Local Work item.32 output from the current Big-Win report found
`helper_status`, `overflow`, and `guard_failed` side-exit reasons. Expected
fallback or known-gap rows are classified as `unsupported` instead of being
treated as failed correctness.

## Work item.35 Decision Matrix

Work item.35 uses the current schema-2 Big-Win report
(`target/performance/cranelift/big_wins_report.json`) plus the guard report
(`target/performance/cranelift/guard-report.json`) as the decision source. The table
below intentionally separates correctness from speed. All speed indications are
local, advisory wall-clock observations from the generated report; they are not
portable benchmark claims and must not become CI gates.

| Feature | Correctness status | Speed indication | Compile overhead | Side-exit rate | Safety status | Default status | Recommendation |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Integer leaf calls and checked integer arithmetic | Correct for the narrow typed int subset; overflow resumes through side exit. Guard report: 4 keep, 1 blacklist. | Mixed/parity in local rows, median about 1.05x interpreter execution time and one overflow correctness row slower by design. | ~0.48-0.71 ms for compiled rows. | 1 side-exit row out of 5 Cranelift rows. | Mitigated by checked ops, ABI hash checks, side-exit fallback, and Work item.33 safety audit. | `jit-cranelift` feature and runtime mode remain off by default. | Keep experimental; expand only where additional hot rows prove benefit. Overflow stress rows stay blacklisted/side-exit tests, not a win. |
| Counted and branchy int loops | Correct for simple int loops and branches; loop-body calls fall back with `CL-GAP-LOOP-BODY-CALL`. Guard report: 6 keep, 1 unsupported. | Local rows are around parity to slightly slower; no clear big win yet. | 0 for unsupported fallback row, up to ~1.00 ms for compiled rows. | 0 side exits in current loop rows; unsupported loop body call falls back at compile/eligibility time. | Mitigated by conservative eligibility and interpreter fallback for dynamic loop bodies. | Off by default. | Keep experimental; revisit after future runtime optimizer and richer loop metadata. Do not market as a current win. |
| Packed int indexed fetch | Correct for the read-only packed int-index shape. Guard report: 1 keep. | Around parity in the local row; useful mainly as ABI proof. | ~0.50 ms. | 0 side exits in the valid row; invalid shapes remain fixture-covered elsewhere. | Mitigated by helper-owned layout/bounds checks and VM fallback. | Off by default. | Keep experimental as a building block for array loops; expand only with stronger array hot-path evidence. |
| Packed foreach integer sum | Correct for the all-int read-only reduction; mixed elements and overflow side-exit. Eligibility consumes runtime-owned packed metadata, including element summary and reference/layout state, instead of duplicating array-layout assumptions in the native tier. Guard report: 1 keep, 1 specialize, 1 blacklist. | Only clear local big win: all-int large-row execution is materially faster; side-exit rows are not wins. | ~1.05-1.43 ms. | 2 side-exit rows out of 3 Cranelift rows. | Mitigated by element/layout/overflow guards, helper status checks, side-exit fallback, and safety audit. | Off by default. | Keep and expand the all-int packed reduction subset. Specialize mixed-layout handling later; keep overflow as blacklist/guard evidence. |
| Known `strlen`/`count` calls | Correct for exact one-argument global `strlen`/`count`; wrong arity and non-string guard paths remain fallback/side-exit. Guard report: 3 keep, 1 specialize, 1 unsupported. | Mixed; local median around parity, with small rows dominated by helper overhead. | 0 for unsupported fallback row, up to ~0.59 ms for compiled rows. | 1 side-exit row out of 5 Cranelift rows. | Mitigated by VM-owned helpers, arity/type guards, and interpreter fallback. | Off by default. | Keep experimental. Expand only after call metadata can distinguish profitable hot callsites. |
| String/string concatenation | Correct for typed string/string concat; string/int conversion and object `__toString` remain unsupported fallback rows. Guard report: 4 keep, 2 unsupported. | Mixed; one local row shows a small win, most are parity/slower due to helper/allocation overhead. | 0 for unsupported fallback rows, up to ~0.44 ms for compiled rows. | 0 side exits; unsupported conversion-sensitive shapes fall back. | Mitigated by helper-owned allocation/string handling and conservative shape filtering. | Off by default. | Keep experimental for exact typed strings; revisit conversion-sensitive strings after future runtime. |
| Monomorphic property load | Correct for simple DTO reads; wrong class and uninitialized typed properties side-exit; hook/magic fallback stays unsupported. Guard report: 2 keep, 2 specialize, 1 unsupported. | Mostly parity/slower locally, with only small row-level signs of benefit. | 0 for unsupported fallback row, up to ~0.43 ms for compiled rows. | 2 side-exit rows out of 5 Cranelift rows. | Mitigated by class/layout/uninitialized guards and VM-owned helper path. | Off by default. | Keep experimental; specialize guard failures and require stronger object hot-path evidence before expansion. |
| Monomorphic method-call dispatch helper | Correct for the current service/DTO loop fixture. Guard report: 1 keep. | Around parity/slightly slower locally; this is a dispatch-helper proof, not an inliner win. | Current cached row reports 0 additional compile time in the generated matrix. | 0 side exits in the matrix row. | Mitigated by method/class epoch metadata and generic fallback for misses. | Off by default. | Revisit after future runtime. Keep as metadata/dispatch groundwork; do not expand as a standalone win until inlining or cheaper dispatch exists. |
| Tiering and process-local compile cache | Correctness support features, not user-visible PHP semantics. Guard report: hot threshold keep; cold threshold unsupported because no native compile is expected. | Intended to reduce compile overhead and avoid cold compiles; current rows are parity/noise. | Cold row 0, hot row ~0.45 ms in current matrix. | 0 side exits. | Mitigated by default thresholding, budget limits, ABI/config/ISA cache keys, epoch validation, and non-persistent cache lifetime. | Conservative threshold mode by default; Cranelift remains off by default. | Keep and expand as control-plane support. These are prerequisites for any future default consideration. |

Classification summary:

- Keep and expand: all-int packed foreach reduction, tiering policy, and
  process-local compile-cache controls.
- Keep experimental: integer leaf calls, simple counted loops, packed int fetch,
  known `strlen`/`count`, typed string/string concat, and monomorphic property
  loads.
- Disable by default: every Cranelift native path remains behind the
  non-default `jit-cranelift` feature and explicit runtime mode.
- Remove: no implemented path should be removed now; rows classified
  `unsupported` stay valuable as guardrails and fixtures.
- Revisit after future runtime: method-call dispatch, conversion-sensitive string
  concat, dynamic loop bodies, broader property/object shapes, and any claim of
  default-on runtime benefit.

The most important risk is measurement uncertainty: most local rows are small,
helper-heavy, and close to parity. The only current path that looks like a real
performance win is the narrow all-int packed foreach reduction. Everything else
is either correctness infrastructure, metadata groundwork, or an experimental
fast path that still needs stronger hot-workload evidence.

## Current Matrix

| Scenario | JIT mode | Iterations | Compile ns | Compile s | Execution s | Total s | Side exits | JIT status | Executed regions | Helper calls | Fast-path hits | Packed foreach hits | Packed foreach layout exits | Packed foreach overflow exits | Overflow exits | Slow-path calls | Code bytes | Known gaps |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `repeated_int_function_calls` | `off` | 3 | 0 | 0.000000 | 0.008889 | 0.008889 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `repeated_int_function_calls` | `cranelift` | 3 | 1152625 | 0.001153 | 0.007528 | 0.008680 | 0 | `executed` | 5 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 36 |  |
| `arithmetic_expression_chain` | `off` | 3 | 0 | 0.000000 | 0.006641 | 0.006641 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `arithmetic_expression_chain` | `cranelift` | 3 | 2097125 | 0.002097 | 0.006998 | 0.009096 | 0 | `executed` | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 104 |  |
| `negative_ints` | `off` | 3 | 0 | 0.000000 | 0.005909 | 0.005909 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `negative_ints` | `cranelift` | 3 | 1377417 | 0.001377 | 0.006589 | 0.007967 | 0 | `executed` | 1 | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 68 |  |
| `boundary_ints` | `off` | 3 | 0 | 0.000000 | 0.006446 | 0.006446 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `boundary_ints` | `cranelift` | 3 | 852167 | 0.000852 | 0.006392 | 0.007244 | 0 | `executed` | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 36 |  |
| `counted_loop_accumulator` | `off` | 3 | 0 | 0.000000 | 0.006027 | 0.006027 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `counted_loop_accumulator` | `cranelift` | 3 | 1397542 | 0.001398 | 0.005603 | 0.007001 | 0 | `executed` | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 80 |  |
| `factorial_like_loop` | `off` | 3 | 0 | 0.000000 | 0.005540 | 0.005540 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `factorial_like_loop` | `cranelift` | 3 | 1286375 | 0.001286 | 0.005625 | 0.006911 | 0 | `executed` | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 96 |  |
| `sum_to_n` | `off` | 3 | 0 | 0.000000 | 0.005427 | 0.005427 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `sum_to_n` | `cranelift` | 3 | 1503708 | 0.001504 | 0.006529 | 0.008032 | 0 | `executed` | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 80 |  |
| `fib_iterative` | `off` | 3 | 0 | 0.000000 | 0.007361 | 0.007361 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `fib_iterative` | `cranelift` | 3 | 1705833 | 0.001706 | 0.007614 | 0.009320 | 0 | `executed` | 1 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 96 |  |
| `branchy_int_loop` | `off` | 3 | 0 | 0.000000 | 0.006694 | 0.006694 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `branchy_int_loop` | `cranelift` | 3 | 1629500 | 0.001630 | 0.006389 | 0.008018 | 0 | `executed` | 1 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 112 |  |
| `branchy_max_min` | `off` | 3 | 0 | 0.000000 | 0.005632 | 0.005632 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `branchy_max_min` | `cranelift` | 3 | 1105292 | 0.001105 | 0.006086 | 0.007191 | 0 | `executed` | 2 | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 56 |  |
| `non_eligible_loop_call` | `off` | 3 | 0 | 0.000000 | 0.005577 | 0.005577 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `non_eligible_loop_call` | `cranelift` | 3 | 0 | 0.000000 | 0.005859 | 0.005859 | 0 | `fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | `CL-GAP-LOOP-BODY-CALL` |
| `packed_array_int_fetch` | `off` | 3 | 0 | 0.000000 | 0.005267 | 0.005267 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `packed_array_int_fetch` | `cranelift` | 3 | 885500 | 0.000886 | 0.005280 | 0.006165 | 0 | `executed` | 1 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 60 |  |
| `packed_foreach_int_sum` | `off` | 3 | 0 | 0.000000 | 0.216691 | 0.216691 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `packed_foreach_int_sum` | `cranelift` | 3 | 2193209 | 0.002193 | 0.041695 | 0.043888 | 0 | `executed` | 64 | 0 | 64 | 64 | 0 | 0 | 0 | 0 | 280 |  |
| `packed_foreach_mixed_element` | `off` | 3 | 0 | 0.000000 | 0.005958 | 0.005958 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `packed_foreach_mixed_element` | `cranelift` | 3 | 1735584 | 0.001736 | 0.005612 | 0.007348 | 1 | `side_exit` | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 | 280 |  |
| `packed_foreach_overflow` | `off` | 3 | 0 | 0.000000 | 0.005556 | 0.005556 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `packed_foreach_overflow` | `cranelift` | 3 | 1821417 | 0.001821 | 0.005931 | 0.007752 | 1 | `side_exit` | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 1 | 280 |  |
| `overflow_correctness` | `off` | 3 | 0 | 0.000000 | 0.005358 | 0.005358 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |  |
| `overflow_correctness` | `cranelift` | 3 | 892333 | 0.000892 | 0.005545 | 0.006437 | 1 | `side_exit` | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 40 |  |

## Work item.23 Known-Call Rows

The Work item.23 rows cover the conservative helper-backed fast path for
exact global `strlen($s)` and `count($a)` calls. The full JSON report also
records all standard JIT counters; this focused table shows the known-call
counters required by the work item.

| Scenario | JIT mode | Iterations | Compile ns | Compile s | Execution s | Total s | Side exits | JIT status | Executed regions | Helper calls | Fast-path hits | Known-call fast hits | Known-call guard exits | Known-call slow calls | Slow-path calls | Code bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `known_strlen_valid` | `off` | 3 | 0 | 0.000000 | 0.004308 | 0.004308 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `known_strlen_valid` | `cranelift` | 3 | 574292 | 0.000574 | 0.004666 | 0.005240 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 40 |
| `known_strlen_non_string` | `off` | 3 | 0 | 0.000000 | 0.004942 | 0.004942 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `known_strlen_non_string` | `cranelift` | 3 | 580834 | 0.000581 | 0.004980 | 0.005561 | 1 | `side_exit` | 0 | 0 | 0 | 0 | 1 | 1 | 1 | 40 |
| `known_count_packed` | `off` | 3 | 0 | 0.000000 | 0.004551 | 0.004551 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `known_count_packed` | `cranelift` | 3 | 558208 | 0.000558 | 0.004832 | 0.005390 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 40 |
| `known_count_mixed` | `off` | 3 | 0 | 0.000000 | 0.004441 | 0.004441 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `known_count_mixed` | `cranelift` | 3 | 574041 | 0.000574 | 0.004780 | 0.005354 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 0 | 40 |
| `known_strlen_wrong_arity` | `off` | 3 | 0 | 0.000000 | 0.005424 | 0.005424 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `known_strlen_wrong_arity` | `cranelift` | 3 | 0 | 0.000000 | 0.005195 | 0.005195 | 0 | `fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Work item.24 String-Concat Rows

The Work item.24 rows cover the conservative helper-backed fast path for
typed string/string concat. Conversion-sensitive string/int and object
`__toString` rows remain fallback rows with zero string-concat fast counters.

| Scenario | JIT mode | Iterations | Compile ns | Compile s | Execution s | Total s | Side exits | JIT status | Executed regions | Helper calls | Fast-path hits | String-concat fast hits | String-concat misses | Slow-path calls | Code bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `string_concat_two_strings` | `off` | 3 | 0 | 0.000000 | 0.005211 | 0.005211 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_two_strings` | `cranelift` | 3 | 562417 | 0.000562 | 0.004805 | 0.005367 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 40 |
| `string_concat_empty_strings` | `off` | 3 | 0 | 0.000000 | 0.005028 | 0.005028 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_empty_strings` | `cranelift` | 3 | 645750 | 0.000646 | 0.005021 | 0.005667 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 40 |
| `string_concat_large_strings` | `off` | 3 | 0 | 0.000000 | 0.004475 | 0.004475 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_large_strings` | `cranelift` | 3 | 541958 | 0.000542 | 0.004530 | 0.005072 | 0 | `executed` | 1 | 1 | 1 | 1 | 0 | 0 | 40 |
| `string_concat_template_loop` | `off` | 3 | 0 | 0.000000 | 0.005100 | 0.005100 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_template_loop` | `cranelift` | 3 | 546375 | 0.000546 | 0.005148 | 0.005695 | 0 | `executed` | 64 | 64 | 64 | 64 | 0 | 0 | 40 |
| `string_concat_string_int` | `off` | 3 | 0 | 0.000000 | 0.004144 | 0.004144 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_string_int` | `cranelift` | 3 | 0 | 0.000000 | 0.004243 | 0.004243 | 0 | `fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_object_to_string` | `off` | 3 | 0 | 0.000000 | 0.004891 | 0.004891 | 0 | `skipped` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `string_concat_object_to_string` | `cranelift` | 3 | 0 | 0.000000 | 0.005440 | 0.005440 | 0 | `fallback` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## Counter Notes

- `repeated_int_function_calls` executes 5 native regions and records 5 inline
  integer fast-path hits in the Cranelift row.
- `arithmetic_expression_chain` executes 1 native region and records 3 inline
  integer fast-path hits in the Cranelift row.
- `negative_ints` and `boundary_ints` cover signed negative values and
  non-overflowing `i64` boundary arithmetic through the inline checked path.
- `counted_loop_accumulator`, `factorial_like_loop`, and `sum_to_n` execute
  through the 07.CL.18 simple counted-loop native path with three fast-path
  hits each and zero helper calls.
- `fib_iterative` and `branchy_int_loop` are the additional 07.CL.19 numeric
  loop rows. They compare `--jit=off` with `--jit=cranelift` and report compile
  time, execution time, total time, side exits, and code bytes separately.
- `branchy_max_min` executes the 07.CL.18 branch/compare path for both calls,
  recording four fast-path hits and zero helper calls.
- `non_eligible_loop_call` is the negative 07.CL.18 row. It falls back with
  `CL-GAP-LOOP-BODY-CALL` because calls in loop bodies remain outside the
  native loop subset.
- `packed_array_int_fetch` is the 07.CL.21 helper-assisted row. It is expected
  to execute natively for the valid packed int-index fixture, record exactly
  one helper call, and increment `packed_fetch_fast_hits` without incrementing
  packed-fetch bounds or layout exits.
- `packed_foreach_int_sum` is the 07.CL.22 native loop row. It executes the
  large packed integer reduction natively, records `packed_foreach_sum_fast_hits`
  and regular `fast_path_hits`, and reports compile, execution, total time, and
  code bytes separately.
- `packed_foreach_mixed_element` compiles the same foreach-sum shape, exits on
  the element/layout guard with `side_exit_reasons.helper_status`,
  increments `packed_foreach_sum_layout_exits`, and resumes through the
  interpreter.
- `packed_foreach_overflow` compiles the foreach-sum shape, exits on checked
  integer addition overflow with `side_exit_reasons.overflow`, increments both
  `packed_foreach_sum_overflow_exits` and `overflow_exits`, and resumes through
  the interpreter.
- `known_strlen_valid`, `known_count_packed`, and `known_count_mixed` compile
  the exact global one-argument known-call shapes. Each Cranelift row records
  one helper-backed fast hit, one helper call, and no guard exits or slow calls.
- `known_strlen_non_string` compiles the known-call shape, fails the helper's
  string guard, increments `known_call_guard_exits` and
  `known_call_slow_calls`, and resumes through the interpreter so TypeError
  behavior remains unchanged.
- `known_strlen_wrong_arity` remains outside the known-call fast path and
  falls back without incrementing known-call counters.
- `string_concat_two_strings`, `string_concat_empty_strings`,
  `string_concat_large_strings`, and `string_concat_template_loop` compile the
  exact typed string/string concat shape. Each Cranelift row records helper
  calls, fast hits, string-concat fast hits, and zero string-concat misses.
- `string_concat_string_int` and `string_concat_object_to_string` remain
  outside the string-concat fast path, preserve generic concat conversion
  semantics, and fall back without incrementing string-concat counters.
- Work item.25 is metadata-only and therefore does not add Cranelift
  benchmark rows. It adds `property_fetch_profiles` to VM counter JSON and
  validates simple declared, subclass layout, magic `__get`, property hook,
  dynamic property fallback, polymorphic/megamorphic receiver classification,
  and uninitialized typed-property reasons through focused VM tests. The
  Cranelift diff fixture corpus now includes
  `tests/fixtures/performance/cranelift/property-load-metadata/`.
- Work item.26 adds helper-assisted monomorphic property-load benchmark
  rows. `property_load_simple_dto` records one property fast hit,
  `property_load_dto_loop` records 64 property fast hits,
  `property_load_wrong_class_side_exit` records one fast hit plus one guard
  exit, `property_load_hook_magic_fallback` remains fallback with zero property
  counters, and `property_load_uninitialized_error_path` records one guard exit
  plus one uninitialized exit. The Cranelift diff fixture corpus now includes
  `tests/fixtures/performance/cranelift/property-load/` and compared 58 fixtures
  locally.
- Work item.27 is metadata-only and therefore does not add direct native
  method-call benchmark rows. It adds `method_call_profiles` to VM counter JSON
  and validates final method, normal monomorphic method, subclass override,
  magic `__call` fallback, by-ref non-eligibility metadata, and stable JSON
  reporting through focused VM/counter tests. The Cranelift diff fixture corpus
  now includes `tests/fixtures/performance/cranelift/method-call-metadata/` and
  compared 63 fixtures locally.
- Work item.28 adds the first direct monomorphic method-call dispatch-helper
  path for Cranelift mode. It adds `direct_call_hits` and
  `direct_call_fallbacks` to counter/report JSON, covers repeated small method
  calls, subclass fallback, magic fallback, and exception propagation under
  `tests/fixtures/performance/cranelift/method-call/`, and adds the
  `method_call_service_dto_loop` benchmark row. The path is intentionally not
  an inliner and does not add a standalone native method-call ABI. The
  Cranelift diff fixture corpus now includes the method-call fixtures and
  compared 68 fixtures locally; the smoke benchmark wrote 66 rows locally, with
  the Cranelift `method_call_service_dto_loop` row recording 23 direct-call
  hits and one direct-call fallback.
- Work item.29 adds the conservative Cranelift tiering policy in
  `docs/adr/0786-cranelift-tiering-policy.md`. The default threshold no longer
  compiles on the first call, `--jit-eager` is used by correctness fixtures
  that intentionally force native execution, and `--jit-max-compile-us` plus
  `--jit-max-functions` reject compilation without changing PHP-visible
  behavior. Counter/report JSON now includes tiering cold/hot/eager decisions
  plus blacklist and budget rejections. The benchmark matrix includes
  `tiering_cold_threshold` and `tiering_hot_threshold` rows to prove that a
  cold threshold fixture stays interpreted while a hot threshold fixture
  compiles.
- Work item.30 adds process-local, non-persistent compiled-function handle
  reuse. Cache keys include function id, IR fingerprint, ABI hash, JIT config,
  and target ISA; entries validate runtime/class layout epoch before reuse.
  Counter/report JSON now includes compile-cache hits, misses, and
  invalidations.
- Work item.31 consolidates the Big-Win matrix into schema version 2. The
  generated JSON groups rows by required scenario family, records explicit
  `path_kinds` for interpreter baseline, helper-call, fast-path, fallback, and
  side-exit rows, and treats `REFERENCE_PHP` as optional orientation instead of
  a correctness gate.
- Work item.32 adds `scripts/performance/cranelift/guard_failure_report.py`, which
  analyzes the schema-2 Big-Win report, emits JSON and text guard reports, and
  recommends whether each row should be kept, specialized, blacklisted, or
  treated as unsupported.
- `overflow_correctness` compiles, detects overflow in native code, records an
  `overflow` side exit plus one slow-path call, and resumes through the
  interpreter.
- `iai-callgrind` is not a workspace dependency. The existing Performance
  Valgrind/Callgrind gate remains separate and skip-safe, so this Cranelift
  matrix uses JSON counters and advisory wall-clock totals without CI-hard
  speedup thresholds or platform-specific assumptions.

## Validation

The report command runs the off-vs-Cranelift correctness diff before producing
benchmark rows:

```bash
nix develop -c just jit-cranelift-diff
nix develop -c just jit-cranelift-bench-smoke
nix develop -c just jit-cranelift-report
nix develop -c just benchmark-smoke
```
