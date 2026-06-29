# gmp

- Strategy: BigInt facade MVP
- Selected manifest: `tests/phpt/manifests/modules/gmp.selected.jsonl`
- Selected fixture: `tests/phpt/generated/gmp/basic.phpt`

## Implemented Surface

The runtime exposes `gmp_init`, `gmp_strval`, `gmp_intval`, `gmp_add`,
`gmp_sub`, `gmp_mul`, `gmp_div_q`, `gmp_mod`, `gmp_abs`, `gmp_neg`, `gmp_pow`,
and `gmp_cmp`.

Values are represented as decimal strings suitable for `gmp_strval` and common
plugin arithmetic flows.

## Gaps

Native GMP object identity, full `GMP` class behavior, full base conversion
coverage, and the complete GMP function set remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime gmp`
- `nix develop -c just phpt-dev-module MODULE=gmp`
