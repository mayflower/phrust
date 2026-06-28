# Runtime Layout Compactness

Date: 2026-06-28.

This note audits the current runtime value, container, and request-local
allocation layout for the fastest-engine FPE-11 slice. It records first safe
wins only; it does not change the public `Value` shape or introduce a new
runtime ownership model.

## Current Inventory

- `Value` is the VM carrier enum. Scalars are inline, while strings, arrays,
  objects, resources, fibers, generators, callables, and reference cells are
  handle-backed variants.
- `PhpString` stores bytes in `Rc<Vec<u8>>`. Assignment clones the handle and
  shares bytes until `separate_for_write` or `bytes_mut` applies COW. Literal
  string interning is request-local in the VM literal pool, and numeric-string
  classification has a request-local cache keyed by storage identity plus byte
  validation.
- `PhpArray` stores ordered array state in `Rc<ArrayStorage>`. The storage keeps
  entries, next append key, packed-length metadata, internal pointer state, and
  a mutation epoch. Mutating shared storage separates with `Rc::make_mut`.
- `ObjectRef` stores object state in `Rc<RefCell<ObjectStorage>>`. The storage
  currently keeps property values, insertion order, display/debug names, and
  class identity metadata.
- `ReferenceCell` stores aliases in `Rc<RefCell<Value>>`. Reads clone the
  contained value, while writes preserve alias identity through the cell.
- VM call frames and register/local files are request-local structures. The VM
  already reuses conservative completed frames and reports frame/register reuse
  counters.
- The VM literal pool interns request-local constants and records
  `literal_intern_hits` and `literal_intern_misses`.

## Visible Pressure Points

- `Value` clones are frequent because helper paths use value ownership as the
  safe default. That is correct but can amplify handle clone traffic.
- String creation allocates a new `Vec<u8>` for each non-interned byte string.
  Small strings, substrings, and conversion outputs are future candidates, but
  conversion order and binary exactness block broad rewrites.
- Arrays use one ordered vector representation for packed and mixed shapes.
  Packed metadata lets fast paths avoid duplicating layout knowledge, but mixed
  lookup and mutation still operate through the general ordered storage.
- Object property storage is hash-map based and carries debug labels/order
  metadata needed for diagnostics, dumps, dynamic properties, and reflection
  behavior.
- COW separations are PHP-visible through references, foreach behavior, object
  destructors reachable from values, and mutation order. Any shortcut must keep
  aliasing and destructor timing unchanged.
- Reference cells are unavoidable for explicit aliases and by-reference
  behavior. Optimizations need alias classification before replacing generic
  cells with specialized storage.

## Counters

FPE-11 adds request-local runtime layout counters that are reset and collected
with VM counters:

- `value_clones`
- `string_allocations`
- `array_handle_clones`
- `cow_separations`
- `reference_cell_creations`
- `object_allocations`

Existing allocation-adjacent counters remain part of the same picture:

- `frame_allocations`, `frame_reuses`
- `frames_allocated`, `frames_reused`
- `register_files_allocated`, `register_files_reused`
- `literal_intern_hits`, `literal_intern_misses`

The performance report, framework smoke focus, acceleration matrix focus, and
hot-path inventory now surface these counters so future layout work can be
ranked from deterministic local evidence.

## First Safe Win

`PhpArray::from_packed` now builds exact packed storage directly from the input
vector rather than constructing an empty array and appending each element. This
keeps the same keys, packed metadata, internal pointer behavior, append key, and
mutation semantics while avoiding repeated mutation through the generic append
path for known packed literals.

This change is local and reversible. It does not split the array storage type,
change public runtime APIs, or make packed/mixed layout a VM concern.

## Correctness Blockers For Future Work

- References and by-reference parameters can make ordinary-looking value paths
  alias-sensitive.
- COW separation timing must preserve mutation, foreach, and diagnostic order.
- Destructors and shutdown ordering can observe when object-containing values
  become unreachable.
- Magic methods, property hooks, dynamic properties, typed and readonly
  properties, and reflection constrain object layout changes.
- Generators, fibers, includes, eval, autoload, resources, globals, and output
  buffers have request-lifetime behavior that blocks broad arena allocation
  until teardown is modeled explicitly.

## Future Candidates

- Audit hot `Value` clone sites and replace local redundant clones only where
  ownership can be proven without changing diagnostics or alias behavior.
- Split array storage into packed and mixed representations behind `PhpArray`
  after packed/mixed transition counters and fixtures justify it.
- Add object-shape storage for declared-property-only classes while preserving
  dynamic properties, hooks, visibility, typed property state, and reflection.
- Research small-string or inline-string storage behind `PhpString`; keep
  byte-exact conversions and COW semantics as the boundary.
- Defer request arenas to the dedicated request-arena work item, where
  destructor, resource, global, include, generator, and fiber teardown can be
  validated as one lifetime model.
