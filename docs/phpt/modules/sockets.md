# sockets

- Strategy: deterministic loopback TCP subset
- Selected manifest: `tests/phpt/manifests/modules/sockets.selected.jsonl`
- Selected fixture: `tests/phpt/generated/sockets/basic.phpt`

## Implemented Surface

The runtime exposes selected socket constants, the PHP 8 `Socket` class, and a
bounded TCP loopback subset:

- `socket_create`
- `socket_bind`
- `socket_listen`
- `socket_getsockname`
- `socket_connect`
- `socket_accept`
- `socket_write`
- `socket_read`
- `socket_close`
- `socket_last_error`
- `socket_clear_error`
- `socket_strerror`

Only IPv4 TCP loopback (`127.0.0.1`/`localhost`, `AF_INET`, `SOCK_STREAM`,
`SOL_TCP`) is enabled. The selected PHPT fixture opens a local listener,
connects a local client, accepts it, and verifies bidirectional reads/writes.

## Gaps

UDP, IPv6, Unix sockets, external addresses, `socket_select`, socket options,
nonblocking mode, timeouts, ancillary data, address-info helpers, stream import
or export, Windows protocol helpers, and exact platform errno text remain out of
scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=sockets`
