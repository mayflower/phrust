# reflection.enums

- Selected manifest: `tests/phpt/manifests/modules/reflection.enums.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionEnum`
- `isBacked()`
- `getBackingType()`
- `getCases()`
- `ReflectionEnumUnitCase`
- `ReflectionEnumBackedCase`
- Backed case values where metadata is available

## Non-Scope

- Enum serialization parity
- Byte-perfect exception text for every enum edge case

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.enums`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Exact exception ordering and serialization remain tracked in runtime semantics known gaps.
