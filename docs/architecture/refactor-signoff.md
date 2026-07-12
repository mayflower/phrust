# Architecture Remediation Sign-off

This note closes the Prompt 00-18 architecture-remediation sequence against
the source and executable gates present on 2026-07-12. The architecture
baseline was introduced at `6fb78c8b`; the matching performance capture uses
source revision `1f304be1`. The final capture was run from the dirty integration
tree based on `46604738`, before the final VM decomposition commit.

## Result

The hard source constraints pass. The inventory reports no structural source
reparsing fallback, diagnostic-string parsing, or pointer-derived logical
identity. The VM facade is 1,663 lines, down from 68,780 lines, and no longer
has module-wide `too_many_arguments` or `result_large_err` suppression. The
runtime and VM behavior gates pass against PHP 8.5.7 without application
changes.

The final source-derived inventory contains 489 production Rust files, 383,370
lines, and 13,779,496 bytes. Prompt 00 contained 383 files, 380,153 lines, and
13,754,384 bytes. The file-count increase is the intended replacement of large
mixed-owner modules with focused owners; total production lines increased
0.85% while `vm/mod.rs` decreased 97.58%.

## Measured Comparison

Both captures used three runs on macOS 26.5 arm64 with Python 3.13.13 and peak
RSS support. Compile measurements use warm dependencies, package-clean rebuilds
or incremental root touches, and disabled sccache. Positive percentages mean
larger or slower.

| Metric | Prompt 00 | Final | Change | Command | Interpretation |
| --- | ---: | ---: | ---: | --- | --- |
| `php_runtime` clean build | 9.528 s | 9.502 s | -0.27% | `cargo build -p php_runtime` | unchanged |
| `php_vm` clean build | 10.860 s | 10.572 s | -2.65% | `cargo build -p php_vm` | unchanged |
| `php_executor` clean build | 1.104 s | 1.269 s | +14.91% | `cargo build -p php_executor` | small absolute increase, 165 ms |
| `php_server` clean build | 6.164 s | 5.809 s | -5.76% | `cargo build -p php_server` | modest improvement |
| `php_runtime` incremental | 5.644 s | 5.871 s | +4.03% | root touch plus package build | unchanged |
| `php_vm` incremental | 5.476 s | 6.052 s | +10.51% | root touch plus package build | 576 ms increase |
| `php_executor` incremental | 0.813 s | 0.978 s | +20.26% | root touch plus package build | small absolute increase, 165 ms |
| `php_server` incremental | 4.663 s | 4.813 s | +3.20% | root touch plus package build | unchanged |
| `php-vm` release binary | 21,391,520 B | 21,687,872 B | +1.39% | release CLI/server build | minor growth |
| `phrust-php` release binary | 24,010,496 B | 24,411,168 B | +1.67% | release CLI/server build | minor growth |
| `phrust-server` release binary | 24,067,408 B | 24,484,016 B | +1.73% | release CLI/server build | minor growth |
| VM benchmark target wall time | 11.761 s | 19.998 s | +70.04% | `just benchmark-suite` | runner/build time, not VM latency; final spread was 104.901 s |
| include-cache target wall time | 2.831 s | 3.718 s | +31.33% | `just inline-cache-smoke` | runner/build time; final spread was 35.867 s |
| compiled-cache target wall time | 6.714 s | 5.217 s | -22.30% | `just cache-roundtrip` | gate passed; spread prevents a speed claim |
| compiler target wall time | 99.629 s | 8.645 s | -91.32% | `just optimizer-diff` | dominated by build-cache state |
| server target wall time | 10.687 s | 0.850 s | -92.05% | `just server-benchmark-smoke` | dominated by build-cache state |
| application target wall time | 21.474 s | 2.357 s | -89.03% | `just app-flow-smoke` | gate passed; not a throughput claim |
| front-controller target wall time | 8.876 s | 1.275 s | -85.64% | `just front-controller-hotpath-smoke` | gate passed; final spread was 18.392 s |
| WordPress root | skipped | skipped | n/a | `just wordpress-root-benchmark` | no real WordPress URL was configured |

The repository-owned targets all passed, but their outer wall times include
compilation and warmup and have several spreads larger than their medians. They
must not be interpreted as pure runtime speedups or regressions. The focused
Criterion samples from the final VM run measured the decomposed hot paths at
28.543 us for `vm_dispatch_micro_loop` (no detected change), 51.968 us for
`function_call_dispatch` (within threshold), 15.486 ns for dense function IC
hits (no detected change), 13.403 ns for dense method IC hits (+1.99%), and
12.232/12.210 ns for dense property fetch/assign hits (no detected change).
Criterion also reported changes in unrelated benchmarks against its mutable
local saved baseline; those comparisons are not Prompt 00 revision evidence.

## Ownership And Dependency DAG

```text
php_syntax -> php_ast -> php_semantics -> php_ir -> php_optimizer
                                                -> php_executor
php_runtime <- php_extensions <- php_std             |
      ^              ^                               v
      +--------------+---------------------------- php_vm
                                                     |
                                              php_vm_cli / php_server

VM facade
  -> rich_dispatch / dense_dispatch
  -> opcode-family owners
  -> calls / method_dispatch / class_operations / property_execution
  -> execution_state
  -> diagnostics / instrumentation / runtime values

builtin_adapter -> calls + execution_state + diagnostics
feature adapters -> builtin_adapter + shared operations
jit_abi -> runtime values + JIT metadata
```

