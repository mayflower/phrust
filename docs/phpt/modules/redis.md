# redis

- Strategy: deterministic in-process fake backend for CI-safe phpredis probes
- Selected manifest: `tests/phpt/manifests/modules/redis.selected.jsonl`
- Selected fixture: `tests/phpt/generated/redis/basic.phpt`

## Current Surface

`extension_loaded("redis")`, `class_exists("Redis", false)`, `new Redis()`,
`instanceof Redis`, and `method_exists()` are available for dependency probes.

`Redis` uses a deterministic request-local fake backend. It does not open
sockets, but it supports the high-use command families needed by smoke tests
and application probes:

- connection/probe methods: `connect`, `pconnect`, `auth`, `select`, `close`,
  `ping`, `isConnected`
- string/key methods: `set`, `setex`, `setnx`, `get`, `mget`, `getMultiple`,
  `mset`, `del`, `delete`, `unlink`, `exists`, `expire`, `pexpire`,
  `persist`, `ttl`, `pttl`, `incr`, `incrBy`, `decr`, `decrBy`
- collection methods: `hSet`, `hGet`, `hGetAll`, `hDel`, `hExists`, `lPush`,
  `rPush`, `lPop`, `rPop`, `lLen`, `sAdd`, `sMembers`, `sIsMember`,
  `sContains`, `sRem`, `sRemove`, `zAdd`, `zRange`
- smoke placeholders: `multi`, `pipeline`, `exec`, `discard`, `scan`,
  `setOption`, `getOption`

## Gaps

External Redis protocol support, real persistent sockets, clusters, sentinel,
pub/sub, Lua `eval`, streams, blocking commands, serializer/compression
compatibility, and real time-based expiration remain gaps. Expiration methods
currently report key existence deterministically and `ttl`/`pttl` return `-1`
for present non-expiring keys and `-2` for missing keys.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm redis_fake_backend_covers_core_wordpress_probe_surface --no-fail-fast`
- `nix develop -c just phpt-dev-module MODULE=redis`
