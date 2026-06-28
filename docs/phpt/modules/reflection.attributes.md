# reflection.attributes

- Selected manifest: `tests/phpt/manifests/modules/reflection.attributes.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionAttribute::getName()`
- `ReflectionAttribute::getArguments()`
- Repeat metadata
- Class, function, method, property, parameter, and enum-case attachment points where metadata exists

## Non-Scope

- `ReflectionAttribute::newInstance()`
- Complete target and repeatability validation parity

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.attributes`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Attribute instantiation remains a documented gap until construction and autoload-sensitive resolution are routed through normal object creation.
