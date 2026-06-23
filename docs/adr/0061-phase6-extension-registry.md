# ADR-0061: Phase 6 Extension Registry

## Status

Accepted for Phase 6.

## Context

Composer platform checks and Reflection need stable metadata for internal
functions, classes, constants, and extensions.

## Decision

Phase 6 introduces a Rust-owned extension registry that exposes deterministic
metadata for implemented and stubbed internal surfaces. Generated metadata may
be added only when the generator is deterministic and reviewable.

## Consequences

`extension_loaded`, `get_loaded_extensions`, Reflection, and coverage reports
share one registry instead of duplicating tables across the CLI and runtime.
