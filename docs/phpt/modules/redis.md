# redis

- Strategy: disabled external service surface
- Selected manifest: `tests/phpt/manifests/modules/redis.selected.jsonl`
- Selected fixture: `tests/phpt/generated/redis/basic.phpt`

## Current Surface

`extension_loaded("redis")` and `class_exists("Redis", false)` are available
for dependency probes. No Redis commands or network connections are implemented
by default.

## Gaps

External Redis protocol support, real connections, clusters, persistent
connections, and in-memory command emulation remain out of scope until an
explicit capability or adapter design is approved.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=redis`
