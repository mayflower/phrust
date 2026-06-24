# ADR 0019: Runtime Runtime Value Representation

## Status

Accepted

## Context

Executable Runtime fixtures need scalar values, arrays, objects, callables,
and simple references. Full Zend zval behavior, copy-on-write, resources, and
extension values are out of scope.

## Decision

Represent runtime values with a Rust `Value` enum plus byte strings,
insertion-ordered arrays, object handles, callable descriptors, and
`ReferenceCell`/`ValueSlot` for the local-reference MVP.

## Consequences

- Simple executable PHP programs can run without pretending to implement zval
  storage compatibility.
- Reference/COW behavior remains explicit and fixture-backed as a Runtime semantics
  known gap.
- Standard-library and extension work must add value forms deliberately rather
  than smuggling them into scalar conversions.
