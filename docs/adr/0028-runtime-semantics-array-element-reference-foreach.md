# ADR-0028: Runtime semantics Array Element References and Foreach

## Status

Accepted for Runtime semantics.

## Context

PHP arrays combine ordered-map behavior, Copy-on-Write, element lvalues, and
iteration rules. `foreach` needs different behavior for by-value snapshots,
by-reference local arrays, objects with public properties, and Traversable-like
objects. Array element references also need to write through to the owning array
while respecting by-value COW separation.

## Decision

Runtime semantics keeps `PhpArray` as the ordered-map storage boundary and routes array
mutation through public APIs such as `insert`, `append`, `get_mut`, and
`remove`. These APIs separate shared storage before write.

Direct array-element references are represented by reference cells stored in
array element values after the array payload has been separated for the write.
By-value foreach over arrays uses a snapshot of key/value pairs. By-reference
foreach over local arrays iterates writable local storage and binds the target
value variable to each element reference in sequence.

Object foreach is handled outside `PhpArray`: public-property iteration,
`Iterator`, and `IteratorAggregate` dispatch are VM iteration sources, not
array conversions.

## Alternatives Considered

- Convert all foreach sources to arrays. This would lose object-property and
  Traversable dispatch behavior and hide SPL gaps.
- Keep by-reference foreach as a known gap until full Zend arrays exist. That
  blocked too much reference/COW validation.
- Expose raw array internals to the VM. That would make future packed/mixed
  optimization observable and fragile.

## Consequences

The implementation can validate the important visible cases while keeping
packed versus mixed layout private. The remaining risks are the full mutation
matrix during iteration, temporary by-reference sources, `ArrayAccess`, SPL
iterator classes, and append-index overflow behavior.
