# ADR 0021: Runtime Object MVP

## Status

Accepted

## Context

PHP object semantics include class tables, visibility, inheritance,
interfaces, traits, enums, hooks, magic methods, autoloading, and object
identity. Runtime needs a small executable object core for real runtime smoke
coverage.

## Decision

Implement concrete class entries, `new`, constructors, public properties,
public instance methods, simple public static methods, shallow clone, and
public-property clone-with. Store objects as identity-bearing handles with a
public property map.

## Consequences

- Basic object programs execute through the same VM as scalar and array code.
- Visibility, readonly, inheritance, traits, enums, hooks, magic methods,
  dynamic object access, and autoload remain explicit known gaps.
- Runtime semantics object work should extend this model with compatibility tests before
  adding framework claims.
