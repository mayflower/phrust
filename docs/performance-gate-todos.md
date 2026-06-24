# Performance Gate TODOs

Work item introduces the Performance command surface before the underlying
performance layers exist. Placeholder gates are intentionally green, write a
small status file under `target/performance`, and point back to this file. Later
work items must replace each placeholder with real work.

| Gate | Placeholder reason | Replacement work item |
| --- | --- | --- |
| _none_ | All current Performance gates have concrete implementations or skip-safe smoke scripts. | _n/a_ |

`cache-roundtrip` was replaced in work item with a real metadata,
fingerprint, and verified IR payload roundtrip gate. `optimizer-diff` was
replaced in work item with a real no-op optimizer A/B smoke gate and expanded
in work item into a differential harness that compares stdout, normalized
stderr, exit code, structured diagnostics, and counter invariants across
`--opt-level=0`, `--opt-level=1`, and `--opt-level=2`. `quickening-smoke` was
replaced in work item with a real A/B gate for `--quickening=off` and
`--quickening=on`, including counter assertions for the request-local side
table. `verify-performance` is not a pure placeholder. It runs `performance-tests`,
`performance-regression`, the real bytecode-cache roundtrip gate, the real optimizer
diff gate, and the real quickening smoke gate. `inline-cache-smoke` was replaced
in work item with a real A/B gate for `--inline-caches=off` and
`--inline-caches=on`, including slot counter assertions for function, method,
property, and dim candidates. Work item extended it with a monomorphic
function-call resolution cache, hit/miss assertions, and an include-driven
invalidation fixture. Work item extended it with monomorphic method-call
resolution cache counters and VM tests for hot loops, inherited methods,
overrides, magic fallback, visibility, and trait aliases. Work item extended
it with monomorphic property-fetch counters and VM tests for public declared
properties, private scoped reads, inherited properties, receiver guard
failures, dynamic-property fallback, `__get` fallback, property-hook fallback,
and uninitialized typed-property errors. Work item extended it with
class-constant/static-property metadata counters and VM tests for repeated
class constants, inherited constants, enum cases, static-property read-after-
write behavior, late static binding guards, and include-driven lookup epoch
invalidation. Work item replaced `perf-report` with a real report generator
and kept `benchmark-suite` as the deterministic CLI benchmark suite plus Criterion
Rust hot-path benchmarks. Work item replaced `jit-smoke` with a default-off
`php_jit` API and feature-compilation smoke that skips native execution until
eligibility, ABI, fallback, and executable-memory safety are implemented.
