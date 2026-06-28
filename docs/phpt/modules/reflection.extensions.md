# reflection.extensions

- Selected manifest: `tests/phpt/manifests/modules/reflection.extensions.selected.jsonl`
- Current selected gate: 3 PHPTs (1 generated, 2 upstream)

## Scope

- `ReflectionExtension` canonical name
- `getFunctions()`
- `getClasses()`/`getClassNames()`
- Extension ownership from the builtin registry and generated arginfo metadata
- Upstream coverage: `ReflectionExtension_getName_basic.phpt`, `ReflectionExtension_getClassNames_variation1.phpt`

## Non-Scope

- Extension dependencies
- Full INI entry matrix
- Module globals
- Zend ABI metadata

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.extensions`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Versions, dependencies, INI entries, constants as full reflection objects, and module-global internals are not invented when unavailable.
