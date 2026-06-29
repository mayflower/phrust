# Guard, Snapshot, Side-Exit, And Resume Table

The guard/snapshot/resume table v2 makes speculative metadata explicit for
quickening, inline caches, dense bytecode, region IR, and future native tiers.
It is report-only metadata. It does not perform native deoptimization, OSR, or
machine-label patching.

## Guards

Each guard records:

- guard id and guard kind;
- source function and bytecode offset;
- optional IR/source span;
- owning tier or feature;
- snapshot id;
- interpreter resume point;
- exit reason;
- counter id;
- dequickening or blacklist policy.

Initial guard kinds cover integer arithmetic, property-shape checks,
packed-array checks, builtin-call checks, quickening type checks, inline-cache
shape checks, and region assumptions.

## Snapshots

Snapshots list live VM slots and the PHP state that must be restored before the
interpreter resumes. Each entry is expressed in VM-owned live-value metadata
rather than raw pointers. Snapshot records also mark foreach state,
exception/try/finally state, output-buffer state, and reference/COW poison.

Reference/COW poison, rejected foreach state, and rejected try/finally state are
validation failures for current optimized paths. Future tiers may add precise
support before changing those policies.

## Side Exits

Shared exits carry a stable exit label id, exit reason, snapshot restore plan,
and resume bytecode offset. They intentionally do not contain machine labels.
That keeps the metadata useful for quickening and inline-cache fallback today
while preserving a shape that future native tiers can consume.

## JSON Schema

`ResumeTable::to_json()` emits a compact versioned schema:

- `schema_version`: currently `2`;
- `guards`: guard id, kind, tier, snapshot, resume offset, reason, policy;
- `snapshots`: snapshot id, entry count, reference/COW poison marker;
- `exits`: exit id, reason, snapshot.

This schema is designed to be embedded under tiering, JIT, or performance stats
without changing PHP-visible execution.
