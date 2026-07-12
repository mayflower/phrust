# ARM performance tranche v2

Date: 2026-07-12

Target: `aarch64-apple-darwin`, PHP reference target `8.5.7`.

## Decision

Keep the cache, copy-patch ABI, dense-call, and pre-argument intrinsic changes.
Keep Cranelift opt-in through `experimental-jit`; do not enable it in the
default engine without a clean WordPress latency win.

No WordPress p50, p95, throughput, or PHP factor is claimed by this tranche.
The local WordPress gate skipped because the WordPress checkout, runnable
Phrust server, database configuration, and reference PHP target were not
available together. Generated `target/` reports from older runs were not used
as evidence.

## Implemented

- Worker-persistent, identity- and epoch-guarded function-call ICs and class
  metadata caches. Request-visible heap defaults fail closed instead of being
  retained.
- ARM copy-patch full-value array fetch ABI for integer and string keys, plus
  full-value return/resume support for null, bool, int, float, string, array,
  and object values. Unsupported or mismatched shapes side-exit.
- Bounded per-unit native-leaf prewarm with positive and negative recognition
  caching. Recompiled units receive a new cache identity and prewarm marker.
- Stack-inline dense positional arguments for up to eight values and direct
  frame initialization. The generic `Vec<PreparedArg>` is absent on this path.
- Dense builtin intrinsics execute from borrowed operands before generic
  `CallArgument` and value-vector materialization, after normal name resolution
  has preserved namespace fallback and userland shadowing.
- Dense method ICs use the same guarded direct-call contract as rich dispatch.
- Worker-persistent Cranelift compile handles. Copy-patch runs first; Cranelift
  is attempted only after a miss and remains opt-in.
- ARM WordPress native-tier A/B tooling covers `experimental-jit`, persistent
  feedback off/on, and copy-patch off/on. Clean timing and instrumented JIT
  compile evidence are emitted as separate runs.

## Focused evidence

| Area | Result |
| --- | --- |
| Persistent IC/class metadata | Second request records worker IC, class, default-slot-template, and constructor cache hits; output remains `3333` |
| Cache invalidation | Recompiled-unit identity invalidates metadata; heap-backed defaults are rejected |
| Native prewarm | First request compiles and rejects representative leaves; second request has positive and negative cache hits and zero prewarm attempts |
| Dense direct calls | Seven calls use bare inline arguments and avoid at least seven prepared-argument vector allocations |
| Pre-argument builtins | Six representative intrinsics hit; internal generic dispatch count is zero |
| Borrowed builtin reads | Focused fixture removed all eight call-argument snapshot clones and reduced total value clones from 54 to 46 |
| Cranelift differential | 68 fixtures pass |
| Cranelift smoke matrix | 70 rows pass; 35 Cranelift rows: 23 executed, 6 expected fallbacks, 6 expected side exits |
| Cranelift native activity | 35 compile attempts, 28 compiled regions, 220 executions, 260 fast-path hits, 198 compile-cache hits, 6 side exits |
| Worker Cranelift cache | First request records a compile-cache miss; second request records a hit on the production `Auto` path |

The smoke matrix disables copy-patch explicitly in both baseline and Cranelift
arms. Without that isolation, default-on copy-patch consumes eligible leaves
before Cranelift and makes the Cranelift matrix report false zero coverage.
WordPress A/B keeps copy-patch as an explicit matrix dimension instead.

## WordPress matrix

| Metric | Current result |
| --- | --- |
| Phrust / PHP 8.5.7 factor | unavailable |
| Phrust p50 / p95 | unavailable |
| PHP p50 / p95 | unavailable |
| Native coverage and side exits | tooling ready; no current WordPress sample |
| Persistent IC/cache hits | focused worker tests only |
| Builtin dispatches | focused fixture: zero generic dispatches for six covered intrinsics |
| Dense/rich split | tooling ready; no current WordPress sample |
| Clone/allocation counters | focused fixtures only |

Run the complete ARM matrix when the environment is available:

```bash
nix develop -c python3 scripts/performance/wordpress_root_benchmark.py \
  --mode clean \
  --cranelift-ab \
  --strict \
  --database-identity <restored-snapshot-id>
```

Each arm writes a clean summary and a separate diagnostic summary containing
`jit_compile_attempts`, `jit_compile_time_nanos`, compiled regions, side exits,
and request phase timings.

## Rejected work

- No request arena was added. Borrowed operand reads already moved the measured
  clone category, while no WordPress allocation profile was available to prove
  an arena would improve a dominant sink.
- Quickening tables were not persisted blindly. Their mutable request-local
  specialization state needs stronger cross-request guard ownership than the
  current immutable IC/class metadata.
- Cranelift was not enabled by default. Microbenchmark coverage and compile
  cache reuse are necessary but do not establish a WordPress p50/p95 win.
- No x86 or x86_64 implementation or performance claim was made.

## Validation

The following final gates passed on `aarch64-apple-darwin`:

- `nix develop -c cargo fmt --all -- --check`
- `nix develop -c cargo clippy --workspace --all-targets -- -D warnings`
- `nix develop -c cargo test --workspace`
- `nix develop -c cargo clippy -p php_jit -p php_vm -p php_executor -p php_server --all-targets --features jit-cranelift -- -D warnings`
- `nix develop -c just perf-pr-guard`
- `nix develop -c just jit-cranelift-bench-smoke`
- `nix develop -c just verify-performance`
- `nix develop -c python3 scripts/performance/wordpress_root_benchmark.py --self-test`

The real four-arm `--cranelift-ab` entrypoint completed as `skip` with
`missing_wordpress_checkout`. Its generated report is under
`target/performance/wordpress-root/cranelift-ab-current/` and is not committed.
The gate must remain a reported skip until the environment blocker is resolved;
it must not be converted into a pass.
