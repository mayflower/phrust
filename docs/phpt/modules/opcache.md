# opcache

- Strategy: request-local API facade
- Classification: bounded OPcache surface
- Selected manifest: `tests/phpt/manifests/modules/opcache.selected.jsonl`
- Current corpus snapshot: 593 `opcache` candidates, 220 PASS, 8 SKIP, 364
  FAIL, 0 BORK, and 449 known non-green outcomes.

## Decision

Expose the PHP-visible OPcache API as a deterministic request-local facade.
The facade records `opcache_compile_file()` calls for existing files so status,
`opcache_is_script_cached()`, invalidation, and reset probes have stable PHP
behavior.

This is not Zend OPcache, a persistent bytecode cache, an optimizer, preloading,
or JIT. PHPTs that require optimizer decisions, file update protection,
preloading, shared memory, or JIT behavior remain out of scope.

## Covered Area

- `extension_loaded("opcache")`
- `opcache_get_status()`
- `opcache_get_configuration()`
- `opcache_compile_file()`
- `opcache_invalidate()`
- `opcache_is_script_cached()`
- `opcache_is_script_cached_in_file_cache()`
- `opcache_reset()`

## Unsupported Area

- Stable ID: `PHPT-DATA-OPCACHE`
- Reference behavior: PHP with Zend OPcache enabled exposes process/shared
  cache state, invalidation, preloading, optimizer behavior, file-cache behavior,
  and JIT controls.
- Current phrust behavior: `extension_loaded("opcache")` is true and the
  selected API surface returns deterministic request-local arrays. No optimizer,
  shared cache, preloader, persistent file cache, or JIT is implemented.
- Fixture: `tests/phpt/generated/opcache/platform-checks.phpt`
- Next owner layer: optional performance/cache layer that can bridge facade
  counters to durable bytecode-cache telemetry.

## Policy

- Zend Opcache replacement: out-of-scope.
- JIT PHPTs: out-of-scope unless minimized to ordinary PHP behavior in the
  owning module.
- Preload/cache invalidation PHPTs: out-of-scope.

## Source References

- `ext/opcache/opcache.stub.php`
- `ext/opcache/tests/`
- `ext/opcache/tests/jit/`

## Target Gates

- `nix develop -c just phpt-dev-module MODULE=opcache`
- `nix develop -c just verify-phpt`
