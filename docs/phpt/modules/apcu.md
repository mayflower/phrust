# apcu

- Strategy: request-local object cache MVP
- Selected manifest: `tests/phpt/manifests/modules/apcu.selected.jsonl`
- Selected fixture: `tests/phpt/generated/apcu/basic.phpt`

## Implemented Surface

The runtime exposes `apcu_enabled`, `apcu_store`, `apcu_add`, `apcu_fetch`,
`apcu_exists`, `apcu_delete`, `apcu_clear_cache`, `apcu_inc`, `apcu_dec`,
`apcu_cache_info`, and `apcu_sma_info`.

Cache state is request-local VM state, with deterministic TTL expiration when a
positive TTL is supplied. `apcu_fetch` supports the optional by-reference
success output parameter. `apcu_inc` and `apcu_dec` update existing integer
entries and support the optional by-reference success output parameter.
`apcu_cache_info` and `apcu_sma_info` expose deterministic request-local shape
and counters for framework probes.

## Gaps

Cross-process shared memory, locking semantics, APCu iterators, per-entry memory
accounting, and persistent SAPI lifecycle behavior remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime apcu`
- `nix develop -c cargo test -p php_std apcu`
- `nix develop -c just phpt-dev-module MODULE=apcu`
