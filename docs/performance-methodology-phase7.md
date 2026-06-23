# Phase 7 Performance Methodology

Phase 7 performance work is measured with reproducible local tools from the Nix
dev shell. Performance data is useful only when it is paired with correctness
checks and enough environment detail to reproduce the run.

## Dev Shell Tools

| Tool | Platform | Purpose | Required for standard gates |
| --- | --- | --- | --- |
| `cargo` / `rustc` / `rustfmt` / `clippy` | Linux through Nix; Darwin from the host | Rust build, test, format, and lint workflow | yes |
| `cargo-nextest` | Linux through Nix | Optional faster Rust test runner for later Phase 7 gates | no |
| `hyperfine` | Linux and Darwin through Nix | CLI wall-clock benchmark smokes and local before/after comparisons | no |
| `jq` | Linux and Darwin through Nix | JSON normalization, report inspection, and shell-script assertions | yes |
| `python3` | Linux and Darwin through Nix | Deterministic benchmark, diff, and report scripts using the standard library | yes |
| `sccache` | Linux and Darwin through Nix | Rust compilation cache through `RUSTC_WRAPPER` | no, but enabled when available |
| `ccache` | Linux and Darwin through Nix | C/C++ compilation cache for reference and native build helpers | no, but enabled when available |
| `mold` | Linux only | Faster linker for Linux Rust builds through `RUSTFLAGS` | no |
| `valgrind` | Linux only | Callgrind/Cachegrind instruction-count and cache smokes | no |
| `perf` / `linuxPackages.perf` | Linux only | Optional local CPU profiling and counter exploration | no |
| `gdb` | Linux only | Local runtime debugging | no |
| `shellcheck` | Linux through Nix; Darwin from the host when installed | Optional script linting for older verification helpers | no |
| native PHP/C build tools | Linux through Nix; Darwin from the host when needed | Optional reference PHP and native-library workflows | no |

Linux-only tools are added with Nix conditionals so Darwin shells and `nix
flake check` do not evaluate them as required Darwin packages. The default
Darwin shell uses `mkShellNoCC` plus a lightweight Nix tool surface for `just`,
`jq`, `hyperfine`, `ripgrep`, `fd`, `python3`, `ccache`, and `sccache`, while
keeping the existing host Rust toolchain stable. Darwin shells intentionally do
not pull `shellcheck` from Nix because that requires a large Haskell closure on
current nixpkgs; script linting remains an optional host-tool check there.

## Environment Normalization

Benchmark scripts added during Phase 7 should set or record:

- `TZ=UTC`
- `LC_ALL=C`
- deterministic temporary directories under `target/phase7`
- deterministic seeds for generated fixtures
- engine version, PHP target version, target triple, and relevant feature flags
- cache directories used by bytecode-cache or compiler tooling

Wall-clock measurements must be reported as advisory unless paired with stable
fixtures, repeated runs, and documented uncertainty.

## Correctness Before Measurement

Every measured optimized mode must have a comparable baseline mode. The baseline
is `--opt-level=0` plus disabled quickening, inline caches, bytecode cache, and
JIT once those flags exist. A performance result is not actionable if the A/B
correctness comparison fails.

## Validation Commands

Standard Phase 7 workflow:

```bash
nix develop -c just verify-phase7
nix develop -c just regression-phase7
nix develop -c just perf-flag-matrix
nix develop -c just bench-phase7-smoke
nix develop -c just perf-baseline
nix develop -c just perf-compare
nix develop -c just perf-report
```

Layer-specific gates:

```bash
nix develop -c just cache-roundtrip
nix develop -c just optimizer-diff
nix develop -c just quickening-smoke
nix develop -c just inline-cache-smoke
nix develop -c just jit-smoke
nix develop -c just phase7-safety-audit-smoke
```

Prompt 07.02 validates the shell surface with:

```bash
nix develop -c just --list
nix develop -c cargo --version
nix develop -c jq --version
nix develop -c hyperfine --version
```

Linux maintainers can additionally inspect optional tools with:

```bash
nix develop -c valgrind --version
nix develop -c perf --version
```

Those Linux commands are not required on Darwin.

## CI Policy

Phase 7 CI is documented in `docs/phase7-ci-policy.md`. Pull-request CI runs
the lightweight flake metadata check and the same required gate maintainers run
locally:

