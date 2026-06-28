# reflection.enums

- Selected manifest: `tests/phpt/manifests/modules/reflection.enums.selected.jsonl`
- Current selected gate: 4 PHPTs (1 generated, 3 upstream)

## Scope

- `ReflectionEnum`
- `isBacked()`
- `getBackingType()`
- `getCases()`
- `ReflectionEnumUnitCase`
- `ReflectionEnumBackedCase`
- Backed case values where metadata is available
- Upstream coverage: `ReflectionEnum_getBackingType.phpt`, `ReflectionEnum_isBacked.phpt`, `ReflectionEnum_hasCase.phpt`

## Non-Scope

- Enum serialization parity
- Byte-perfect exception text for every enum edge case

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.enums`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Exact exception ordering and serialization remain tracked in runtime semantics known gaps.
