# ADR-0026: Runtime semantics GC Skeleton and Root Tracking

## Status

Accepted for Runtime semantics Work items 31 and 32.

## Context

Runtime semantics needs a safe foundation for PHP-like reference counting and later cycle
collection. The current runtime uses Rust-owned storage:

- arrays use copy-on-write `Rc<ArrayStorage>`;
- objects use stable `ObjectRef` identities backed by `Rc<RefCell<_>>`;
- references use `ReferenceCell` backed by `Rc<RefCell<Value>>`;
- closures store captured values or reference cells;
- generators and fibers are reserved runtime categories but do not yet expose
  suspended stacks in the VM.

Executing collection from Rust destructors would be unsafe for this runtime
shape, so Work item introduces observation and candidate detection rather than
freeing storage.

## Decision

Add `php_runtime::gc` as a deterministic debug/test API:

- `GcRoot` records a root kind, debug name, and effective value;
- `scan_roots()` walks values into a `GcSnapshot`;
- `GcEntityId` and `GcNode` record arrays, objects, references, closures, and
  reserved generator/fiber/string categories;
- `GcNode::refcount_estimate` exposes Rust `Rc` strong counts when available;
- cycle candidates are entities that can reach themselves through scanned
  edges.

The VM contributes roots from:

- frame registers;
- frame locals, after dereferencing reference slots through the existing slot
  API;
- static locals;
- static properties and enum-case objects owned by class-table state;
- destructor queue entries.

Generator and fiber stack roots are reserved in `GcRootKind` and remain empty
until those runtime stacks exist.

Work item adds internal weak handles and a tracked-heap test hook:

- arrays, objects, and reference cells expose weak debug handles;
- `GcTrackedHeap::track_value()` records weak handles reachable from explicit
  test values;
- `GcTrackedHeap::collect_cycles()` scans the supplied roots and clears
  outgoing edges for unrooted objects and reference cells;
- clearing object properties and resetting reference cells to `Uninitialized`
  is an internal collection action, not PHP-visible `unset()` behavior.

## Invariants

- GC scanning must not mutate runtime values.
- GC scanning must not execute PHP code.
- GC scanning must not panic on self-referential object/reference graphs.
- Debug IDs are process-local and are not PHP-visible handles.
- No `unsafe` blocks are required for the Work item skeleton.
- Cycle candidates are diagnostic metadata. Work item collection may only break
  unrooted object/reference cycles through internal test hooks.
- Destructors are not run by the GC test hook. Destructor scheduling remains
  owned by the VM `DestructorQueue`, which still guarantees one queued
  destructor execution per queue residency.

## Consequences

The VM can now prove root tracking and cycle-candidate discovery in tests, and
the runtime can prove simple unrooted object cycles and reference-mediated
array cycles are not permanently retained by the internal hooks. Visible PHP
behavior must continue to treat public `gc_*` functions, Zend-compatible cycle
collection counts, cyclic destructor timing, and exact refcount-triggered
lifetime as known gaps until they are implemented and diff-tested.

## Alternatives Considered

- Add public `gc_collect_cycles()` semantics immediately. Rejected because
  Zend-compatible counts, destructor timing, and weak-reference behavior are
  not specified for this runtime yet.
- Depend on Rust reference counts alone. Rejected because PHP-visible cycles
  through arrays, objects, and references need explicit root/candidate
  reasoning.
- Add a tracing collector that mutates live runtime values during normal VM
  execution. Rejected for Runtime semantics; mutation is limited to deterministic test
  hooks until public semantics are specified.

## Standard library Handoff

Standard library should turn the debug root model into public GC behavior only after
destructor ordering, weak references, suspended generator/fiber stacks, and
`gc_status()`/`gc_collect_cycles()` result compatibility are specified and
diff-tested.
