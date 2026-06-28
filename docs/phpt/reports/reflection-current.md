# Reflection Current Status

Generated: 2026-06-28

Branch: `phpt/b3-spl-reflection`

The Reflection PHPT gate is split into eight selected submodules. Branch 3
promoted focused upstream Reflection tests only where the metadata is backed by
generated arginfo, frontend/IR metadata, runtime class metadata, or the standard
library registry.

## Selected PHPT Results

| Submodule | Before branch | After branch | Main covered metadata | Known gaps |
| --- | --- | --- | --- | --- |
| `reflection.functions` | 1 PASS | 3 PASS | internal/userland function names, counts, return type, extension, closure flag | callable invocation, doc comments |
| `reflection.parameters` | 1 PASS | 3 PASS | generated arginfo names, positions, optionality, variadic, by-ref, simple types | default constants, complex ReflectionType parity |
| `reflection.classes` | 1 PASS | 4 PASS, 1 SKIP | names, namespace, flags, parent, interfaces, member counts, extension owner | full internal class parity, autoload-sensitive construction |
| `reflection.methods` | 1 PASS | 1 PASS | declaring class, visibility, static/final/abstract modifiers, parameters, return type | invocation, broad object interpolation/stringification cases |
| `reflection.properties` | 1 PASS | 2 PASS | declaring class, visibility, static, readonly, type, modifier bits | private value mutation, property-hook object parity |
| `reflection.attributes` | 1 PASS | 1 PASS | names, arguments, repeat metadata, class/method/property/parameter targets | `newInstance`, full target validation |
| `reflection.enums` | 1 PASS | 4 PASS | backed enum type, cases, backed case values | serialization and exact exception parity |
| `reflection.extensions` | 1 PASS | 2 PASS, 1 SKIP | extension name, functions, classes, owner metadata | dependencies, INI matrix, module globals, Zend ABI |
| `reflection` aggregate | 8 PASS | 20 PASS, 2 SKIP | all selected Reflection subareas | full upstream `ext/reflection/tests` backlog |

The aggregate `reflection` selected run has 22 selected PHPTs and 0 non-green
target outcomes. The full upstream Reflection corpus remains tracked by the
committed full PHPT baseline and known-gap catalog.

## Upstream Promotions

- `reflection.functions`: `ReflectionFunction_getExtensionName.phpt`,
  `ReflectionFunction_isClosure_basic.phpt`
- `reflection.parameters`: `ReflectionParameter_isVariadic_basic.phpt`,
  `ReflectionParameter_getPosition_basic.phpt`
- `reflection.classes`: `ReflectionClass_isEnum.phpt`,
  `ReflectionClass_getNamespaceName.phpt`,
  `ReflectionClass_isAbstract_basic.phpt`,
  `ReflectionClass_getExtensionName_basic.phpt`
- `reflection.properties`: `ReflectionProperty_getModifiers.001.phpt`
- `reflection.enums`: `ReflectionEnum_getBackingType.phpt`,
  `ReflectionEnum_isBacked.phpt`, `ReflectionEnum_hasCase.phpt`
- `reflection.extensions`: `ReflectionExtension_getName_basic.phpt`,
  `ReflectionExtension_getClassNames_variation1.phpt`

`ReflectionMethod_getModifiers_basic.phpt` was probed but not promoted because
the upstream case depends on object stringification/interpolation and invocation
behavior outside this branch's metadata scope.

## Closed During Branch 3

- `ReflectionParameter::getPosition()` now reports real parameter positions for
  internal and userland parameters.
- `ReflectionMethod::getModifiers()` now reports metadata-backed
  public/protected/private/static/final/abstract modifier bits for internal and
  userland methods.
- The standard-library registry exposes Reflection as an enabled extension using
  generated arginfo class ownership.
- `ReflectionExtension('reflection')->getName()` preserves case-insensitive
  lookup while returning the canonical `Reflection` display name.

## Verification

Passed with the pinned PHP 8.5.7 reference binary and branch-local PHPT target:

- `nix develop -c just phpt-dev-module MODULE=reflection.functions`
- `nix develop -c just phpt-dev-module MODULE=reflection.parameters`
- `nix develop -c just phpt-dev-module MODULE=reflection.classes`
- `nix develop -c just phpt-dev-module MODULE=reflection.methods`
- `nix develop -c just phpt-dev-module MODULE=reflection.properties`
- `nix develop -c just phpt-dev-module MODULE=reflection.attributes`
- `nix develop -c just phpt-dev-module MODULE=reflection.enums`
- `nix develop -c just phpt-dev-module MODULE=reflection.extensions`
- `nix develop -c just phpt-dev-module MODULE=reflection`
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
- `nix develop -c cargo test -p php_std`
- `nix develop -c cargo test -p php_runtime object`
- `nix develop -c cargo test -p php_vm`

Every PHPT module run also verified the pinned `php-src` source-integrity
manifest: 24,475 entries checked, 0 skipped.

## Next Step

Promote additional upstream `ext/reflection/tests` cases only as their owning
runtime, VM, arginfo, or metadata gaps close. Reflection invocation,
`ReflectionAttribute::newInstance()`, Zend ABI metadata, and fake internal
surfaces remain out of scope for this branch.
