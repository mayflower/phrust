# ADR-0029: Runtime semantics Object Model, Traits, Enums, and Hooks

## Status

Accepted for Runtime semantics.

## Context

Runtime semantics needs enough object semantics for framework-style PHP: inheritance,
visibility, late static binding, properties, magic methods, traits, interfaces,
enums, attributes, Reflection metadata, and property hooks. At the same time,
the project boundary forbids a second frontend or Zend ABI emulation.

## Decision

Runtime class metadata is stored in `php_runtime::ClassEntry` and consumed by
`php_vm`. Class and member declarations are lowered from Semantic frontend HIR into IR
metadata, then into runtime class tables. Method bodies continue to execute
through the same VM call-frame machinery as free functions.

The VM implements hierarchy lookup, visibility checks, late static binding
frame metadata, declared and static property storage, readonly and typed
property checks, property magic, method magic, cloning, traits, interfaces,
enums, attributes, Reflection metadata, and fixture-covered property hooks as a
single object layer.

Property hooks are lowered as synthetic method-like functions and dispatched by
the normal VM call path with `$this` and class scope set. Accessible hooks run
before ordinary backing storage; missing or inaccessible properties still use
magic-property fallback.

## Alternatives Considered

- Treat traits, enums, hooks, and Reflection as separate ad hoc VM paths. That
  would duplicate class metadata and make Standard library linking harder.
- Desugar hooks and traits only in syntax. That would hide source spans and
  semantic diagnostics from later layers.
- Implement Zend object handlers or extension ABI compatibility now. That is
  outside Runtime semantics and would couple the VM to APIs it cannot yet support.

## Consequences

The object layer is coherent enough for Standard library standard-library and Composer
work to reuse. Its main liabilities are breadth and exactness: SPL method
surfaces, full variance rules, property-hook inheritance matrices, doc comments,
serialization magic, and extension objects still need explicit implementation.
