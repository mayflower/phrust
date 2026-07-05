# bcmath

- Strategy: bounded decimal math MVP
- Selected manifest: `tests/phpt/manifests/modules/bcmath.selected.jsonl`
- Selected fixtures:
  - `tests/phpt/generated/bcmath/basic.phpt`
  - `tests/phpt/generated/bcmath/scale-state.phpt`

## Implemented Surface

The runtime exposes `bcadd`, `bcsub`, `bcmul`, `bcdiv`, `bcmod`, `bcpow`,
`bccomp`, and `bcscale`.

The implementation uses a BigInt-backed decimal representation and truncating
division for selected dependency-facing arithmetic.

`bcscale()` is request-local: reads return the current default scale, writes
return the previous scale, and omitted arithmetic scale arguments consume the
current default.

## Gaps

Full rounding parity and every warning edge case remain out of scope for this
MVP.

## Target Gates

- `nix develop -c cargo test -p php_runtime bcmath`
- `nix develop -c just phpt-dev-module MODULE=bcmath`
