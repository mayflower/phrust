# ADR-0065: Standard library PCRE and Date Strategy

## Status

Accepted for Standard library.

## Context

PCRE and Date/Time behavior has large compatibility surface. Full PHP parity
requires careful dependency and platform decisions.

## Decision

Standard library permits narrow dependencies for PCRE2/tzdata-style compatibility when
documented in Nix and Cargo. MVP behavior is differential-tested for common
Composer and framework paths; obscure flags and locale/natural-language
parsing edges may remain known gaps.

For Work item, Date/Time starts with a small Rust-owned abstraction rather
than timelib FFI. The first Date/Time slice provides a deterministic timezone
identifier registry, request-local default timezone state, and `DateTimeZone`
metadata. Later `DateTime` parsing/formatting work can either expand this
abstraction with a tzdb crate or introduce timelib FFI behind the same runtime
boundary.

Work item extends that abstraction with deterministic timestamp formatting,
restricted `strtotime` parsing, `DateTime`/`DateTimeImmutable`/`DateInterval`
runtime object helpers, and date/time stdlib metadata. PHP-level object
constructor and method dispatch remains a VM integration gap, not a second
Date/Time implementation.

## Consequences

The project can add practical regex and Date/Time behavior without claiming
complete ext/pcre or ext/date parity.
