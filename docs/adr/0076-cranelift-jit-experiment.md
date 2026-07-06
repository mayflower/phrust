# ADR-0076: Cranelift JIT Experiment

## Status

Accepted as an experimental, default-off Performance path.

## Context

Performance is a correctness-preserving optimization layer. The interpreter, IR
verifier, optimizer, quickening layer, inline caches, and runtime fast paths
already provide the semantic baseline for PHP 8.5.7 behavior. A JIT experiment
can only be useful if it consumes the existing frontend-to-runtime pipeline and
never becomes a second frontend, a second semantic model, or the only execution
strategy.

Cranelift is the candidate backend because it is Rust-native, embeddable, has a
stable IR API surface compared with hand-written machine code, and supports
multiple host architectures through one backend. It also lets this repository
test a real native-code path without committing to Zend Opcache JIT internals or
platform-specific assembly in Performance.

## Decision

Performance may add a `jit-cranelift` experiment, default off. The experiment is
allowed to add JIT API scaffolding, eligibility analysis, ABI types, smoke
gates, optional Cranelift-backed IR lowering for a tiny subset, and guarded
safe execution prototypes that remain interpreter-fallback-first. The default
build must not require Cranelift, executable memory, or platform JIT support.

The interpreter remains the source of truth. All JIT-eligible code must have an
available interpreter fallback. Any unsupported feature, guard failure, ABI
error, platform limitation, code-cache failure, or safety-audit failure must
reject JIT execution and continue through the interpreter.

## Initial IR Subset

The first eligible subset is intentionally small:

- pure leaf functions or single hot-loop regions;
- primitive integer and boolean operations;
- local-slot reads and writes that do not alias through references;
- simple constants and simple returns;
- no arrays, objects, resources, strings requiring PHP conversion, references,
  copy-on-write mutation, destructors, includes, eval, autoload, generators,
  fibers, exceptions, `try`, `catch`, or `finally`;
- no userland calls and no internal calls except explicitly modeled intrinsics.

Everything outside that subset is rejected with a stable reason. Rejecting too
much is correct; accepting unsupported PHP behavior is not.

## ABI Boundary

The JIT ABI is a narrow boundary between VM-owned state and native code:

- VM context is passed as an opaque handle, not a borrowed Rust reference that
  can escape native code.
- Frame/register access is represented through explicit view types owned by the
  JIT boundary layer.
- Values crossing the boundary use a documented representation that cannot
  bypass reference, COW, GC, visibility, or destructor invariants.
- Native code returns a structured result: normal return, bailout/deopt,
  runtime-callout request, or exception propagation marker.
- Runtime callouts are explicit and must re-enter the interpreter/runtime through
  safe wrappers.

No raw internal Rust reference may be stored by JIT code or survive past the
call boundary. If `unsafe` becomes necessary, it must be isolated in a small
module with invariants documented in `docs/performance/safety-audit.md`.

## Guards and Deoptimization

The first JIT tier uses conservative guards:

- eligibility guards are checked before compile;
- runtime guards are checked before entering native code and at callout points;
- guard failure returns a bailout result and resumes the interpreter at a
  well-defined IR location;
- megamorphic or repeatedly failing regions are disabled for the request or code
  cache epoch;
- guard and bailout counters are emitted so `jit-smoke` can prove fallback is
  exercised without depending on wall-clock timing.

Performance does not require speculative object, array, method, property, reference,
or exception deoptimization.

## Code Cache Lifecycle

The code cache is request-local until a later ADR proves a shared lifecycle. A
cache entry is keyed by IR identity, compiler options, target triple, feature
flags, and invalidation epochs that affect eligibility. Entries are dropped on
source/IR mismatch, unsupported platform, feature disable, failed verification,
or guard instability.

Persistent OPcache-style native code sharing, preloading, process-wide eviction,
and FPM/SAPI worker lifecycle are outside Performance.

## Safety Model

Default builds and tests run with `jit-cranelift` disabled and must not allocate
or execute writable/executable memory. Feature-enabled builds may compile JIT
infrastructure and may execute guarded safe-Rust prototypes after Cranelift
verification. Executing native machine code requires a documented W^X or
equivalent policy before it can become more than a local experiment.

The safety model requires:

- no executable-memory path without a local safe wrapper and audit note;
- no native code execution from unverified or stale IR;
- no fallback path that skips PHP diagnostics, destructors, references, COW,
  exceptions, or observable output;
- no hard performance gate based only on wall-clock timings;
- no feature-on behavior that changes feature-off interpreter output.

## Platform Boundaries

The required Performance behavior is portable skip or fallback. A host without
Cranelift support, executable-memory support, or a verified W^X implementation
must compile feature-off and pass `jit-smoke` as skipped/default-off. Feature-on
native execution may be limited to explicitly documented targets after tests
exist for those targets.

## Feature Flag

The experiment uses a default-off feature named `jit-cranelift`. CLI behavior
must default to `--jit=off` once a CLI switch exists. Enabling the feature is not
the same as enabling execution; runtime flags and eligibility must still permit
or reject each region.

Work item adds optional dependencies on `cranelift-codegen` and
`cranelift-frontend` behind this feature. The current prototype builds and
verifies Cranelift IR text. Work item adds `--jit=on` integration for hot,
eligible integer leaf functions; after warmup the VM calls the Cranelift lowerer
as compile proof and executes only a guarded safe-Rust integer evaluator. It
does not emit executable memory, does not return native function pointers, and
falls back to the interpreter on compile rejection, guard failure, or unsupported
runtime values.

## Abort Criteria

The JIT experiment must stay disabled, be reverted, or be handed off to a later
layer if any of these happen:

- JIT output, stderr, exit code, diagnostics, exception class, or
  timing-independent side effects diverge from interpreter output.
- A bailout cannot resume the interpreter at a proven-safe location.
- ABI values can violate reference, COW, GC, destructor, or visibility
  invariants.
- Executable-memory handling cannot satisfy W^X or platform security rules.
- Feature-off builds pull in Cranelift or require executable-memory support.
- The implementation requires broad frontend, IR, VM, or runtime rewrites.
- The only evidence of benefit is noisy wall-clock data without correctness
  proof.

## Consequences

This decision allows later Performance work items to add `php_jit`, eligibility, ABI
types, smoke gates, and optional Cranelift compilation. It does not authorize a
production JIT, shared native-code cache, Zend JIT compatibility, or any
semantic shortcut for speed.
