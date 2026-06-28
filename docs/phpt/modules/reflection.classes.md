# reflection.classes

- Selected manifest: `tests/phpt/manifests/modules/reflection.classes.selected.jsonl`
- Current selected gate: 1 generated PHPT

## Scope

- `ReflectionClass` names, short names, namespace names, class/interface/enum flags, abstract/final flags, parent class, interface names, methods, properties, and constants
- Userland metadata from the runtime class table
- Internal class metadata where the standard-library registry exposes it

## Non-Scope

- Autoload-sensitive constructor behavior
- Complete internal class hierarchy and member parity

## Target Gates

- `nix develop -c cargo test -p php_runtime object`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=reflection.classes`

## Known Gaps

- Internal class surfaces remain registry-bound.
- Unsupported methods fail deterministically instead of returning guessed metadata.
