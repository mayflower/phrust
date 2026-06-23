# ADR-0063: Phase 6 Streams and Capabilities

## Status

Accepted for Phase 6.

## Context

Streams and filesystem APIs are necessary for Composer and framework smoke
tests, but unrestricted host, network, and process access would make tests
nondeterministic and unsafe.

## Decision

Phase 6 supports deterministic local streams and filesystem operations behind a
resource table. Network, TLS, curl, openssl, and process execution are
default-off capabilities.

## Consequences

Unsupported capabilities produce deterministic diagnostics or documented known
gaps instead of implicit host access.
