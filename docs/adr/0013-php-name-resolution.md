# ADR 0013: PHP Name Resolution

## Status

Accepted

## Context

PHP has separate import and resolution behavior for class-like names,
functions, and constants. Function and constant lookup can defer to runtime
fallbacks.

## Decision

Phase 3 models names by kind and resolves them using current namespace and
import tables. Class-like, function, and constant imports are tracked
separately. Runtime-sensitive fallback is represented explicitly as deferred
metadata rather than hidden as a successful compile-time lookup.

## Consequences

- Semantic fixtures can distinguish compile-time resolution from runtime
  fallback behavior.
- No autoloading or cross-file loading is required.
- Duplicate aliases and invalid imports become semantic diagnostics.
