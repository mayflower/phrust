# Phase 7 Benchmark Corpus

The Phase 7 benchmark corpus is a small deterministic smoke set under
`tests/fixtures/phase7/perf_smoke`. It is intentionally not a real-world
benchmark suite. The fixtures are designed to exercise common hot paths while
remaining stable under PHP 8.5.7 and the Rust VM baseline.

Each executable fixture has a sibling `.out` file with expected stdout. Fixtures
must not depend on networking, wall-clock time, random data, host paths, locale,
or platform-specific filesystem behavior.

## Fixtures

| Test ID | Fixture | Expected output | Hot path |
| --- | --- | --- | --- |
| `PHASE7-PERF-SMOKE-ARITHMETIC` | `arithmetic.php` | `arithmetic.php.out` | Integer arithmetic, loop-carried locals, add/sub/mul dispatch. |
| `PHASE7-PERF-SMOKE-LOOPS` | `loops.php` | `loops.php.out` | Nested `for`/`while` control flow and branch dispatch. |
| `PHASE7-PERF-SMOKE-ARRAYS-PACKED` | `arrays_packed.php` | `arrays_packed.php.out` | Packed append, foreach value iteration, `count()`. |
| `PHASE7-PERF-SMOKE-ARRAYS-MIXED` | `arrays_mixed.php` | `arrays_mixed.php.out` | String-key array reads/writes and string-key overwrite. |
| `PHASE7-PERF-SMOKE-FUNCTION-CALLS` | `function_calls.php` | `function_calls.php.out` | User function lookup, frame setup, return value flow. |
| `PHASE7-PERF-SMOKE-METHOD-CALLS` | `method_calls.php` | `method_calls.php.out` | Object construction, method dispatch, typed property read/write. |
| `PHASE7-PERF-SMOKE-PROPERTIES` | `properties.php` | `properties.php.out` | Public property read/write and integer update loop. |
| `PHASE7-PERF-SMOKE-STRINGS-CONCAT` | `strings_concat.php` | `strings_concat.php.out` | String concatenation and `strlen()` dispatch. |
| `PHASE7-PERF-SMOKE-OUTPUT-WRITES` | `output_writes.php` | `output_writes.php.out` | Multi-argument `echo`, scalar conversion, print-visible output bytes, and output-buffer flush. |
| `PHASE7-PERF-SMOKE-EXCEPTIONS-NO-THROW` | `exceptions_no_throw.php` | `exceptions_no_throw.php.out` | Try/catch region setup without throwing in the hot path. |
| `PHASE7-PERF-SMOKE-AUTOLOAD` | `autoload_smoke.php` | `autoload_smoke.php.out` | SPL autoload registration, include/require, class lookup. |

`_support/Phase7AutoloadSmoke.php` is support code for
`PHASE7-PERF-SMOKE-AUTOLOAD` and is not a standalone benchmark fixture.

## Current Execution Policy

The corpus is executed by `scripts/phase7/bench_matrix.py` through the just
recipes below. The standard smoke builds `php-vm`, runs the Rust VM over every
top-level fixture, records counters out-of-band, compares stdout against each
`.out` file, and writes `target/phase7/bench-phase7-smoke.json`.

```bash
nix develop -c just bench-phase7-smoke
nix develop -c just hotpaths-phase7
nix develop -c just perf-baseline
nix develop -c just perf-compare
nix develop -c just perf-report
```

Reference PHP runs are included when `REFERENCE_PHP` points at an executable
binary or the pinned `third_party/php-src/sapi/cli/php` exists. If no reference
binary is available, the runner records a skip reason instead of failing the
Rust VM smoke.

```bash
REFERENCE_PHP=third_party/php-src/sapi/cli/php nix develop -c just bench-phase7-smoke
```

The default benchmark smoke intentionally does not enable optimizer,
quickening, inline-cache, bytecode-cache, or JIT flags. Flag behavior is covered
by `just perf-flag-matrix`, `just optimizer-diff`, `just quickening-smoke`,
`just inline-cache-smoke`, `just cache-roundtrip`, and `just jit-smoke`.

## Troubleshooting

- Flaky wall-clock timings: rerun `perf-baseline` and `perf-compare`, increase
  `PHRUST_PHASE7_BASELINE_REPETITIONS`, and treat wall-clock as advisory unless
  a stable CI/Callgrind budget exists.
- Output mismatches: inspect the generated stdout/stderr files under
  `target/phase7`, then run the fixture directly with `target/debug/php-vm run`.
- Missing reference PHP: either set `REFERENCE_PHP` or accept the recorded
  reference skip; Rust VM fixture correctness still runs.
- Missing optional profiling tools: `bench-phase7-callgrind-smoke` skips on
  Darwin or without Valgrind. That skip is expected for standard local gates.
