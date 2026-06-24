# ADR 0020: Runtime Array MVP

## Status

Accepted

## Context

PHP arrays carry list, map, insertion-order, key-conversion, reference, and
copy-on-write semantics. Runtime needs enough behavior for literals, dimension
operations, foreach, and selected builtins without claiming full parity.

## Decision

Use an insertion-ordered `PhpArray` with integer and string keys. Implement
literal insertion, fetch, assign, append, `isset`, `empty`, `unset`, packed
facade access for variadics, and by-value foreach snapshots. Reject or mark as
known gaps the wider key-conversion, spread, Traversable, element-reference,
and copy-on-write matrices.

## Consequences

- Array behavior is stable for the curated fixture set.
- Runtime semantics must handle COW and element references before claiming broad PHP
  array compatibility.
- Reference-diff reports should treat unsupported array edge cases as known
  gaps, not regressions.
