# Platform Crypto Math Cache Current Report

## Status

This branch adds selected optional extension surfaces for crypto, encoding,
validation, math, cache, and default-disabled transports. The selected PHPT
slice is one generated fixture per module:

| Module | Fixture | Current behavior |
| --- | --- | --- |
| `ctype` | `tests/phpt/generated/ctype/basic.phpt` | ASCII C-locale predicates |
| `filter` | `tests/phpt/generated/filter/basic.phpt` | Common scalar validation and sanitization |
| `iconv` | `tests/phpt/generated/iconv/basic.phpt` | UTF-8, ASCII, ISO-8859-1 conversion and state |
| `sodium` | `tests/phpt/generated/sodium/basic.phpt` | Real BLAKE2b, Ed25519, hex/base64, random keygen |
| `bcmath` | `tests/phpt/generated/bcmath/basic.phpt` | BigInt-backed decimal arithmetic |
| `gmp` | `tests/phpt/generated/gmp/basic.phpt` | BigInt string facade arithmetic |
| `apcu` | `tests/phpt/generated/apcu/basic.phpt` | Request-local object cache subset |
| `redis` | `tests/phpt/generated/redis/basic.phpt` | Extension/class introspection only |
| `memcached` | `tests/phpt/generated/memcached/basic.phpt` | Extension/class introspection only |
| `ftp` | `tests/phpt/generated/ftp/basic.phpt` | Deterministic default-disabled connect failures |
| `sockets` | `tests/phpt/generated/sockets/basic.phpt` | Constants plus deterministic creation failure |

## Before And After

Before this branch, these modules were absent or incomplete for the selected
dependency probes. After this branch:

- `ctype`, `filter`, `iconv`, `sodium`, `bcmath`, `gmp`, and `apcu` expose
  bounded real behavior.
- `redis` and `memcached` expose dependency introspection only, without fake
  external service behavior.
- `ftp` and `sockets` expose deterministic default-disabled behavior and do not
  open host network connections.

## Dependencies Added

- `blake2b_simd` for libsodium-compatible BLAKE2b generichash.
- `ed25519-dalek` for detached Ed25519 signatures.
- `num-bigint` and `num-traits` for bcmath and gmp arithmetic.

## Remaining Optional Service Gaps

- Redis and Memcached do not implement external protocols or local adapters.
- FTP and sockets do not perform real network I/O.
- APCu is request-local, not shared memory.
- Sodium remains a selected real-crypto subset, not the full libsodium API.
- iconv remains limited to UTF-8, ASCII, and ISO-8859-1.
- bcmath and gmp are bounded arithmetic MVPs, not full extension parity.

## Validation

Use the branch closeout gates:

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=filter`
- `nix develop -c just phpt-dev-module MODULE=ctype`
- `nix develop -c just phpt-dev-module MODULE=iconv`
- `nix develop -c just phpt-dev-module MODULE=sodium`
- `nix develop -c just phpt-dev-module MODULE=bcmath`
- `nix develop -c just phpt-dev-module MODULE=gmp`
- `nix develop -c just phpt-dev-module MODULE=apcu`
- `nix develop -c just phpt-dev-module MODULE=redis`
- `nix develop -c just phpt-dev-module MODULE=memcached`
- `nix develop -c just phpt-dev-module MODULE=ftp`
- `nix develop -c just phpt-dev-module MODULE=sockets`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
