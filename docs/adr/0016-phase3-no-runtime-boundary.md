# ADR 0016: Phase 3 Runtime Boundary

## Status

Accepted

## Context

Semantic frontend work can easily expand into execution semantics, autoloading,
include/eval behavior, attributes, and runtime value modeling.

## Decision

Phase 3 stops at semantic frontend output. It does not execute PHP files,
instantiate attributes, run include/require/eval, invoke autoloaders, create
runtime values, generate bytecode, dispatch VM opcodes, emulate Zend ABI, or
load extensions.

## Consequences

- Phase 3 remains auditable against `php -l` and deterministic fixtures.
- Runtime-dependent behavior is marked as deferred metadata or known gaps.
- Phase 4 receives structured HIR and semantic metadata as its input contract.
