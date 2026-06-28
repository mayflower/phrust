# reflection.methods

- Selected manifest: `tests/phpt/manifests/modules/reflection.methods.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionMethod` name, declaring class, visibility, static, final, abstract, modifier bits, parameters, return type, and extension name where available

## Non-Scope

- Invoking methods through Reflection
- Upstream method PHPTs that were probed currently depend on unrelated string interpolation, object stringification, or invocation behavior

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.methods`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Complete internal method surfaces depend on generated class-method metadata and registry coverage.
- `ReflectionMethod_getModifiers_basic.phpt` is blocked by string interpolation of reflection object properties, not modifier bit metadata.
