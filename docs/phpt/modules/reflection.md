# reflection

- Priority: 21
- Selected manifest: `tests/phpt/manifests/modules/reflection.selected.jsonl`
- Current selected gate: 22 PHPTs covering the Reflection MVP subareas (8 generated, 14 upstream)
- Baseline context: 304 upstream Reflection corpus candidates remain tracked in the full PHPT baseline

## Scope

- Reflection metadata for functions, parameters, classes, methods, properties, attributes, enums, and extensions
- Generated arginfo for internal functions and methods where available
- Userland metadata from the existing frontend, IR, runtime class table, and VM source maps

## Submodules

| Submodule | Selected manifest | Fixture |
| --- | --- | --- |
| `reflection.functions` | `tests/phpt/manifests/modules/reflection.functions.selected.jsonl` | `tests/phpt/generated/reflection.functions/builtin-and-user-functions.phpt` |
| `reflection.parameters` | `tests/phpt/manifests/modules/reflection.parameters.selected.jsonl` | `tests/phpt/generated/reflection.parameters/internal-parameter-arginfo.phpt` |
| `reflection.classes` | `tests/phpt/manifests/modules/reflection.classes.selected.jsonl` | `tests/phpt/generated/reflection.classes/class-basics.phpt` |
| `reflection.methods` | `tests/phpt/manifests/modules/reflection.methods.selected.jsonl` | `tests/phpt/generated/reflection.methods/method-metadata.phpt` |
| `reflection.properties` | `tests/phpt/manifests/modules/reflection.properties.selected.jsonl` | `tests/phpt/generated/reflection.properties/property-metadata.phpt` |
| `reflection.attributes` | `tests/phpt/manifests/modules/reflection.attributes.selected.jsonl` | `tests/phpt/generated/reflection.attributes/attribute-metadata.phpt` |
| `reflection.enums` | `tests/phpt/manifests/modules/reflection.enums.selected.jsonl` | `tests/phpt/generated/reflection.enums/enum-metadata.phpt` |
| `reflection.extensions` | `tests/phpt/manifests/modules/reflection.extensions.selected.jsonl` | `tests/phpt/generated/reflection.extensions/extension-symbols.phpt` |

## Upstream Promotions

- `reflection.functions`: `ReflectionFunction_getExtensionName.phpt`, `ReflectionFunction_isClosure_basic.phpt`
- `reflection.parameters`: `ReflectionParameter_isVariadic_basic.phpt`, `ReflectionParameter_getPosition_basic.phpt`
- `reflection.classes`: `ReflectionClass_isEnum.phpt`, `ReflectionClass_getNamespaceName.phpt`, `ReflectionClass_isAbstract_basic.phpt`, `ReflectionClass_getExtensionName_basic.phpt`
- `reflection.properties`: `ReflectionProperty_getModifiers.001.phpt`
- `reflection.enums`: `ReflectionEnum_getBackingType.phpt`, `ReflectionEnum_isBacked.phpt`, `ReflectionEnum_hasCase.phpt`
- `reflection.extensions`: `ReflectionExtension_getName_basic.phpt`, `ReflectionExtension_getClassNames_variation1.phpt`

## Non-Scope

- Fake metadata not backed by frontend/runtime/arginfo
- Reflection invocation APIs
- Attribute instantiation
- Complete upstream Reflection API and modifier bit parity
- Zend ABI module internals

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection`
- `nix develop -c just phpt-dev-module MODULE=reflection.functions`
- `nix develop -c just phpt-dev-module MODULE=reflection.parameters`
- `nix develop -c just phpt-dev-module MODULE=reflection.classes`
- `nix develop -c just phpt-dev-module MODULE=reflection.methods`
- `nix develop -c just phpt-dev-module MODULE=reflection.properties`
- `nix develop -c just phpt-dev-module MODULE=reflection.attributes`
- `nix develop -c just phpt-dev-module MODULE=reflection.enums`
- `nix develop -c just phpt-dev-module MODULE=reflection.extensions`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Full generated arginfo parity for every internal symbol and exact default constants
- Complete internal class, method, property, and class-constant surfaces
- ReflectionMethod upstream broad cases currently depend on object stringification/interpolation or invocation rather than only metadata
- full `ReflectionAttribute::newInstance()` parity, including target/repeatability validation
- Property hook Reflection object parity
- Enum serialization and byte-perfect exception gaps
- Extension versions, dependencies, INI entries, module globals, and Zend ABI metadata

## Next Step

Promote upstream `ext/reflection/tests` cases into the submodule manifests as their owning runtime, VM, arginfo, or metadata gaps close.
