# ADR-0781: Backend-Neutral JIT API

## Status

Accepted for the Phase 7 Cranelift addendum.

## Context

The Phase 7 JIT experiment originally exposed a small `JitEngine` and optional
Cranelift lowering helper from `php_jit`. That was enough for the default-off
experiment, but the Cranelift big-wins addendum needs the VM and future tooling
to speak to a backend-neutral contract. Otherwise each new prompt would leak
Cranelift-specific assumptions into tiering, counters, side exits, and runtime
helpers.

## Decision

`php_jit` owns the backend-neutral API:

- `JitBackendApi`: trait implemented by concrete backend adapters.
- `JitBackendCompileRequest`: stable compile request boundary carrying region
  metadata, optional IR context, optional function id, and runtime native-entry
  permission.
- `JitBackendCompileOutcome`: backend result boundary carrying status,
  optional opaque handle, and stable diagnostics.
- `NoopJitBackend`: default-off backend for feature-light builds.
- `CurrentJitBackend`: build-selected adapter that currently resolves to the
  no-op backend or the feature-gated Cranelift experiment.

`JitEngine::compile` now routes through this boundary. `JitEngine` remains
responsible for runtime switches and counters; backend implementations remain
responsible for deciding whether a region can compile.

## Invariants

- Backend APIs must not panic for unsupported input.
- Compiled handles are opaque and are not raw executable pointers.
- Runtime `--jit=off` blocks backend calls before compile.
- Feature-off builds use `NoopJitBackend` and do not require Cranelift.
- Feature-on builds can still reject native execution when runtime permission
  or safety gates are absent.
- Diagnostics must use stable, machine-readable statuses for smoke scripts and
  reports.

## Consequences

The VM can depend on `php_jit` request/result types without importing
Cranelift modules. Later prompts can add Cranelift execution, side exits, helper
symbols, compile caches, and reports behind the trait without changing the
interpreter source-of-truth rule.
