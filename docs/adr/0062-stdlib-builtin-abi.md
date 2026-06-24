# ADR-0062: Standard library Builtin Function ABI

## Status

Accepted for Standard library.

## Context

Runtime semantics builtins are implemented directly in runtime/VM code. Standard library needs a
larger builtin surface with common argument validation, diagnostics, output,
request context, and by-reference metadata.

## Decision

Builtin functions use a narrow Rust ABI with call context, positional/named
arguments, arginfo, return values, diagnostics, output, request context, and
capability access.

## Consequences

The ABI becomes the boundary for standard-library crates and keeps PHP-visible
behavior testable through differential fixtures.
