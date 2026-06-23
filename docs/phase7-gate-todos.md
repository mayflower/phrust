# Phase 7 Gate TODOs

Prompt 07.03 introduces the Phase 7 command surface before the underlying
performance layers exist. Placeholder gates are intentionally green, write a
small status file under `target/phase7`, and point back to this file. Later
prompts must replace each placeholder with real work.

| Gate | Placeholder reason | Replacement prompt |
| --- | --- | --- |
| _none_ | All current Phase 7 gates have concrete implementations or skip-safe smoke scripts. | _n/a_ |

`cache-roundtrip` was replaced in prompt 07.14 with a real metadata,
fingerprint, and verified IR payload roundtrip gate. `optimizer-diff` was
replaced in prompt 07.17 with a real no-op optimizer A/B smoke gate and expanded
in prompt 07.18 into a differential harness that compares stdout, normalized
stderr, exit code, structured diagnostics, and counter invariants across
`--opt-level=0`, `--opt-level=1`, and `--opt-level=2`. `quickening-smoke` was
replaced in prompt 07.24 with a real A/B gate for `--quickening=off` and
`--quickening=on`, including counter assertions for the request-local side
table. `verify-phase7` is not a pure placeholder. It runs `test-phase7`,
`regression-phase7`, the real bytecode-cache roundtrip gate, the real optimizer
diff gate, and the real quickening smoke gate. `inline-cache-smoke` was replaced
in prompt 07.29 with a real A/B gate for `--inline-caches=off` and
`--inline-caches=on`, including slot counter assertions for function, method,
property, and dim candidates. Prompt 07.30 extended it with a monomorphic
function-call resolution cache, hit/miss assertions, and an include-driven
invalidation fixture. Prompt 07.31 extended it with monomorphic method-call
resolution cache counters and VM tests for hot loops, inherited methods,
overrides, magic fallback, visibility, and trait aliases. Prompt 07.32 extended
it with monomorphic property-fetch counters and VM tests for public declared
properties, private scoped reads, inherited properties, receiver guard
failures, dynamic-property fallback, `__get` fallback, property-hook fallback,
and uninitialized typed-property errors. Prompt 07.33 extended it with
class-constant/static-property metadata counters and VM tests for repeated
class constants, inherited constants, enum cases, static-property read-after-
write behavior, late static binding guards, and include-driven lookup epoch
invalidation. Prompt 07.47 replaced `perf-report` with a real report generator
and kept `bench-phase7` as the deterministic CLI benchmark suite plus Criterion
Rust hot-path benchmarks. Prompt 07.49 replaced `jit-smoke` with a default-off
`php_jit` API and feature-compilation smoke that skips native execution until
eligibility, ABI, fallback, and executable-memory safety are implemented.
