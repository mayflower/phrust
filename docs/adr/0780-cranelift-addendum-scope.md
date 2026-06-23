# ADR-0780: Cranelift Big-Wins Addendum Scope

## Status

Accepted as a Phase 7 addendum scope record.

## Context

Phase 7 already contains a conservative, default-off JIT experiment described
by `docs/adr/0076-cranelift-jit-experiment.md`. That experiment proves the
repository can keep Cranelift optional while preserving interpreter fallback,
but it does not yet define a complete big-win plan for native-code-backed
optimization work.

The Cranelift addendum starts after the Phase 7 counter, benchmark, tiering,
quickening, inline-cache, and initial JIT surfaces exist. It must keep the same
pipeline:

```text
php_lexer -> php_syntax -> php_ast -> php_semantics/HIR -> php_ir -> php_runtime -> php_vm -> php_vm_cli
```

The interpreter remains the source of truth for stdout, stderr, exit status,
diagnostics, exceptions, destructor order, reference behavior, copy-on-write
behavior, autoload effects, include/eval behavior, and all other observable PHP
semantics.

## Decision

The addendum may extend Phase 7 with a backend-neutral JIT API and a
Cranelift-backed implementation for a small set of high-value regions. The
feature stays default-off, Cranelift dependencies stay behind `jit-cranelift`,
and every optimized path needs a deterministic interpreter fallback and
off-versus-on differential fixture.

The first big-win targets are:

- integer arithmetic leaf functions;
- counted integer loops;
- read-only packed-array indexed fetches;
- packed-array `foreach` integer reductions;
- known pure or side-effect-modeled internal calls such as `strlen` and
  `count`;
- string concatenation fast paths with bounded allocation semantics;
- monomorphic property loads with epoch-checked metadata;
- direct monomorphic method calls with epoch-checked metadata.

Any unsupported PHP construct must be rejected before native entry or leave via
a structured side exit. Rejecting too much is acceptable. Accepting a region
that can silently diverge from the interpreter is not acceptable.

## Out Of Scope

This addendum does not authorize:

- a second lexer, parser, AST, semantic frontend, IR generator, or source-string
  execution path;
- production Zend ABI compatibility;
- a complete PHP standard library;
- FPM/SAPI, OPcache, preload, or shared native-code lifecycle;
- broad speculative optimization for references, destructors, generators,
  fibers, exceptions, traits, enums, dynamic includes, eval, or autoload;
- default-on JIT behavior;
- wall-clock-only performance claims.

## Risk Register

| Risk | Required mitigation |
| --- | --- |
| Cranelift API stability | Keep Cranelift dependencies optional, pinned through Cargo, and isolated behind `php_jit` backend traits. Use no-exec and CLIF verifier gates before native execution. |
| JIT memory ownership | Keep executable memory absent until a small owner type, lifecycle tests, and W^X or equivalent policy are documented and audited. |
| ABI unsafety | Use `repr(C)` boundary types for VM handles, values, callouts, and exits. Do not pass raw Rust references into native code. |
| Side exits | Every guard failure, unsupported value, helper failure, and stale metadata case must return a structured side-exit reason and resume the interpreter at a known safe point. |
| Platform support | Unsupported hosts must skip or fall back cleanly. Standard Phase 7 verification must not require native JIT support. |
| Semantic drift | Every optimized fixture must pass `--jit=off` versus `--jit=on` comparisons before any performance report is accepted. |
| Benchmark noise | Machine-readable reports must store environment metadata, feature flags, counters, and correctness status. Wall-clock data stays advisory unless paired with stable correctness evidence. |

## Big-Win Evidence Policy

Each big-win path is complete only when it has:

- a stable eligibility rule with rejection reason coverage;
- at least one focused fixture;
- an off/on differential run;
- counters for attempts, compile success or skip, execution, guard exits,
  interpreter resumes, and blacklisting where relevant;
- a row in the Cranelift big-win report schema;
- documentation of remaining known gaps.

## Consequences

The addendum can incrementally grow native-code coverage without changing the
Phase 7 default behavior. It also creates a stricter stop rule: if a prompt
cannot prove correctness with the relevant diff and smoke gates, the next prompt
must not start.
