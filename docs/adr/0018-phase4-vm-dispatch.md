# ADR 0018: Phase 4 VM Dispatch

## Status

Accepted

## Context

The VM must execute the Phase 4 IR predictably while preserving source-backed
diagnostics and avoiding hidden parser or runtime side effects.

## Decision

Use an interpreter loop over basic blocks and register instructions. Calls,
returns, try/catch/finally, includes, and `$this` bindings are handled through
explicit frame and handler state. The VM validates IR by default and uses a
step guard for runaway execution.

## Consequences

- Runtime behavior is deterministic enough for fixture and diff reports.
- The VM stays decoupled from parser internals and does not include a second
  PHP parser.
- Future performance work can add dispatch optimizations only after preserving
  the current diagnostics, trace, and fixture behavior.
