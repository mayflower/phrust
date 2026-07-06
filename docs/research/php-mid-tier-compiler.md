# PHP-Aware Mid-Tier Compiler Research

Date: 2026-06-28.

Reference target: PHP 8.5.7 (`php-8.5.7`).

This document defines a future PHP-semantics-aware mid-tier compiler for the VM.
It is design and metadata-only prototype evidence. It does not enable default-on
native execution, allocate executable memory, or add a second parser, semantic
frontend, bytecode format, or source-string execution path.

## Tier Role

The mid-tier is not a generic LLVM-style optimizer over arbitrary code. Its job
would be to compile only the warm PHP regions where runtime feedback proves that
the interpreter, quickening, inline caches, and copy-and-patch stencils leave
meaningful work on the table while the full Cranelift path would be too broad or
too expensive.

The tier is PHP-specific because profitability and correctness depend on
language state that generic scalar optimization cannot infer safely: reference
identity, Copy-on-Write mutation, numeric-string coercion, destructor order,
autoload/include invalidation, magic methods, property hooks, generator/fiber
suspension, and diagnostic/output ordering.

## Inputs

A valid mid-tier candidate needs all of these inputs from existing VM-owned
layers:

- quickened dense bytecode and dense block/instruction indexes;
- inline-cache feedback for functions, methods, properties, includes, and
  builtins;
- array shapes, packedness, key classifications, and element summaries;
- object class/layout/property-slot shape feedback;
- numeric-string classification feedback for operands and keys;
- alias/reference state, including reference/COW poison markers;
- branch-bias metadata;
- persistent feedback with source, option, epoch, and architecture validation;
- deopt and live-state maps for interpreter resume.

The prototype command reports those inputs as required metadata and classifies
current dense functions/regions without executing native code:

```bash
php-vm dump-mid-tier-plan <file.php> --json
```

The smoke gate is:

```bash
nix develop -c just mid-tier-plan-smoke
```

It writes per-fixture reports and an aggregate summary under:

```text
target/performance/mid-tier/
```

The current smoke summary covers six fixtures and produced:

| Metric | Value |
| --- | ---: |
| Eligible functions | 4 |
| Rejected functions | 3 |
| Quickened superinstructions | 22 |
| Deopt points | 83 |
| Candidate optimization families | 6 |
| Rejection reason families | 8 |
| Expected guard families | 11 |
| Required helper families | 2 |

## Output

The initial output is report-only pseudo-IR metadata:

- per-function `eligible` or `ineligible` classification;
- candidate optimizations;
- expected guards;
- required helpers;
- deopt points;
- rejection reasons and implementation prerequisites.

No generated report is executable. The interpreter remains the only runtime path
for the prototype.

## Candidate Optimizations

| Optimization | PHP-specific proof required | Current prototype evidence |
| --- | --- | --- |
| Method/property shape check hoisting | Monomorphic class/layout epochs, visibility scope, property-hook/magic rejection, uninitialized typed-property exit, function/class invalidation. | Reported as a prerequisite gap when dense bytecode lacks property/method-shape metadata for the candidate region. |
| Tiny leaf method inlining | Final/monomorphic callee, no by-reference params/returns, no generator/fiber, no include/eval mutation, exact frame and destructor behavior. | Tiny dense functions without rejections are marked as `tiny_leaf_method_inlining_candidate`. |
| Builtin intrinsic inlining | Exact builtin identity, stable arity and argument shapes, helper status contract, diagnostic ordering. | Dense known `strlen`/`count` calls are marked as `builtin_intrinsic_inlining`. |
| Packed-array loop specialization | Packed layout, integer keys, element summary, no by-reference element access, no mutation epoch changes. | Dense dimension fetches are marked as `packed_array_loop_specialization` with helper/deopt requirements. |
| Numeric-string guard specialization | Operand feedback that distinguishes int, float, numeric string, non-numeric string, object conversion, and overflow behavior. | Dense add/sub/mul instructions are marked as `numeric_string_guard_specialization`. |
| Branch layout | Taken/not-taken bias, guard failure rate, fallthrough safety, and exact resume blocks. | Dense conditional jumps are marked as `branch_layout`. |
| Allocation/scratch-buffer elision | Proven string/output lifetimes, no observable buffer callbacks, no conversion-sensitive object/string behavior. | Dense concat/echo paths are marked as `allocation_scratch_buffer_elision`. |

