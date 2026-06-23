# ADR-0060: Phase 6 Standard Library Scope

## Status

Accepted for Phase 6.

## Context

Phase 5 produced a runtime capable of executing a meaningful PHP 8.5.7 subset,
but framework and Composer compatibility now depends on standard-library,
extension metadata, streams, SPL, Reflection, and diagnostics breadth.

## Decision

Phase 6 implements a deterministic offline MVP of PHP 8.5.7 standard-library
behavior. The required path covers core functions, JSON, PCRE, Date/Time, SPL,
Reflection, tokenizer, streams, filesystem, Composer-local autoloading, and
Composer platform checks.

## Consequences

The VM and runtime may receive small integration hooks, but Phase 6 does not
rewrite lexer, parser, HIR, IR, VM, or existing runtime contracts.
