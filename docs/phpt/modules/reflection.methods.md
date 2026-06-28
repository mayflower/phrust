# reflection.methods

- Selected manifest: `tests/phpt/manifests/modules/reflection.methods.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionMethod` name, declaring class, visibility, static, final, abstract, parameters, return type, and extension name where available

## Non-Scope

- Invoking methods through Reflection
- Exact modifier bit parity outside selected fixtures

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=reflection.methods`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Complete internal method surfaces depend on generated class-method metadata and registry coverage.
