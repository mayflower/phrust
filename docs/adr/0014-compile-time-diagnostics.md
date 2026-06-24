# ADR 0014: Compile-Time Diagnostics

## Status

Accepted

## Context

The parser reports syntax errors, but PHP also rejects many programs during
compile-time semantic checks. Combining those diagnostics would make parser
acceptance harder to compare with the PHP lint oracle.

## Decision

Semantic frontend introduces a separate semantic diagnostic type with stable diagnostic
IDs, severity, phase, byte spans, labels, and related information. Parser
diagnostics are included in frontend results but are not reclassified as
semantic diagnostics.

## Consequences

- Fixture snapshots can assert semantic behavior directly.
- Parser tests remain focused on syntax.
- PHP reference text may drift without invalidating stable diagnostic IDs.
