# ADR 0015: Constant Expression Lowering

## Status

Accepted

## Context

PHP validates constant-expression contexts during compilation, but full PHP
expression evaluation requires runtime value semantics that are outside Phase
3.

## Decision

Phase 3 validates whether expressions are allowed in constant-expression
contexts and records symbolic constant-expression metadata. It may fold only
small, deterministic, non-runtime forms. It does not implement zvals, object
creation, function execution, autoloading, or general expression evaluation.

## Consequences

- Constant-expression diagnostics can match PHP acceptance without starting a
  VM.
- PHP 8.5 closure, callable, and cast forms can be represented symbolically.
- Full value compatibility remains a later runtime phase concern.
