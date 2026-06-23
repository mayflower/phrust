# Phase 7 Preflight

Phase 7 starts from the existing PHP 8.5.7 Rust engine and is limited to
correctness-preserving performance infrastructure. This preflight records the
current repository surface before adding bytecode cache, optimizer, quickening,
inline caches, specialization, or JIT work.

## Workspace Inventory

- Workspace root: `/Volumes/CrucialMusic/src/phrust`.
- Target PHP reference: PHP 8.5.7 / `php-8.5.7`, configured in `flake.nix`.
- Dev environment: Nix flakes only; validation commands run with
  `nix develop -c ...`.
- Existing workspace crates from `Cargo.toml`:
  - `crates/php_source`
  - `crates/php_lexer`
  - `crates/php_lexer_cli`
  - `crates/php_syntax`
  - `crates/php_parser_cli`
  - `crates/php_ast`
  - `crates/php_semantics`
  - `crates/php_frontend_cli`
  - `crates/php_ir`
  - `crates/php_runtime`
  - `crates/php_std`
  - `crates/php_testkit`
  - `crates/php_vm`
  - `crates/php_vm_cli`
- Current docs include Phase 0 through Phase 6 final audit and known-gap files,
  including `docs/phase-2/final-audit.md`,
  `docs/phase-3/phase-3-final-audit.md`, `docs/phase4-known-gaps.md`,
  `docs/phase5-final-audit.md`, `docs/phase6-final-audit.md`, and
  `docs/known-gaps-phase6.md`.
- The worktree was dirty before Phase 7 started. Phase 7 prompt 07.00 adds only
  `docs/phase7-preflight.md` and `docs/known-gaps-phase7.md`.

## VM, IR, Runtime, And Standard Library Structures

- `crates/php_ir` owns the Phase 4 bytecode/IR layer and lowering entry points,
  including `crates/php_ir/src/lower.rs`.
- `crates/php_vm` owns frame management, include handling, and VM dispatch in
  `crates/php_vm/src/frame.rs`, `crates/php_vm/src/include.rs`, and
  `crates/php_vm/src/vm.rs`.
- `crates/php_vm_cli` provides the executable CLI used by runtime and
  differential scripts.
- `crates/php_runtime` owns values, arrays, output, conversion, GC skeleton,
  resources, date/time, PCRE, INI, serialization, tokenizer support, and builtin
  context types through files such as `src/value.rs`, `src/array.rs`,
  `src/context.rs`, `src/output.rs`, and `src/builtins.rs`.
- `crates/php_std` contains the Phase 6 extension registry, arginfo model,
  constants, and metadata surface used by runtime and reflection work.
- `crates/php_testkit` exists for shared test harness support and is the natural
  home for reusable Phase 7 A/B or differential helpers if they become shared.

## Existing Test And Differential Harnesses

The current `justfile` exposes Phase 0 through Phase 6 gates:

- `verify-phase0`
- `verify-phase1`
- `verify-phase2`
- `verify-phase3`
- `verify-phase4`
- `verify-phase5`
- `verify-phase6`

Phase 6 specific commands are present:

- `test-phase6`
- `coverage-phase6`
- `phase6-generate-arginfo`
- `diff-stdlib`
- `diff-streams`
- `diff-json-pcre-date`
- `diff-spl-reflection`
- `composer-smoke`
- `composer-smoke-source`
- `composer-smoke-autoload`
- `composer-smoke-platform`
- `process-capability-smoke`
- `phase6-phpt-smoke`
- `phase6-corpus-smoke`

Important scripts:

- `scripts/verify-phase6.sh` runs `test-phase6`, the historical regression
  smoke, coverage checks, Phase 6 differential gates, and Composer smoke.
- `scripts/test-phase6.sh` runs Phase 6 preflight checks, `cargo test -p
  php_std`, `cargo test -p php_vm std_builtins`, builds `php-vm`, and runs the
  stdlib differential fixture area.
- `scripts/phase6_regression_smoke.sh` checks that Phase 0 through Phase 5 gates
  and final-audit artifacts still exist.
