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
array values, resource handles, non-interned request strings, and — as of the
writer-accounting slice — globals, superglobals, output buffers, and sessions.
Interned or engine-owned immutable strings are the only string payload class
accepted by the line-format validator.

## CLI Reporting

The VM CLI exposes the validation path without leaking data into PHP stdout:

```bash
php-vm run \
  --persistent-feedback-read target/performance/feedback/input.pff \
  --persistent-feedback-stats-json target/performance/feedback/stats.json \
  fixtures/runtime/valid/hello.php
```

The stats JSON (schema v2) records advisory/default-off status, accepted
entries, entries written by the engine-owned writer, and rejection counts split
by cause — stale (source/engine/PHP-target/IR identity), epoch mismatch,
architecture mismatch, config mismatch, corrupt, and userland-state — plus
metadata bytes and whether execution fell back to baseline. Splitting the former
single `rejected_stale` counter lets an operator tell an out-of-date deployment
(config/arch/epoch) apart from a genuinely stale source.

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

## Writer Accounting (current slice)

`PersistentFeedbackContext::render_sites_counted` is the engine-owned writer: it
emits only validator-accepted entries and returns how many it wrote, which the
CLI records as `entries_written`. Emitted entries carry the context's epochs, so
a writer fed real epochs would persist non-zero epochs. Consumption stays
default-off; the writer only widens metadata coverage and accounting.

## Remaining Work

- **Capture non-zero epochs.** The `ExecutionState` class/function/autoload/
  include epochs (`vm/mod.rs`) are request-local and dropped when `execute`
  returns, so the write context still uses the cold-start zero epoch (a
  conservative value that rejects as `epoch_mismatch`, never optimistic reuse).
  The plumbing is: stash the final epochs out of `execute`, surface them on
  `PhpExecutionOutput`, and build the write context with them.
- persist the full accepted payload (callsite/scalar/array/object/branch/
  include-autoload observations), not just the quickening sub-field, once the VM
  produces those observations;
- integrate accepted feedback with quickening, inline caches, and later tiers;
- add Composer map fingerprints when the autoload graph model is promoted from
  request-local runtime behavior into persistent engine metadata.
