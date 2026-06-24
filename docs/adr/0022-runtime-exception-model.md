# ADR 0022: Runtime Exception Model

## Status

Accepted

## Context

Runtime needs `throw`, `try`, `catch`, and `finally` behavior for executable
fixtures, but full PHP `Throwable`, `Exception`, `Error`, stack traces, and
engine wording are larger compatibility tasks.

## Decision

Use VM handler state for `EnterTry`, `LeaveTry`, `Throw`, and `EndFinally`.
Represent `new Exception("message")` as a VM-internal object shape with a
public `message` property and deterministic uncaught diagnostics.

## Consequences

- Core throw/catch/finally control flow is executable and testable.
- Catch-type matching beyond the MVP and PHP stacktrace formatting remain
  known gaps with stable IDs.
- Runtime semantics can replace or extend the internal exception object without changing
  parser or semantic frontend boundaries.