```bash
nix flake check
nix develop -c just verify-phase7
```

Long benchmark suites are manual or scheduled only:

```bash
nix develop -c just bench-phase7
nix develop -c just perf-report
```

The required CI path must not depend on secrets, network access from tests,
native JIT support, or optional profiling tools. JIT and profiling paths must
skip or fall back with explicit reasons when unsupported.

## Optional Callgrind Smoke

Prompt 07.45 adds `just bench-phase7-callgrind-smoke` for instruction-count
smoke measurements. The gate is intentionally optional:

- non-Linux hosts skip with a recorded reason in
  `target/phase7/callgrind/summary.json`;
- Linux hosts without `valgrind` in `PATH` also skip cleanly;
- Linux hosts with Valgrind run three small CLI scenarios from
  `tests/fixtures/phase7/perf_smoke/` under `--tool=callgrind`;
- outputs are still compared to fixture expectations before instruction counts
  are accepted;
- no strict instruction thresholds are enforced until a stable CI environment is
  dedicated to those counters.

The summary files are local artifacts under `target/phase7/callgrind/` and are
not committed.

## Criterion Rust Hot-Path Benchmarks

Prompt 07.46 adds a benchmark-only in-repository package, `php_bench`, with
Criterion as a dev-dependency. It is excluded from the main workspace so
`cargo test --workspace` and `verify-phase7` do not compile Criterion.
Engine/runtime crates do not depend on Criterion. The `just bench-rust-phase7`
recipe runs deterministic Rust-level hot-path cases:

- lexer plus parser smoke;
- semantic frontend to IR lowering;
- VM dispatch loop;
- user function call dispatch;
- property lookup through the VM;
- packed array access;
- mixed array access;
- PHP string buffer growth.

These benchmarks are local trend indicators. They are not compatibility
evidence by themselves and must be paired with `verify-phase7` or a narrower
correctness gate before optimization claims are accepted.

## Performance Report

Prompt 07.47 adds `just perf-report`, which renders
`target/phase7/perf-report.md` and `target/phase7/perf-report.json` from the
latest benchmark JSON, usually `target/phase7/bench-phase7-smoke.json`. The
report includes environment metadata, commit/version, optimization flags,
scenario status, counter hotspots, cache hit/miss counters, quickening counters,
inline-cache counters, and the current Phase 7 known-gap table.

The report is intentionally local and host-specific, so generated files remain
under `target/phase7`. It does not compare wall-clock timings by itself. Create
a local baseline with `nix develop -c just perf-baseline`, compare with
`nix develop -c just perf-compare`, and use the report as a readable index into
those artifacts.

If benchmark JSON is missing, `perf-report` still writes a readable report with
a missing-data section and the commands needed to create inputs.

## Failure Policy

- Missing required all-platform tools are a dev-shell failure.
- Missing Linux-only tools on Darwin are expected.
- Missing optional profiling tools should make optional profiling gates skip
  clearly, not fail the standard verification path.
- No shell hook may download unpinned binaries or mutate global tool state.

## Troubleshooting

- Flaky benchmarks: rerun with larger `PHRUST_PHASE7_BASELINE_REPETITIONS` or
  `PHRUST_PHASE7_BENCH_SMOKE_REPETITIONS`, compare trends rather than a single
  wall-clock sample, and keep `verify-phase7` as the correctness source.
- Missing Valgrind or `perf`: `bench-phase7-callgrind-smoke` skips on Darwin or
  without Valgrind. Linux maintainers can install/use those tools through the
  Nix shell and rerun the optional gate.
- Unsupported JIT platform: keep `--jit=off` for standard runs. Use
  `jit-smoke` for the supported default-off and feature-on proof path; do not
  claim production JIT readiness.
- Cache invalidation failures: run `cache-roundtrip`, use
  `--bytecode-cache-stats`, clear the local cache directory, and inspect
  fingerprint dimensions before changing cache semantics.
- Output diffs: inspect the per-gate artifacts under `target/phase7`, rerun the
  single fixture with `target/debug/php-vm run`, and compare against the
  baseline flags from `perf-flag-matrix`.

## Performance JSON Format

Prompt 07.04 adds `crates/php_perf`, a data-model crate for normalized Phase 7
performance JSON. The crate does not execute benchmarks. It defines:

- `PerfRunId`: stable run identifier.
- `PerfEnvironment`: engine version, optional git commit, Rust target triple,
  optimization flags, feature flags, and normalized environment fields.
