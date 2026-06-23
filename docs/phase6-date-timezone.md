# Phase 6 Date/Time MVP

Reference target: PHP 8.5.7 (`php-8.5.7`).

Prompts 06.34 and 06.35 start Date/Time with a Rust-owned abstraction rather
than timelib FFI. The runtime exposes a deterministic timezone registry,
request-local default timezone state, core date functions, and DateTime-like
runtime object helpers through:

- `date_default_timezone_get`
- `date_default_timezone_set`
- `timezone_identifiers_list`
- `date`
- `time`
- `strtotime`
- `DateTimeInterface`, `DateTime`, `DateTimeImmutable`, and `DateInterval`
  metadata in `php_std`
- `DateTimeZone` class metadata in `php_std`

The initial registry intentionally covers `UTC`, `Europe/Berlin`, and a small
set of common package-facing identifiers. It does not read host `TZ`, platform
timezone databases, or locale state.

The DateTime helper layer stores timestamps and timezone identifiers as runtime
object properties and covers constructor-style creation, `format`,
`getTimestamp`, `setTimestamp`, `setTimezone`, `modify`, `add`, `sub`, and
`diff` MVP behavior through internal helper functions. Mutable helpers update
`DateTime` in place; immutable helpers return a new `DateTimeImmutable` object.
`DateInterval` stores an MVP signed second delta.

`strtotime` accepts integer timestamps, ISO-like absolute strings such as
`2024-01-02 03:04:05`, and restricted relative modifiers such as `+2 days`.
Unsupported natural-language forms return deterministic failure instead of
guessing.

## Strategy

ADR-0065 permits narrow PCRE2/tzdata-style dependencies. This slice keeps the
Date/Time boundary dependency-free and leaves room for a later tzdb crate or
timelib FFI behind the same runtime abstraction.

## Known Gaps

The following gaps are tracked in `docs/known-gaps-phase6.md`:

- `PHASE6-GAP-DATE-TIMELIB-PARITY`
- `PHASE6-GAP-DATE-TIMEZONE-VM-PERSISTENCE`
- `PHASE6-GAP-DATETIME-METHOD-DISPATCH`
- `PHASE6-GAP-DATETIME-TZDB-DST`
