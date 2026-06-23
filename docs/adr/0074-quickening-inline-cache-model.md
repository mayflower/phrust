# ADR-0074: Quickening And Inline Cache Model

## Status

Accepted.

## Context

Phase 7 introduces performance infrastructure after the Phase 4 VM, Phase 5
runtime model, Phase 6 standard-library surface, bytecode cache, and optimizer
framework. The next layer is adaptive execution: gather local runtime evidence,
try a guarded specialized operation, and fall back to the existing interpreter
operation whenever the evidence no longer applies.

This ADR defines the shared model for quickening and inline caches before any VM
opcode is changed. It is intentionally separate from optimizer passes and from
JIT compilation.

## Decision

Quickening is a request-local VM execution strategy. A baseline instruction may
accumulate counters and attach side-table state. Once a threshold is reached,
the VM may use an adaptive or specialized handler for that instruction. The
generic interpreter instruction remains the semantic source of truth.

### Terms

- Baseline opcode: the existing IR instruction behavior with no runtime
  specialization. This is the path used by `--opt-level=0` and by every
  fallback.
- Adaptive opcode: a baseline instruction with attached counters and optional
  quickening state. It observes operand shapes and dispatches either to the
  generic path or to a candidate specialized path.
- Specialized opcode: a guarded fast handler for one stable shape, such as
  int/int `ADD` or string/string `CONCAT`. Phase 7 may implement this as a VM
  side-table entry before adding new IR instruction variants.
- Guard: a checked assumption that must hold before a specialized handler can
  run. Guards include value type, array layout, key kind, function identity,
  class identity, property slot, reference state, COW state, and invalidation
  epoch as appropriate.
- Dequickening/deopt fallback: the transition back to the baseline instruction
  when a guard fails, a counter saturates negatively, an invalidation epoch is
  stale, or the specialized handler reports unsupported behavior.
- Counter threshold: the minimum evidence required before trying a specialized
  handler. Thresholds must be configurable or documented constants and must
  prefer under-specialization over incorrect specialization.
- Invalidation event: a runtime event that makes previously observed lookup or
  shape evidence stale.
- Stats: counters emitted for observability, including attempts, installs,
  hits, misses, guard failures, dequickenings, and invalidations.

### Relationship To Existing Layers

Quickening is not an optimizer pass. Optimizer passes transform IR before
execution, are controlled by opt levels, and must preserve verifier validity.
Quickening observes runtime values during execution and must be discardable
without changing the compiled unit.

Quickening is not a bytecode cache. Quickening state is request-local in Phase 7
unless a later ADR defines a safe lifecycle for cross-request sharing. Cached
IR/bytecode artifacts must not depend on warmed quickening state.

Quickening is not JIT. Specialized handlers are still interpreter code. JIT
eligibility, executable memory, W^X policy, native ABI, and tiering belong to
the later JIT ADRs and must not be assumed by this model.

Inline caches are a related side-table mechanism for lookups. A quickened
opcode may consult an inline cache entry, but cache contents are guarded by the
same fallback and invalidation rules as other quickening state.

## First Candidates

The initial quickening candidates are deliberately small and reversible.

| Candidate | Baseline behavior | Guard | Fallback |
| --- | --- | --- | --- |
| `ADD` int/int | Generic binary addition and PHP numeric coercion behavior. | Both operands are unreferenced integer values; addition does not overflow the engine integer representation; no numeric-string coercion is needed. | Generic binary addition. |
| `CONCAT` string/string | Generic PHP concat conversion and allocation behavior. | Both operands are PHP strings; COW/reference state is respected; allocation succeeds. | Generic concat conversion path. |
| `FETCH_DIM` packed-array/int-key | Generic array/string/object/null dimension fetch behavior. | Receiver is an array with packed layout; key is a non-negative integer in range; element read does not expose mutable storage unsafely. | Generic dimension fetch, including warnings/null behavior. |
| `CALL` known function | Generic callable resolution and call binding. | Function table epoch and function id match; named/variadic/by-ref/default binding assumptions match the observed call shape. | Generic call resolution and binding. |
| `FETCH_PROP` monomorphic class slot | Generic property visibility, dynamic property, hook, magic, and error behavior. | Object class id/epoch matches; property slot and visibility are stable; no property hooks, magic access, dynamic property creation, or uninitialized typed-property edge applies. | Generic property fetch path. |

## Guard Failure Behavior

Guard failure is normal. It must not be a user-visible error by itself.

On a guard failure the VM must:

1. Increment the relevant guard-failure and miss counters.
2. Execute the baseline operation for that instruction.
3. Preserve output, diagnostics, exception behavior, references, COW,
   destructors, autoload side effects, and shutdown behavior.
4. Optionally decrement confidence or dequicken the instruction when failures
   exceed the documented threshold.

Specialized handlers must not partially mutate state before all required guards
for that mutation have passed. If a handler cannot prove this, it must not be
installed.

## Counter Thresholds

Phase 7 starts with conservative thresholds:

- Observation threshold: 8 executions of the same instruction before
  specialization is considered.
- Stable-shape threshold: 6 matching observations for the same shape.
- Guard-failure threshold: 2 failures after installation dequicken the entry.
- Saturation: counters saturate rather than wrap.

Prompts that implement quickening may tune these constants, but changes require
tests that prove fallback behavior and counter determinism.

## Invalidation Events

Quickening and inline-cache entries must be invalidated or ignored when any
assumption they rely on may be stale. Relevant events include:

- New function, class, method, constant, property, trait, enum, or interface
  declarations from include/require/eval.
- Autoload registration, unregistration, execution, or failure that can change
  class/function availability.
- Include path, working directory, request context, INI, or stream wrapper state
  changes that affect resolution.
- Class inheritance, interface, method table, property table, static property,
  or class constant epoch changes.
- Dynamic property creation or property hook/magic method paths for objects
  that were previously assumed monomorphic.
- Any runtime feature that can alter reference/COW aliasing assumptions.

Phase 7 state is request-local by default. No global persistent quickening
state is allowed without a lifecycle plan covering invalidation, request
cleanup, worker recycling, and configuration reload.

## PHP Correctness Risks

Specialization must account for PHP reference behavior. A value that appears to
be a plain scalar may be stored in a reference cell or may be reachable through
aliases. Fast paths must not skip reference separation, by-reference parameter
binding, lingering foreach references, or property/array reference semantics.

COW remains observable through later mutation behavior even when storage
identity is not directly visible. String and array fast paths must either reuse
existing COW helpers or prove they do not expose shared mutable storage.

Destructors and exceptions can run at surprising times. Fast paths must not skip
object destruction registration, exception propagation, finally blocks, shutdown
handlers, or diagnostics. A specialized handler that allocates or invokes code
must have a precise fallback boundary.

Reference PHP compatibility remains the acceptance standard. Quickened and
baseline execution must match output, exit status, diagnostics, warning
continuation, exception classes, and timing-independent side effects.

## Stats

The VM counter surface should add quickening and inline-cache fields as the
implementation lands:

- `quickening_observations`
- `quickening_installs`
- `quickening_hits`
- `quickening_misses`
- `quickening_guard_failures`
- `quickening_deopts`
- `quickening_invalidations`
- `inline_cache_hits`
- `inline_cache_misses`
- `inline_cache_invalidations`

Stats are diagnostic. They must not affect semantics and must remain optional
for normal execution.

## Consequences

Quickening can improve hot interpreter operations without requiring IR changes
or native code. The cost is extra state, invalidation complexity, and more
fallback tests. Phase 7 therefore starts with side-table state, strict guards,
small candidates, deterministic stats, and default-safe fallback behavior.
