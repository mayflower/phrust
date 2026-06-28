# Fastest Engine Current-State Plan

Date: 2026-06-28.

This is the v2 current-state fastest-engine coordination plan. It is not a
Phase 09 continuation and does not reopen already-landed acceleration work.
The goal is to use the current engine evidence to drive the next PHP
engine/runtime performance work while preserving the existing PHP 8.5.7
correctness contract.

This plan is engine-only. It does not introduce a web server, production SAPI,
Zend ABI compatibility, extension ABI compatibility, OPcache replacement, a
default-on JIT, or a pinned PHP target update.

## Current Baseline

The active source pipeline remains:

```text
php_lexer -> php_syntax -> php_ast -> php_semantics/HIR -> php_ir -> php_runtime -> php_vm -> php_vm_cli
```

Rich IR remains the verified frontend and optimizer boundary. Dense bytecode is
a VM execution format, not a replacement frontend. `--exec-format=ir` remains
the correctness baseline unless a later default-policy audit explicitly changes
that policy.

## Completed Acceleration Surface

The current source state already includes these performance surfaces:

- deterministic performance smoke and framework-like smoke fixtures;
- release smoke, optional PGO/BOLT measurement flows, and local report
  generation;
- correctness-first acceleration matrix rows for baseline IR, dense bytecode,
  superinstructions, optimizer levels, quickening, inline caches, all non-JIT
  optimizations, release binaries, and optional Cranelift;
- dense bytecode structures, verifier checks, strict execution for supported
  shapes, and `auto` fallback to rich IR for unsupported shapes;
- dense bytecode support for scalar operations, comparisons, direct calls,
  selected builtin calls through the existing VM target path, packed-array
  operations, and by-value foreach;
- default-off dense superinstructions for the initial measured echo-adjacent
  producer patterns;
- request-local quickening sites for selected arithmetic, concat, and branch
  shapes;
- guarded function and builtin inline caches, including exact initial fast
  stubs for `strlen`, `count`, `is_int`, `is_string`, and `is_array`;
- property-fetch metadata and guarded interpreter property-fetch cache paths;
- packed-array metadata owned by `php_runtime`, including packed/mixed kind,
  direct-reference presence, COW sharing, mutation epoch, and length;
- guarded packed fetch, append, and by-value foreach interpreter fast paths;
- conservative request-local frame/register reuse with blocked-reason counters;
- output fast append counters and exact string, integer, boolean, and null echo
  fast paths where modeled;
- verifier-bracketed optimizer passes for safe constant folding, literal-pool
  compaction, block-local register copy propagation, NOP/self-move peepholes,
  and conservative CFG simplification;
- default-off Cranelift experiments with feature gating, runtime selection,
  side-exit accounting, guard reports, and current evidence that only narrow
  all-int packed foreach reduction is a keep-and-expand speed candidate.

These surfaces are treated as the starting point for the v2 fastest-engine
work. New prompts should extend or measure them instead of rebuilding them.

## Target Architecture

### Tier 0: Compact IR And Dense Interpreter

Tier 0 is the correctness baseline. It uses rich IR plus dense bytecode where
explicitly selected and verified. It calls existing VM/runtime semantic helpers
for PHP-visible behavior and must preserve diagnostics, exceptions, references,
COW, destructors, visibility, magic methods, property hooks, include/autoload
order, generators, fibers, and output buffering.

### Tier 1: Quickened Interpreter, Inline Caches, And Superinstructions

Tier 1 specializes hot interpreter sites through quickening, guarded inline
caches, and semantic-preserving superinstructions. Every site needs counters,
guard-failure accounting, an off switch, and fallback to the generic helper or
an explicit strict-mode unsupported result.

### Tier 1.5: Byte/String Kernels And Compact Runtime Layouts

Tier 1.5 covers safe byte scanning, string/output kernels, allocation pressure,
frame/register layout, array/object metadata, and runtime value compactness. It
must remain byte-string aware and cannot replace PHP runtime semantics with
alternate unchecked implementations.

### Tier 2a: Optional Baseline Native Tier Prerequisites

Tier 2a is research and prerequisite work until executable-memory policy,
W^X/mprotect behavior, code-cache lifecycle, ABI and helper hashes, source-map
mapping, side exits, live-state maps, reference/COW identity, foreach state,
try/finally and exception state, generator/fiber policy, diagnostics, and PHPT
evidence are in place. No executable baseline-native tier is introduced by this
plan.

### Tier 2b: Selective Cranelift For Proven Hot Regions

Tier 2b is the existing optional Cranelift path. It remains feature-gated and
runtime-off by default. It may expand only for proven hot regions with low
guard-failure rates, clear compile-budget controls, complete fallback, and
interpreter-owned semantics.

## What Fastest Means

In this repository, "fastest" means measurable, correctness-preserving engine
throughput on the committed benchmark corpus, release profile, optional PGO/BOLT
or profiler artifacts when available, and PHP reference comparison where a
pinned reference binary is available.

It does not mean a global "fastest PHP" claim from host-local timings. Timing is
advisory unless paired with methodology, warmups, repetitions, environment
capture, output/stderr/exit-status parity, counter sanity, and clear skip
classification for optional rows.

## Policy For V2 Work

- `--exec-format=ir` remains the correctness baseline.
- Cranelift remains feature-gated and runtime-off by default.
- Optimized paths need off switches, counters, and fallback paths.
- Strict optimized modes may return explicit unsupported status for unsupported
  shapes.
- Generated reports stay under `target/` and are not committed.
- Reference-dependent checks skip clearly when no reference PHP binary is
  available and become strict when `REFERENCE_PHP` is explicitly set.
- New speed work must update the relevant gap catalog, report, or audit doc.

## Recommended V2 Order

The recommended order starts with documentation and evidence, then moves from
low-risk interpreter/runtime wins toward native-tier prerequisites:

1. Current-state gap reset.
2. Profiling evidence and top-hotpath report.
3. Safe byte-kernel facade.
4. Lexer/source-map byte scanning integration.
5. Dense bytecode coverage for properties, methods, statics, and includes.
6. Measured superinstructions v2.
7. Function/builtin polymorphic inline caches and exact generated stubs.
8. Property assignment caches and object-shape write fast paths.
9. Method-call caches and tiny-safe inlining metadata.
10. Array fast paths v2.
11. String/output batching v2.
12. Runtime value, string, and allocation layout compactness.
13. Profile-guided optimizer expansion.
14. Production fast preset and default-on audit.
15. Selective Cranelift region expansion.
16. Baseline native prerequisites and no-exec stencil prototype.
17. Deopt, live-state, and OSR metadata.
18. Comparative fastest-engine matrix.
19. Compatibility, safety, and no-regression sweep.

The additional v2 acceleration ideas follow after this base: request-local
arenas, persistent metadata feedback, numeric-string specialization,
reference/aliasing deopt policy, record-like array shapes, include/autoload
dependency graphs, builtin intrinsics, specialized call frames, hot/cold slow
path outlining, region-profile recording, copy-and-patch stencil research, and
a PHP-semantics-aware mid-tier design.
