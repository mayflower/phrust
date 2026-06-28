# reflection.parameters

- Selected manifest: `tests/phpt/manifests/modules/reflection.parameters.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionParameter` names, required/optional state, variadic flag, by-reference flag, and simple type display
- Internal function parameters sourced from generated PHP 8.5 arginfo
- Userland parameter metadata sourced from IR when covered by `ReflectionFunction`

## Non-Scope

- Complete default constant parity
- Full union, intersection, and DNF ReflectionType object parity

## Target Gates

- `nix develop -c cargo test -p php_std`
- `nix develop -c just phpt-dev-module MODULE=reflection.parameters`

## Known Gaps

- Defaults are exposed only where the existing metadata carries deterministic values.
- Complex type parity remains part of the broader upstream Reflection backlog.
