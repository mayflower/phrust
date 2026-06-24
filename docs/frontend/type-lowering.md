# Type Lowering

Type lowering converts AST type syntax into semantic type records. It does not
perform runtime type checks.

The lowered records live in `HirModule::types()` and are source mapped by
`TypeId`. JSON output includes each type ID, source context, source spelling,
span, shape, child type IDs for composite types, and resolved class-like names
when resolution is static.

## Type Forms

- `Named` class-like types, resolved with namespace/import rules
- `Builtin`: `array`, `callable`, `object`, `iterable`, `bool`, `int`,
  `float`, `string`
- `Nullable`: stores the inner `TypeId` and records normalization to
  `T|null`
- `Union`: stores member `TypeId`s in source order
- `Intersection`: stores member `TypeId`s in source order
- `Dnf`: stores member `TypeId`s when union terms include intersections
- special atoms: `self`, `parent`, `static`, `mixed`, `never`, `void`,
  `null`, `false`, `true`

## Contexts

The implemented `TypeContext` values are:

- `parameter`
- `return`
- `property`
- `class_constant`
- `closure_use`
- `catch`
- `enum_backing`

Context controls diagnostics such as `void` outside return position, `never`
outside return position, `callable` property types, and
`self`/`parent`/`static` outside a class-like context.

Duplicate and impossible combinations are semantic diagnostics.

## Fixture Coverage

`fixtures/semantic/types/` covers parameter, return, property, union,
intersection, DNF, invalid property `void`, invalid parameter `never`, valid
class-like `static` return, and invalid top-level `self`.

The type fixtures currently match the pinned PHP 8.5.7 acceptance oracle. No
type-lowering known gaps are recorded.
