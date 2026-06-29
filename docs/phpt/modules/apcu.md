# apcu

- Strategy: request-local object cache MVP
- Selected manifest: `tests/phpt/manifests/modules/apcu.selected.jsonl`
- Selected fixture: `tests/phpt/generated/apcu/basic.phpt`

## Implemented Surface

The runtime exposes `apcu_enabled`, `apcu_store`, `apcu_add`, `apcu_fetch`,
`apcu_exists`, `apcu_delete`, and `apcu_clear_cache`.

Cache state is request-local VM state, with deterministic TTL expiration when a
positive TTL is supplied. `apcu_fetch` supports the optional by-reference
success output parameter.

## Gaps

Cross-process shared memory, locking semantics, APCu iterators, statistics, and
persistent SAPI lifecycle behavior remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime apcu`
- `nix develop -c just phpt-dev-module MODULE=apcu`
