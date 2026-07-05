# gmp

- Strategy: BigInt facade MVP
- Selected manifest: `tests/phpt/manifests/modules/gmp.selected.jsonl`
- Selected fixture: `tests/phpt/generated/gmp/basic.phpt`

## Implemented Surface

The runtime exposes a deterministic BigInt-backed facade for common
`gmp_*` APIs: initialization/string conversion/import/export, arithmetic,
quotient/remainder helpers, gcd/lcm/invert, roots, modular exponentiation,
basic primality helpers, bitwise operations, bit scans/counts, factorial, and
binomial coefficients.

Values are represented as decimal strings suitable for `gmp_strval` and common
plugin arithmetic flows. This keeps integer behavior BigInt-backed without
pretending to provide full native GMP object identity.

## Gaps

Native GMP object identity, mutable `GMP` object bit operations, full
serialization, limb-order import/export flags, secure randomness, and
Jacobi/Legendre/Kronecker helpers remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime gmp`
- `nix develop -c cargo test -p php_std gmp`
- `nix develop -c just phpt-dev-module MODULE=gmp`
