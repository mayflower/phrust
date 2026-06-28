# Persistent Type Feedback and Invalidation

FPE-20 adds the first engine-owned persistent feedback contract. The current
implementation is advisory and default-off: metadata can be loaded, validated,
and reported, but accepted entries do not change VM execution.

## Key Model

Every feedback entry is keyed by:

- source fingerprint from the same cache-fingerprint machinery used by the
  bytecode-cache envelope;
- engine version and PHP target version;
- compile options, including opt level, execution format, quickening, inline
  caches, bytecode cache mode, JIT mode, and tiering mode;
- function ID and instruction ID;
- IR fingerprint over the current IR snapshot;
- class-table, function-table, autoload, and include-path epochs;
- target architecture/config label.

Any mismatch rejects the entry as stale. Stale entries are counted and the run
continues through baseline execution.

## Metadata Only

The persistent payload can represent:

- monomorphic, polymorphic, megamorphic, and blacklisted callsite state;
- observed scalar operand kinds;
- array layout and key-shape summaries;
- object class/layout/property-slot observations;
- branch bias;
- include/autoload target stability;
- guard-failure and blacklist summaries.

The parser rejects explicit userland value state: VM `Value`s, object handles,
array values, resource handles, and non-interned request strings. Interned or
engine-owned immutable strings are the only string payload class accepted by the
line-format validator.

## CLI Reporting

The VM CLI exposes the validation path without leaking data into PHP stdout:

```bash
php-vm run \
  --persistent-feedback-read target/performance/feedback/input.pff \
  --persistent-feedback-stats-json target/performance/feedback/stats.json \
  fixtures/runtime/valid/hello.php
```

The stats JSON records advisory/default-off status, accepted entries, stale
entries, corrupt entries, rejected userland-state entries, metadata bytes, and
whether execution fell back to baseline.

## Matrix Policy

The acceleration matrix includes `persistent-feedback-advisory` only with
`--include-persistent-feedback` or
`PHRUST_ACCEL_MATRIX_PERSISTENT_FEEDBACK=1`.

The fastest-engine matrix includes `phrust-persistent-feedback-optional` only
with `--include-persistent-feedback` or
`PHRUST_FASTEST_MATRIX_PERSISTENT_FEEDBACK=1`.

Both rows are optional/default-off. They exercise metadata validation and stats
reporting, then compare PHP stdout, diagnostics, and exit status against the
baseline row.

## Remaining Work

- persist accepted metadata through an owned engine cache writer;
- integrate accepted feedback with quickening, inline caches, and later tiers;
- model non-zero class/function/autoload/include epochs from a persistent engine
  context rather than the current cold-start zero epoch;
- add Composer map fingerprints when the autoload graph model is promoted from
  request-local runtime behavior into persistent engine metadata.
