# memcached

- Strategy: deterministic in-process fake backend for cache probes
- Selected manifest: `tests/phpt/manifests/modules/memcached.selected.jsonl`
- Selected fixture: `tests/phpt/generated/memcached/basic.phpt`

## Current Surface

`extension_loaded("memcached")`, `Memcached`, and `MemcachedException` are
registered for dependency probes. `new Memcached()` constructs a request-local
fake cache object with deterministic support for server registration,
`get`/`set`/`add`/`replace`, multi-key get/set/delete, counters, touch/flush,
options, result codes/messages, append/prepend, CAS-shaped writes, and empty
stats/version arrays.

The fake backend does not open sockets and does not require an external daemon.
It is intended for framework and WordPress object-cache compatibility checks
that only need a predictable cache API surface.

## Gaps

External Memcached protocol support, persistent IDs and sockets, binary
protocol, TLS, SASL, session handlers, real TTL expiry, serializer wire-format
compatibility, compression, and live daemon result-code transitions remain out
of scope for this deterministic slice.

## Target Gates

- `nix develop -c cargo test -p php_vm memcached_fake_backend_tracks_core_cache_result_surface`
- `nix develop -c cargo test -p php_std memcached`
- `nix develop -c just phpt-dev-module MODULE=memcached`
