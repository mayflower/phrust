# ADR 0011: Typed AST Views over CST

## Status

Accepted

## Context

Phase 2 produces a lossless CST. Semantic analysis needs structured access to
declarations, statements, expressions, names, types, and attributes without
moving semantic rules into the parser.

## Decision

Phase 3 introduces `php_ast` as typed, read-only views over `php_syntax` CST
nodes and tokens. The views do not re-lex, reparse, own a second syntax tree,
or evaluate source. They preserve CST byte spans and expose optional children
for recovery-safe lowering.

## Consequences

- Parser and semantic layers remain independently testable.
- Semantic lowering can evolve without changing CST construction.
- Malformed CST regions must be represented defensively instead of causing
  panics.
