# ADR 0023: Runtime Include Model

## Status

Accepted

## Context

Include/require behavior is a high-risk PHP compatibility surface involving
relative paths, include_path, stream wrappers, current scope, symbol side
effects, and filesystem policy.

## Decision

Compile included files through the existing frontend and execute them in the
same VM. Resolve paths relative to the including file, canonicalize them, and
allow only configured include roots. Track `include_once` and `require_once`
by canonical path. Missing `include` warns and continues; missing `require`
is fatal.

## Consequences

- Includes do not introduce a second parser or ad hoc evaluator.
- Local include graphs are executable while filesystem and stream-wrapper
  behavior stays deterministic.
- Full include_path, stream, resource, and cross-file symbol compatibility are
  Runtime semantics known gaps.
