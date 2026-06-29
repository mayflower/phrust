# sockets

- Strategy: default-disabled sockets surface
- Selected manifest: `tests/phpt/manifests/modules/sockets.selected.jsonl`
- Selected fixture: `tests/phpt/generated/sockets/basic.phpt`

## Implemented Surface

The runtime exposes selected socket constants plus `socket_create`,
`socket_last_error`, and `socket_strerror`.

`socket_create` returns `false` deterministically by default. No network socket
is opened.

## Gaps

Socket connect, bind, listen, accept, read, write, select, options, and real
network capability handling remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c just phpt-dev-module MODULE=sockets`