The checked boundary report records every workspace edge and its reason. The
VM consumes frontend artifacts through compiled units; concrete include
compilation is owned by `php_executor`, not by `php_vm`. Extension state is
owned by typed request slots, while generated extension descriptors own the
metadata, registration, and reflection surface.

## Largest Production Files

| Prompt 00 | Lines | Final | Lines |
| --- | ---: | --- | ---: |
| `php_vm/src/vm/mod.rs` | 68,780 | `php_runtime/src/builtins/modules/core.rs` | 14,292 |
| `php_runtime/src/builtins/modules/core.rs` | 14,284 | `php_ir/src/lower/expressions.rs` | 10,536 |
| `php_ir/src/lower/expressions.rs` | 10,797 | `php_vm/src/vm/builtin_classes.rs` | 8,770 |
| `php_vm/src/vm/builtin_classes.rs` | 8,744 | `php_vm/src/vm/spl.rs` | 6,928 |
| `php_vm/src/counters.rs` | 7,036 | `php_vm/src/counters.rs` | 6,927 |
| `php_vm/src/vm/spl.rs` | 6,915 | `php_vm_cli/src/commands.rs` | 6,713 |
| `php_vm_cli/src/commands.rs` | 6,783 | `php_phpt_tools/src/commands/mod.rs` | 6,645 |
| `php_std/src/extensions.rs` | 6,696 | `php_vm/src/vm/dense_dispatch.rs` | 5,881 |
| `php_phpt_tools/src/commands/mod.rs` | 6,634 | `php_jit/src/copy_patch.rs` | 5,280 |
| `php_runtime/src/builtins/context.rs` | 6,014 | `php_vm/src/inline_cache.rs` | 5,053 |

The remaining large files are cohesive implementation catalogs or hot direct
dispatch surfaces. `core.rs` owns core builtins, `expressions.rs` owns typed IR
expression lowering, `builtin_classes.rs` and `spl.rs` own internal-class
adapters, and `dense_dispatch.rs` keeps opcode dispatch direct. Their exact
line and byte budgets are ratcheted by `architecture-guardrails`.

## Removed Debt

- Module-wide lint allowances fell from 24 to 14. All VM-wide
  `too_many_arguments` and `result_large_err` allowances were removed.
- Source-reparsing findings fell from 31 to zero; classified token/source
  access remains explicit and non-structural.
- Diagnostic-string parsing findings fell from 7 to zero.
- Pointer-derived logical identity findings fell from 4 to zero.
- Root public modules/re-exports fell from 37/29 to 3/0 in `php_runtime` and
  from 24/18 to 2/0 in `php_vm`.
- `BuiltinExtensionState` fallback-plus-optional-slot ownership was replaced by
  typed request-state slots; APCu is registered like other migrated state.
- Handwritten extension metadata ownership was replaced by generated canonical
  descriptors consumed by metadata, registration, and reflection.
- The include compiler no longer parses rendered diagnostics or concatenates
  trait source. Multi-file compilation sessions preserve source identity.
- Inline-cache variant option fields were replaced by typed payloads and dense
  slot-indexed hot caches.
- Optimizer rollback is transaction-owned and does not clone the full `IrUnit`
  for every pass.

## Correctness Evidence

The include regression suite covers
`include_cache_rejects_same_metadata_atomic_replacement`, local trait atomic
replacement, `include_cache_rejects_symlink_target_swap`, mutable in-place
content changes, unchanged cache hits, and compiler-configuration separation.
The multi-file compilation tests preserve declaration and diagnostic source
origins. Runtime fixtures cover calls, objects, generators, fibers, reflection,
extensions, sessions, includes, autoload, and traits. Server tests cover stable
script identity, typed worker errors, request isolation, and concurrency.

Executed final gates:

- `nix develop -c just fmt`
- `nix develop -c just lint`
- `nix develop -c just test`
- `nix develop -c cargo check -p php_vm --all-features`
- `nix develop -c cargo clippy -p php_vm --all-targets --all-features -- -D warnings`
- `nix develop -c cargo test -p php_vm --all-features` (1,049 passed)
- `nix develop -c just architecture-guardrails`
- `nix develop -c just architecture-inventory`
- `nix develop -c just verify-runtime` (471 oracle cases, zero failures)
- `nix develop -c just verify-phpt` (21,548-entry baseline and source-integrity checks passed)
- `nix develop -c just cache-roundtrip`
- `nix develop -c just inline-cache-smoke` (25 fixtures)
- `nix develop -c just jit-smoke`
- `nix develop -c just architecture-performance-baseline --runs 3 --out target/architecture/performance-baseline/final-prompt18`

`ci-local` and `verify-performance` were not rerun after these owner gates
because they repeat the same full workspace and benchmark commands; validation
was run once at each owning layer instead. Miri, sanitizers, and supported Linux
release builds were not run because the current host is macOS arm64 and no
corresponding repository target is exposed by `just help`. WordPress remained
skipped because no true WordPress endpoint was configured.
