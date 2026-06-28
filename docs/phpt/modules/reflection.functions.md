# reflection.functions

- Selected manifest: `tests/phpt/manifests/modules/reflection.functions.selected.jsonl`
- Current selected gate: 3 PHPTs (1 generated, 2 upstream)

## Scope

- `ReflectionFunction` for internal functions through `php_std::arginfo`
- `ReflectionFunction` for userland functions through IR metadata
- Names, internal/userland flags, parameter counts, return type, and extension name
- Upstream coverage: `ReflectionFunction_getExtensionName.phpt`, `ReflectionFunction_isClosure_basic.phpt`

## Non-Scope

- Reflection invocation APIs
- Doc comment parity
- Unsupported dynamic callable forms

## Target Gates

- `nix develop -c cargo test -p php_std`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=reflection.functions`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- Variadic upstream coverage remains blocked by unrelated output/stringification gaps.
- Method and extension callable reflection is still intentionally bounded.
- Doc comments are not retained in runtime metadata.
