# ADR 0012: HIR, Symbol IDs, and Interning

## Status

Accepted

## Context

Phase 3 needs stable semantic identities for declarations, names, scopes, HIR
nodes, types, and attributes. CST identity alone is not enough for semantic
diagnostics or Phase 4 handoff.

## Decision

`php_semantics` owns typed IDs, arenas, interned names, symbol tables, scope
tables, HIR tables, and source maps. IDs are stable within one analysis result
and are serializable for snapshots.

## Consequences

- Semantic output can be inspected without walking raw CST nodes.
- Phase 4 can consume HIR and semantic metadata directly.
- IDs are not promised to be stable across separate analysis runs unless a
  later incremental layer defines that contract.
