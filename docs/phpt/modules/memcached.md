# memcached

- Strategy: disabled external service surface
- Selected manifest: `tests/phpt/manifests/modules/memcached.selected.jsonl`
- Selected fixture: `tests/phpt/generated/memcached/basic.phpt`

## Current Surface

`extension_loaded("memcached")` and `class_exists("Memcached", false)` are
available for dependency probes. No Memcached commands or network connections
are implemented by default.

## Gaps

External Memcached protocol support, real connections, persistent connections,
and in-memory command emulation remain out of scope until an explicit
capability or adapter design is approved.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=memcached`
