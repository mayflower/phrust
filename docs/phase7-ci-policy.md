# Phase 7 CI Policy

Phase 7 CI uses the same Nix entrypoint as local development. The required
workflow is `.github/workflows/phase7.yml` and the required command is:

```bash
nix develop -c just verify-phase7
```

`verify-phase7` is the source of truth for required Phase 7 correctness and
smoke coverage. It runs workspace tests, regression fixtures, the full
performance-flag A/B matrix, bytecode-cache roundtrip checks, optimizer
differential checks, quickening smoke, inline-cache smoke, skip-safe Callgrind
smoke, default-off JIT smoke, safety audit smoke, benchmark smoke, hot-path
inventory, and perf-report generation.

The focused commands remain available for local bisection and CI log triage:

```bash
nix develop -c just cache-roundtrip
nix develop -c just optimizer-diff
nix develop -c just quickening-smoke
nix develop -c just inline-cache-smoke
nix develop -c just jit-smoke
```

The required workflow also runs:

```bash
nix flake check
```

The flake check must remain lightweight. Optional profiling tools, optional
benchmark-only crates, and feature-gated JIT dependencies must not become hard
flake checks. Unsupported JIT/native-code configurations must fail closed,
fallback, or skip with an explicit reason; they must not make pull-request CI
architecture-specific.

Long benchmark jobs are not required for every pull request. The same workflow
has an optional benchmark job that runs only on the weekly schedule or when a
maintainer starts `workflow_dispatch` with `run_long_benchmarks=true`:

```bash
nix develop -c just bench-phase7
nix develop -c just perf-report
```

Benchmark output is uploaded as CI artifacts from `target/phase7` and
`target/criterion`. These artifacts are diagnostic evidence for the CI host, not
portable performance budgets.

CI tests must not require secrets, network access from test code, a vendored
`php-src`, or a prebuilt reference PHP binary. Reference-dependent checks keep
the existing policy: they skip clearly when no reference binary is configured
and are strict when `REFERENCE_PHP` is explicitly set.
