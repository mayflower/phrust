# Phase 7 Performance Principles

Phase 7 adds a correctness-preserving performance layer to the PHP 8.5.7 Rust
engine. It does not redefine PHP semantics, replace the existing frontend or VM,
or treat benchmark wins as a substitute for Phase 0 through Phase 6 regression
proof.

## Optimization Layers

### Measurement And Benchmark Infrastructure

The first layer establishes deterministic fixtures, machine-readable metrics,
environment capture, baseline generation, comparison tooling, counters, and
reports. Wall-clock data is advisory unless paired with stable methodology and
clear uncertainty.

### Bytecode And IR Cache

The cache stores verified compiled artifacts with fingerprints covering source,
engine version, target PHP version, format versions, options, and relevant
configuration. Corrupt or stale artifacts must fall back to compile-from-source.

### Optimizer Pass Framework

Optimizer passes run behind explicit opt levels. `--opt-level=0` is the semantic
baseline. Higher levels may add safe constant folding, peepholes, and CFG
simplification only when verifier and A/B tests prove no visible behavior
changes.

### Quickening

Quickening may rewrite or side-table op behavior after hotness evidence, but
specialized paths must preserve fallback to the generic interpreter path. Guard
misses, overflow, type changes, by-reference behavior, exceptions, and other PHP
edge cases must deopt or remain unspecialized.

### Inline Caches

Inline caches may cache resolution results for functions, methods, properties,
class constants, static properties, include paths, autoload lookups, and internal
dispatch only when guarded by class, function, method-table, property-shape,
autoload, include-path, and configuration epochs as appropriate.

### Runtime Fast Paths

Runtime fast paths may optimize local slots, frame reuse, arrays, numeric-string
classification, parameter prologues, internal functions, and output buffering.
They must not bypass references, COW, destructors, generators, fibers,
exceptions, visibility, magic methods, or diagnostics.

### Tiering Policy

Tiering is request-local and advisory. The current policy tracks function entry
count, loop backedge count, inline-cache stability score, and guard-failure
score. Tier 0 is the baseline interpreter, Tier 1 is the quickened interpreter,
and Tier 2 is the default-off experimental JIT when the `jit-cranelift` feature
and `--jit=on` are both enabled. `--tiering=off` disables adaptive quickening
observations and JIT attempts for the request. `--tiering-stats-json <path>`
writes stats outside PHP stdout.

### Experimental JIT

JIT work is default-off and feature-gated. The initial scope is a tiny safe
subset such as pure integer leaf functions. The interpreter remains the source
of truth, JIT eligibility must reject unsupported code, and fallback must be
available for every failure mode. See
`docs/adr/0076-cranelift-jit-experiment.md` and
`docs/jit-experiment-phase7.md` for the Phase 7 decision, scope, ABI boundary,
guard/deopt policy, code-cache lifecycle, platform limits, and abort criteria.

## Correctness Contract

- `--opt-level=0` is the baseline when optimization flags exist.
- `--quickening=off`, `--inline-caches=off`, `--bytecode-cache=off`, and
  `--jit=off` are required once the respective layers exist.
- `--tiering=off` must keep adaptive quickening and JIT tier decisions inactive.
- Optimized and baseline runs must match output, stderr, exit status,
  diagnostics, exception classes, warning text where modeled, and
  timing-independent side effects.
- Guard failure falls back to the generic path.
- Cache miss, cache corruption, stale fingerprint, unsupported IR, and
  unsupported JIT platform must degrade to safe baseline behavior.
- Any known deviation is documented in `docs/known-gaps-phase7.md`.

## Roadmap

- `07.00`: preflight and initial known-gap catalog.
- `07.01`: scope ADR and performance principles.
- `07.02` to `07.03`: Nix/tooling and Phase 7 justfile gates.
- `07.04` to `07.09`: metrics crate, benchmark corpus, runner, counters,
  baseline/compare tooling, and hot-path inventory.
- `07.10`: IR/bytecode verifier hardening.
- `07.11` to `07.16`: bytecode-cache design, crate, fingerprinting,
  roundtrip, CLI integration, and lifecycle documentation.
- `07.17` to `07.22`: optimizer framework, differential harness, constant
  folding, peepholes, CFG cleanup, and literal pool/string interning.
- `07.23` to `07.27`: quickening model, framework, and selected
  specializations for integer add, string concat, and packed array dim fetch.
- `07.28` to `07.35`: inline-cache design, slots, stats, and caches for
  functions, methods, properties, class constants, static properties, includes,
  and Composer/autoload lookup.
- `07.36` to `07.43`: runtime fast paths and a unified deopt/fallback protocol.
- `07.44` to `07.47`: stress regressions, optional callgrind smoke, Criterion
  hot-path benchmarks, and performance report generation.
- `07.48` to `07.55`: experimental Cranelift JIT ADR, crate, eligibility, ABI,
  lowering, execution smoke, tiering, and safety audit.
