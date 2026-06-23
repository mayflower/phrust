# Phase 7 Optimized-Path Regression Fixtures

Prompt 07.44 adds `tests/fixtures/phase7/regressions/` to stress optimized
paths under control-flow conditions that can expose duplicated side effects,
lost exceptions, stale caches, or invalid by-reference assumptions. The
`regression-phase7` gate runs each fixture through `--opt-level=0`, `1`, and
`2`, with quickening off/on and inline caches off/on.

Prompt 07.56 extends `regression-phase7` with `perf-flag-matrix`, which
compares the explicit baseline (`--opt-level=0`, quickening off, inline caches
off, bytecode cache off, JIT off) against opt 1, opt 2, quickening, inline
caches, bytecode-cache read/write, and all-non-JIT-on variants. The matrix also
adds selected Phase 4-6 runtime fixtures so performance flags are checked
outside the Phase 7-only stress set. JIT is only included when requested with
`PHRUST_PHASE7_MATRIX_JIT=1` or `--include-jit` on a supported feature/platform
run.

| Fixture | Coverage |
| --- | --- |
| `exception-optimized-call.php` | Hot internal-call dispatch while a `ValueError` is thrown and caught inside the loop. |
| `destructor-near-temporary.php` | Destructor output ordering next to optimized integer arithmetic and temporary lifetime boundaries. |
| `generator-yield-around-packed-dim.php` | Generator suspension around packed-array integer fetch and integer add quickening. |
| `generator-yield-around-concat.php` | Generator suspension around repeated string concat quickening. |
| `fiber-suspend-around-arithmetic.php` | Fiber suspend/resume through a hot arithmetic loop. |
| `fiber-suspend-around-ic.php` | Fiber suspend/resume through method and property inline-cache sites. |
| `polymorphic-method-property-ic.php` | Fixed-size method/property polymorphic inline-cache guard list with megamorphic overflow fallback. |
| `byref-array-aliasing.php` | Array element by-reference aliasing around packed-array read/write fast paths. |
| `byref-property-object-aliasing.php` | Object-variable by-reference aliasing around method/property fast paths without relying on unsupported property references. |
| `autoload-invalidation-method-property.php` | Autoload class-table mutation at a shared method/property cache site. |

Expected outputs are checked exactly after CRLF normalization. Stderr is
compared across all optimization combinations for each fixture so the harness
does not filter real behavioral differences.

`perf-flag-matrix` writes per-run stdout, stderr, status files, and
`summary.json` under `target/phase7/perf-flag-matrix/`. On any behavior change
it prints a unified diff for stdout or stderr and exits nonzero.

`just polymorphic-inline-cache-smoke` additionally runs the polymorphic fixture
with inline caches off/on, compares output exactly, and asserts that VM counters
show polymorphic hits plus megamorphic fallback.
