# openssl

- Strategy: deterministic crypto helper MVP
- Selected manifest: `tests/phpt/manifests/modules/openssl.selected.jsonl`
- Selected gate: 5 PASS

## Implemented Surface

The runtime exposes the `openssl` extension and selected helpers used by
application update, hashing, and security probes:

- `openssl_random_pseudo_bytes`
- `openssl_digest`
- `openssl_encrypt`
- `openssl_decrypt`
- `openssl_get_md_methods`
- `openssl_get_cipher_methods`
- `openssl_cipher_iv_length`
- `openssl_verify`
- `OPENSSL_ALGO_SHA256`
- `OPENSSL_RAW_DATA`
- `OPENSSL_ZERO_PADDING`
- `OPENSSL_DONT_ZERO_PAD_KEY`

Digest support is implemented for the selected hash families backed by Rust
digest crates. `openssl_random_pseudo_bytes()` uses OS randomness.
Cipher metadata and encrypt/decrypt behavior are limited to selected
AES-128-CBC and AES-256-CBC probes backed by OpenSSL. `OPENSSL_RAW_DATA` and
`OPENSSL_ZERO_PADDING` are covered by deterministic AES-CBC fixtures.
`OPENSSL_DONT_ZERO_PAD_KEY` is exposed for PHP-visible constant parity but
returns the selected false result until key-length override behavior is
implemented. Selected cipher failures and unsupported public-key helpers append
request-local OpenSSL error strings that `openssl_error_string()` drains in FIFO
order, returning `false` when the queue is empty.
`openssl_verify()` intentionally returns the explicit unsupported verification
result covered by the selected fixture; it does not fake signature validation.

## Gaps

Certificate parsing, key loading, signature verification parity,
AEAD cipher modes, stream TLS contexts, host OpenSSL configuration, and
certificate-store behavior remain unsupported until backed by deterministic
fixtures and an approved dependency strategy.

## Source References

- `ext/openssl/openssl.stub.php`
- `ext/openssl/tests/`

## Target Gates

- `nix develop -c cargo test -p php_runtime openssl`
- `nix develop -c just phpt-dev-module MODULE=openssl`
- `nix develop -c just phpt-dev-module MODULE=closure.extensions`