- `07.56` to `07.60`: A/B flag matrix, performance result consolidation,
  developer docs, CI/Nix hardening, final audit, and Phase 8 handoff.
- `07.A` to `07.F`: optional profiling workflow, optional LTO/PGO plan,
  shared-cache research, polymorphic inline caches, framework-like smokes, and
  optional W^X/mprotect JIT memory prototype.

## Reference Links

- `php-src/php-8.5.7/Zend/zend_vm_def.h`
- `php-src/php-8.5.7/ext/opcache/`
- `php-src/php-8.5.7/ext/opcache/jit/README.md`
- PEP 659: https://peps.python.org/pep-0659/
- Cranelift: https://cranelift.dev/
- Criterion.rs: https://bheisler.github.io/criterion.rs/book/
- iai-callgrind: https://docs.rs/iai-callgrind

## Validation Policy

Documentation-only prompts use the strongest available docs or smoke gate. If no
dedicated docs gate exists, the current fallback is:

```bash
nix develop -c just verify-phase6
```

Code-changing Phase 7 prompts must add or update a narrower Phase 7 gate and
then include that gate in `verify-phase7` once the gate exists.

## Command Surface

Prompt 07.03 introduced the Phase 7 recipes below. Later prompts replaced the
initial scaffolding with concrete cache, optimizer, quickening, inline-cache,
JIT, safety, matrix, and reporting gates.

| Command | Current behavior |
| --- | --- |
| `verify-phase7` | Runs `test-phase7`, `regression-phase7`, cache/optimizer/quickening/inline-cache/JIT/safety gates, benchmark smoke, hot-path inventory, and `perf-report`. |
| `test-phase7` | Runs `cargo test --workspace` plus Phase 7 script self-tests. |
| `regression-phase7` | Runs `scripts/phase6_regression_smoke.sh`, then `scripts/phase7/regression_smoke.sh` across opt levels 0/1/2, quickening off/on, and inline caches off/on for the Prompt 07.44 stress fixtures, followed by `perf-flag-matrix`. |
| `perf-flag-matrix` | Compares baseline output/exit/stderr against opt 1, opt 2, quickening, inline caches, bytecode-cache read/write, and all-non-JIT-on combinations across Phase 7 regressions and selected Phase 4-6 fixtures. JIT is opt-in with `PHRUST_PHASE7_MATRIX_JIT=1` when feature/platform support is available. |
| `bench-phase7-smoke` | Builds the VM, runs deterministic Phase 7 smoke fixtures, checks expected output, and writes `target/phase7/bench-phase7-smoke.json`. |
| `bench-phase7-callgrind-smoke` | Optional Callgrind smoke; skips cleanly outside Linux or without `valgrind`, otherwise writes `target/phase7/callgrind/summary.json`. |
| `bench-rust-phase7` | Runs Criterion benchmarks from the benchmark-only, workspace-excluded `php_bench` package for Rust hot paths. |
| `bench-phase7` | Runs the deterministic CLI benchmark matrix and then `bench-rust-phase7`. |
| `perf-baseline` | Builds the VM and writes a local host-specific baseline to `target/phase7/baseline.json`. |
| `perf-compare` | Compares `target/phase7/baseline.json` with a fresh benchmark smoke and writes `target/phase7/perf-compare.md` plus JSON. |
| `cache-roundtrip` | Runs fingerprint smoke coverage, bytecode-cache roundtrip/verifier/corrupt fallback tests, and CLI cache hit/miss/path-component tests. |
| `optimizer-diff` | Verifies IR invariants and compares opt levels 0, 1, and 2 across optimizer fixtures with output, exit, and diagnostic diffs. |
| `quickening-smoke` | Builds the VM, compares `--quickening=off` and `--quickening=on` across Phase 7 smoke fixtures, and asserts quickening counters. |
| `inline-cache-smoke` | Builds the VM, compares `--inline-caches=off` and `--inline-caches=on` across Phase 7 smoke fixtures, and asserts IC slots, function-call hits/misses, method-call hits/misses, property-fetch hits/misses, class/static hits/misses, and include/eval/autoload epoch invalidation counters. |
| `jit-smoke` | Runs default-off `php_jit` API, eligibility, ABI, optional Cranelift lowering tests, feature-on VM JIT tests, and a CLI A/B smoke comparing `--jit=off` and `--jit=on`; asserts compile/execution/fallback counters while keeping native machine-code execution disabled. |
| `phase7-safety-audit-smoke` | Scans the Phase 7 cache/JIT/adaptive runtime surface for Rust `unsafe`, runs bytecode-cache negative tests, and runs a small Miri cache test when the active toolchain supports it. |
| `perf-report` | Renders `target/phase7/perf-report.md` and JSON from benchmark measurements, VM counters, comparison artifact presence, and known gaps. |

Tiering flags available to `php-vm run`:

```bash
--tiering off|on
--tiering-function-threshold N
--tiering-loop-threshold N
--tiering-ic-stability-threshold N
--tiering-guard-failure-threshold N
--tiering-stats-json target/phase7/tiering.json
```
