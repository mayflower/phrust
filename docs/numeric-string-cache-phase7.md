# Phase 7 Numeric-String Classification Cache

Prompt 07.39 adds a conservative runtime cache under the existing
numeric-string classifier. The raw classifier remains the source of truth; the
cache stores only classification results, never diagnostics or converted
operation results.

## Scope

The cached wrapper is used by scalar conversions and comparisons:

- explicit int and float casts,
- arithmetic conversion through `to_number`,
- PHP 8 style loose comparisons between strings and numbers,
- string/string numeric comparisons.

Array key normalization remains separate because PHP array-key conversion is a
different rule set: decimal integer strings without leading plus and without
leading zeroes become integer keys, while whitespace and float-looking strings
do not. Tests cover that this behavior remains unchanged.

## Cache Key

The request-local key is:

- string storage identity,
- byte length,
- stable byte fingerprint.

Including the fingerprint keeps the cache safe for both copy-on-write mutation
and unique in-place byte mutation. Changed bytes cannot reuse stale
classification even if the allocation identity and length are unchanged.

The cache is intentionally bounded and clears itself when it reaches the small
Phase 7 limit. That avoids unbounded retention while keeping hot-loop repeated
strings visible as hits.

## Counters

When VM counters are enabled, each execution resets the numeric-string cache and
exports:

- `numeric_string_cache_hits`
- `numeric_string_cache_misses`

The counters are harvested after execution, including runtime-error exits, so a
non-numeric arithmetic error still reports the classification miss that produced
the error without caching or delaying the diagnostic.

## Semantics

The cache stores `NumericStringKind` and parsed numeric payloads for the existing
Phase 5/6 classifier:

- non numeric,
- integer numeric,
- float numeric,
- leading numeric,
- whitespace-trimmed full numeric,
- integer overflow that falls back to float classification.

Locale is not consulted. The classifier remains byte-oriented and deterministic.
