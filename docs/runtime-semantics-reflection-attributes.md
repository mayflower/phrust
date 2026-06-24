# Runtime semantics Reflection and Attributes

Runtime semantics carries attribute and declaration metadata from the existing frontend
pipeline into runtime class/function tables. Reflection objects are VM metadata
handles over that data; they are not full userland Reflection object
implementations.

## Attribute Metadata

Attribute entries are produced by Semantic frontend semantic lowering, transported
through IR, and stored in `php_runtime::AttributeEntry` lists on classes,
methods, functions, parameters, properties, class constants, and enum cases.

Each entry preserves:

- source-spelled attribute name;
- normalized/resolved name where the frontend has one;
- source span;
- folded constant-expression arguments;
- repeated-on-target marker metadata.

`ReflectionAttribute::getName()` and `getArguments()` read that metadata.
`ReflectionAttribute::newInstance()` remains
`E_PHP_RUNTIME_UNSUPPORTED_ATTRIBUTE_NEWINSTANCE` because it requires attribute
class lookup, constructor invocation, autoload-sensitive errors, and target /
repeatability validation.

## Reflection Metadata

The VM exposes a fixture-covered Reflection MVP for:

- `ReflectionClass`, including class/interface/trait/enum flags, methods,
  properties, constants, attributes, interface names, source file and line
  metadata, and instantiability checks;
- `ReflectionFunction` and `ReflectionMethod`, including attributes,
  parameters, return type, static variables for closures, closure scope, source
  metadata, and visibility/static/abstract/final flags for methods;
- `ReflectionProperty`, `ReflectionClassConstant`, `ReflectionParameter`, and
  `ReflectionNamedType`;
- `ReflectionEnum` and `ReflectionEnumUnitCase` for unit/backed enum metadata;
- `ReflectionFunction` construction from closures and first-class
  user-function callables.

Source locations are best-effort from IR spans. Doc comments currently return
`false`; comment retention is not part of the Runtime semantics IR metadata contract.

Unsupported Reflection methods fail with deterministic diagnostics such as
`E_PHP_VM_UNKNOWN_METHOD` or `E_PHP_VM_REFLECTION_UNSUPPORTED_CALLABLE` instead
of returning plausible but wrong metadata.

## Public APIs

Standard library should build on these existing public data structures rather than
inventing parallel reflection tables:

- `php_runtime::ClassEntry`, `ClassMethodEntry`, `ClassPropertyEntry`,
  `ClassConstantEntry`, `ClassEnumCaseEntry`, and their flag structs;
- `php_runtime::RuntimeType` plus `runtime_type_name()` and
  `value_matches_runtime_type()`;
- `php_runtime::AttributeEntry`;
- `php_runtime::CallableValue` and `ClosureCaptureValue` for callable and
  closure reflection;
- VM class/function/constant tables exposed through `php_vm::CompiledUnit` and
  request execution state.

The normalized lookup names in these APIs are internal keys. User-visible
reflection output must continue to prefer source spelling when the metadata
stores it.

## Known Gaps

Reflection remains incomplete in these areas:

- `ReflectionAttribute::newInstance()` and complete target/repeatability
  validation;
- doc comment retention;
- full `ReflectionEnum`, `ReflectionClass`, `ReflectionMethod`, and
  `ReflectionParameter` API parity;
- method, internal-builtin, unresolved dynamic, and extension callable
  reflection;
- autoload-sensitive Reflection construction and error ordering;
- object identity, inheritance, visibility, and extension behavior that depends
  on SPL or standard-library classes.

## Standard library Direction

Standard library should treat Reflection as a compatibility feature, not just metadata
inspection. The next layer needs constructor invocation APIs such as
`ReflectionClass::newInstanceArgs`, callable invocation APIs, autoload-aware
metadata lookup, doc comments, full enum/class/member APIs, and interaction
with Composer containers that use Reflection for dependency injection.
