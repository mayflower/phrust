# Array Shape Observation

Date: 2026-06-28.

FPE-23 adds runtime-owned metadata for non-packed array shapes without changing
the PHP array representation. The VM observes shapes through `PhpArray`
helpers, records counters, and only uses guarded reads when the helper can
preserve PHP key coercion, insertion order, COW, references, and foreach
semantics.

## Shape Kinds

`PhpArray::shape_metadata()` classifies arrays as:

- `empty`
- `packed`
- `packed_with_holes`
- `small_inline_map`
- `interned_string_key_record`
- `shape_stable_record_like`
- `mixed_hash`
- `shared_immutable_literal_array`
- `cow_or_reference_fallback`

The classifier also exposes length, mutation epoch, sharing, reference
presence, key-kind summary, and numeric-string-key ambiguity. Consumers must use
the metadata and lookup helpers instead of reimplementing array layout checks.

## Guarded Reads

The VM currently uses the helpers for exact, fail-closed `FetchDim` and
one-dimensional `IssetDim` cases:

- interned-string-key and shape-stable record-like array reads
- small inline map reads
- numeric-string-key, insertion-order, mixed-hash, COW, and reference fallbacks

Shared immutable literal arrays are observed for future work, but generic reads
still use the existing array lookup path. Packed arrays continue to use the
packed-array metadata and fast paths from the earlier array slice.

## Counters

VM counters now include:

- `array_shape_observed_by_kind`
- `record_shape_hits`
- `record_shape_misses`
- `small_map_hits`
- `small_map_misses`
- `key_coercion_fallbacks`
- `order_semantics_fallbacks`
- `cow_or_reference_fallbacks`

`scripts/performance/perf_report.py` renders the shape counters and observed
shape map when benchmark or framework-smoke JSON includes them.

## Covered Fixtures

The runtime unit fixtures cover classification and helper fallback behavior.
The VM fixture covers route-param/config/JSON-row/DB-row style maps,
mixed int/string small maps, numeric-string keys, insertion-order fallbacks,
unset/reinsert behavior, COW copies, references, and foreach/order-sensitive
fallback accounting.

## Remaining Work

This slice does not add a new small-map or record storage representation, shape
interning, immutable literal storage, dense bytecode lowering for record
lookups, or native/JIT consumption. Those remain blocked on broader PHPT and
framework evidence plus exact mutation, order, reference, and COW invalidation
proof.
