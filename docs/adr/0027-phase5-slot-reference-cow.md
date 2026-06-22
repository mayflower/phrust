# ADR-0027: Phase 5 Slot, Reference, and Copy-on-Write Model

## Status

Accepted for Phase 5.

## Context

PHP variables are storage locations, not just values. By-reference assignment,
by-reference parameters and returns, closure captures, array element lvalues,
`global`, `$GLOBALS`, and Copy-on-Write all require the VM to distinguish a
read value from the location being written.

The Phase 4 VM mostly moved `Value` objects through registers and locals. That
was enough for scalar execution, but it could not represent alias identity or
separate by-value COW sharing from true references.

## Decision

Phase 5 uses three explicit runtime concepts:

- `Value` is the effective PHP value;
- `Slot` is a writable storage location;
- `ReferenceCell` is shared alias storage.

Locals, globals, array elements, and property storage use slots or slot-like
mutation boundaries. Reading a slot dereferences a reference cell. Writing a
reference slot writes through the cell. Creating a reference upgrades the slot
to a cell and binds aliases to that cell.

Strings and arrays use shared payload storage with separation on write.
Copy-on-Write sharing is not reference identity; only `ReferenceCell` creates
alias identity.

## Alternatives Considered

- Put references directly into every `Value` and dereference opportunistically.
  This made temporaries and lvalues indistinguishable and risked accidental
  write-through from registers.
- Keep values unshared and copy arrays/strings eagerly. This avoided COW bugs
  but could not match PHP-visible by-value assignment behavior and would make
  later performance work harder.
- Model Zend zvals exactly. That would overfit Phase 5 to Zend internals before
  the VM has enough execution breadth to justify the complexity.

## Consequences

The model is explicit enough for Phase 6 to add more lvalue kinds without
changing the parser or semantic frontend. It also keeps performance-sensitive
COW behavior localized in `PhpString`, `PhpArray`, and VM mutation helpers.

Object-property references and string-offset writes remain known gaps until
their lvalue APIs can participate in the same slot/reference/COW contract.
