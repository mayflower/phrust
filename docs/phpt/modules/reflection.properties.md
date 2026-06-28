# reflection.properties

- Selected manifest: `tests/phpt/manifests/modules/reflection.properties.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionProperty` name, declaring class, visibility, static, readonly, type, default metadata, and property-hook flags where modeled

## Non-Scope

- Setting private values through Reflection
- Complete PHP 8.5 property-hook Reflection object parity

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.properties`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Property hooks expose stable flags/lists only where the frontend and runtime already model them.