## Mandatory Rejections

The mid-tier must reject or remain metadata-only for these conditions until the
corresponding prerequisite is implemented and fixture-covered:

| Rejection | Reason |
| --- | --- |
| `references_or_unknown_aliasing` | Unknown alias state can make loads, stores, COW copies, and by-reference calls observe different identities. |
| `cow_mutation_ambiguity` | Array/object mutation needs exact COW ownership, allocator state, key coercion, and diagnostic order. |
| `magic_hooks_or_dynamic_calls` | Dynamic dispatch, magic methods, property hooks, and userland calls require exact call-frame and visibility behavior. |
| `eval_include_mutation_requires_invalidation` | Includes and eval can mutate function/class tables, constants, autoload state, and local symbol visibility. |
| `exceptions_try_finally_need_live_state_support` | Throw, catch, finally, and unwind paths need exact live-state and pending-finally materialization. |
| `generators_fibers_require_suspend_state` | Suspension needs resumable VM frames, locals, stack state, pending exceptions, and output state. |
| `destructor_sensitive_values_need_materialization` | Temporaries and locals whose release can run destructors must preserve ordering and exception behavior. |

These are not optional safety checks. A future executable implementation must
prove each rejection can be guarded, represented, or excluded.

## Placement Against Existing Tiers

| Situation | Preferred tier |
| --- | --- |
| Cold code, weak feedback, dynamic code loading, references, generators, fibers, magic/hook-heavy paths, or broad mutation. | Interpreter with counters and inline caches. |
| Short warm paths made of locals, scalar guards, branches, known builtins, and simple packed fetches where compile latency dominates. | Future copy-and-patch stencils. |
| Warm regions with stable ICs, shapes, branch bias, numeric-string feedback, and exact live-state maps, where multiple guards or helpers can be shared. | Future PHP-aware mid-tier. |
| Narrow hot packed/numeric kernels that justify higher compile cost and already pass Cranelift eligibility and safety gates. | Default-off Cranelift kernel tier. |

The mid-tier should not replace existing quickening, superinstructions, inline
caches, or the no-exec baseline-native research. It should consume those
metadata sources and only compile when they prove a stable, profitable region.

## Cranelift Comparison

Cranelift remains useful for the narrow rows where native code has already
proved correctness and local performance potential, especially all-int packed
foreach reductions. The mid-tier would sit before Cranelift for PHP-heavy
regions where the profitable work is guard sharing, shape hoisting, call/builtin
metadata, and deopt layout rather than generic low-level instruction selection.

The current Cranelift reporting path already models helper status, side exits,
ABI hashes, and default-off execution. The mid-tier must reuse those safety
concepts, but it needs stricter PHP runtime metadata before any executable
region can exist.

## Implementation Prerequisites

Before this design can move beyond reports:

- dense bytecode must expose method/property shape operations in a form the tier
  can consume;
- IC feedback must carry stable monomorphic/polymorphic/megamorphic states with
  invalidation epochs;
- numeric-string classification feedback must be available for operands and
  array keys;
- alias/reference/COW summaries must be exact enough to reject poisoned regions;
- branch-bias feedback must include taken/not-taken history;
- persistent feedback must be written by an owned engine cache and validated by
  source/options/epochs/architecture;
- deopt/live-state maps must materialize locals, registers, temporaries,
  pending diagnostics, output buffers, exceptions/finally, destructors,
  generator/fiber state, and frame identity;
- helper ABI/status contracts must be stable and hashed;
- PHPT/reference fixtures must prove every enabled region preserves output,
  diagnostics, exit status, warning order, destructor order, and side effects.

Method dispatch (`CallMethod`/`CallStaticMethod`) now surfaces its own metadata
in the plan alongside the existing property-fetch/assign guards: the candidate
optimization `monomorphic_method_dispatch_specialization`, the guards
`receiver_class_epoch`, `method_table_epoch`, `method_slot`,
`final_or_static_method`, and `by_reference_parameter_compatibility`, and the
specific rejection reason `method_dispatch_requires_runtime_class_binding`. It
stays rejected — this is metadata for a future tier, not eligibility.

Until those prerequisites are complete, the mid-tier remains report-only.
