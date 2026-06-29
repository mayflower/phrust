# sodium

- Strategy: real crypto MVP
- Selected manifest: `tests/phpt/manifests/modules/sodium.selected.jsonl`
- Selected fixture: `tests/phpt/generated/sodium/basic.phpt`

## Implemented Surface

The sodium slice uses real Rust crypto dependencies rather than fake results:
`blake2b_simd` for `sodium_crypto_generichash` and `ed25519-dalek` for detached
Ed25519 signatures.

Implemented functions are `sodium_bin2hex`, `sodium_hex2bin`,
`sodium_bin2base64`, `sodium_base642bin`, `sodium_crypto_generichash`,
`sodium_crypto_generichash_keygen`, `sodium_crypto_sign_detached`,
and `sodium_crypto_sign_verify_detached`.

## Gaps

Secretbox, password hashing, AEAD, key exchange, sealed boxes, sodium-specific
random byte helpers, and the broader libsodium API are not implemented.
Unsupported algorithms must stay explicit failures.

## Target Gates

- `nix develop -c cargo test -p php_runtime sodium`
- `nix develop -c just phpt-dev-module MODULE=sodium`
- `nix develop -c just verify-stdlib`
