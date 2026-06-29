# bcmath

- Strategy: bounded decimal math MVP
- Selected manifest: `tests/phpt/manifests/modules/bcmath.selected.jsonl`
- Selected fixture: `tests/phpt/generated/bcmath/basic.phpt`

## Implemented Surface

The runtime exposes `bcadd`, `bcsub`, `bcmul`, `bcdiv`, `bcmod`, `bcpow`,
`bccomp`, and `bcscale`.

The implementation uses a BigInt-backed decimal representation and truncating
division for selected dependency-facing arithmetic.

## Gaps

Full rounding parity, every warning edge case, and persistent global `bcscale`
state remain out of scope for this MVP.

## Target Gates

- `nix develop -c cargo test -p php_runtime bcmath`
- `nix develop -c just phpt-dev-module MODULE=bcmath`