- `PerfScenario`: stable scenario id, human name, group, and optional fixture.
- `PerfMetric`: named numeric metric with unit and directionality.
- `PerfMeasurement`: scenario, iterations, metrics, optional wall time, and
  optional VM counters.
- `PerfReport`: schema version, run id, environment, and measurements.

Reports use serde JSON with normalized pretty output and a trailing newline via
`PerfReport::to_stable_json()`. Maps use sorted `BTreeMap` storage so feature
flags, extra environment fields, and VM counters are deterministic.

Minimal report shape:

```json
{
  "schema_version": 1,
  "run_id": "phase7-test-run",
  "environment": {
    "engine_version": "phrust-0.0.0",
    "rust_target_triple": "aarch64-apple-darwin",
    "opt_flags": ["--opt-level=0"],
    "feature_flags": {}
  },
  "measurements": []
}
```

Validation:

```bash
nix develop -c cargo test -p php_perf
```

## Benchmark Runner

Prompt 07.06 adds `scripts/phase7/bench_matrix.py`. The runner discovers only
top-level `*.php` files in `tests/fixtures/phase7/perf_smoke`, reads the matching
`*.php.out` expected output, and invokes engines with argument vectors rather
than interpolated shell commands. Fixture names are therefore not shell input.

The runner normalizes each process environment with:

- `TZ=UTC`
- `LC_ALL=C`
- `LANG=C`
- `TMPDIR`, `TMP`, and `TEMP` under `target/phase7/tmp`
- deterministic seed environment variables for future generated fixtures

`just bench-phase7-smoke` builds `php-vm`, runs the Rust engine over the smoke
corpus, and writes `target/phase7/bench-phase7-smoke.json`. If `REFERENCE_PHP`
is set, or the pinned `third_party/php-src/sapi/cli/php` binary exists, the same
fixtures are also run as a separate `reference-php` engine in the same report.
Reference PHP absence is recorded as a skip reason in report environment
metadata and does not fail the smoke.

Wall-clock values in these reports are advisory smoke measurements only. A run
fails if an engine exits non-zero or if the last measured output differs from
the fixture's expected output.

## VM/Runtime Counters

Prompt 07.07 adds optional VM counters behind `VmOptions::collect_counters` and
the CLI flag:

```bash
nix develop -c target/debug/php-vm run --counters-json target/phase7/counters.json path/to/file.php
```

Counters are off by default and are returned out-of-band through `VmResult` or a
JSON file. They are never appended to PHP stdout. The current counter set
records executed instructions, stable opcode-family counts, function calls,
method calls, array-dimension fetches, property fetches/accesses, `instanceof`
type checks, include/require instructions, autoload attempts, string concats,
runtime fast-path hits/misses, quickening events, inline-cache events, dispatch
cache events, and JIT compile/execute/fallback counters.

`bench_matrix.py` enables Rust VM counters by default and embeds the parsed
counter JSON under `vm_counters` for Rust measurements. Reference-PHP
measurements remain separate and do not carry VM counters.

Validation:

```bash
nix develop -c just bench-phase7-smoke
nix develop -c jq . target/phase7/*.json
```

## Optional Local Profiling

Maintainers can use the opt-in recipes in
`docs/profiling-workflow-phase7.md` for VM dispatch, array-heavy, call-heavy,
and Composer-like local profiling. These recipes are intentionally outside
standard gates and write any profiler artifacts under `target/phase7/profiles/`.

## Optional Release Profiles

The LTO/PGO plan in `docs/release-build-profile-phase7.md` is experimental.
It keeps the default debug/dev workflow unchanged and requires comparable
before/after benchmark JSON before making any release-build claim.

## Framework Micro-Smokes

Prompt 07.E adds offline framework-like smokes in
`tests/fixtures/phase7/framework_smoke/`:

- router dispatch;
- DI-container lookup;
- attribute/reflection warm path;
- template-like string output.

Run them with:

```bash
nix develop -c just framework-smoke-phase7
nix develop -c just perf-report
```

The smoke compares opt-off against opt-on (`--opt-level=2`, quickening on,
inline caches on) and writes `target/phase7/framework-smoke/summary.json`.
`perf-report` includes that summary when present. The fixtures are local and do
not use Packagist or vendored framework repositories.