- `scripts/phase6_diff.py` is the Phase 6 differential harness used by stdlib,
  streams, JSON/PCRE/Date, SPL/Reflection, Composer, and corpus fixture areas.
- `scripts/runtime-corpus-smoke.sh`, `scripts/run_parser_fixtures.py`,
  `scripts/compare_parser_acceptance.py`, and `scripts/run_semantic_fixtures.py`
  remain available from earlier phases.

Fixture roots relevant to Phase 7 regression selection include:

- `tests/fixtures/runtime`
- `tests/fixtures/phase6/_harness`
- `tests/fixtures/phase6/corpus`
- `tests/fixtures/phpt`
- `fixtures/runtime`
- `fixtures/phase6`

## Existing Benchmark Or Profiling Surface

The repository has limited benchmark infrastructure before Phase 7:

- `just bench-lexer`
- `just bench-parser`
- `just bench-frontend`
- `just bench-vm-smoke`
- `just phase5-bench-smoke`
- `scripts/phase5_bench_smoke.py`
- `tools/bench_vm_smoke.rs`
- `docs/research/phase4-bench-smoke.md`

There is no committed `benches/` directory yet and no Phase 7 benchmark matrix,
performance JSON format, baseline/compare tooling, bytecode-cache roundtrip, or
optimizer/quickening/inline-cache/JIT smoke gate yet.

The Linux dev shell includes performance-adjacent tooling and build speedups:
`sccache`, `ccache`, `jq`, `hyperfine`, `ripgrep`, `fd`, `python3`, the native
PHP/C build stack, `mold`, `gdb`, and `shellcheck`. Darwin shells use
`mkShellNoCC` with the lightweight Nix tool surface needed for Phase 7
validation (`just`, `jq`, `hyperfine`, `ripgrep`, `fd`, `python3`, `ccache`,
and `sccache`) while keeping the host Rust toolchain stable and avoiding the
large compiler, Apple SDK, and Haskell package closures.

## Phase 7 Risks

- Correctness risk: Phase 7 optimizations must not change output, exit status,
  diagnostics, exception behavior, warnings, notices, deprecations, or
  timing-independent side effects.
- Invalidation risk: quickening and inline caches must be invalidated by
  function, class, method, property, include, autoload, and configuration
  changes before they can be trusted.
- Runtime model risk: Phase 6 still records several VM-context persistence and
  compatibility gaps in `docs/known-gaps-phase6.md`; Phase 7 must not hide those
  gaps behind fast paths.
- Measurement risk: existing benchmark support is smoke-level only. Phase 7
  needs deterministic benchmark inputs, environment normalization, and
  machine-readable result comparison before performance claims are meaningful.
- Cache risk: bytecode-cache input is untrusted local data and must be
  fingerprinted, versioned, verified, and safely ignored on corruption.
- JIT risk: JIT work is experimental, default-off, feature-gated, and must not
  introduce executable-memory behavior without a documented safety model.
- Worktree risk: the current checkout has pre-existing uncommitted Phase 6
  changes. Phase 7 changes should stay scoped and avoid reverting unrelated
  files.

## Validation Record

Prompt 07.00 validation command:

```bash
nix develop -c just verify-phase6
```

Completed result:

- Command: `nix develop -c just verify-phase6`
- Status: passed.
- Evidence:
  - `cargo test -p php_std`: 40 library tests passed.
  - `cargo test -p php_vm std_builtins`: 2 tests passed.
  - `scripts/phase6_regression_smoke.sh`: passed.
  - `coverage-phase6`: passed and regenerated the Phase 6 function coverage
    document.
  - `diff-stdlib`: reused the `test-phase6` report with 42 total fixtures, 36
    passed, 0 failed, 0 skipped, and 6 known gaps.
  - `diff-streams`: 2 total, 2 passed.
  - `diff-json-pcre-date`: 3 total, 3 passed.
  - `diff-spl-reflection`: 2 total, 2 passed.
  - `composer-smoke`: 5 total, 5 passed.
