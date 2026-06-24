# Standard library Encoding, Hash, HTML, and URL Helpers

Reference target: PHP 8.5.7 (`php-8.5.7`).

Work item adds pragmatic standard-library helpers for binary/hex,
base64, MD5/SHA1/CRC32, common HTML escaping, URL encoding, and a simple
`http_build_query` MVP.

Dependency review:

- `md-5` and `sha1` are RustCrypto digest crates used only for deterministic
  PHP-compatible digest bytes.
- `base64` is used for RFC 4648-compatible standard base64 encode/decode.
- `crc32fast` is used for the standard CRC32 checksum.
- `getrandom` is used only by `random_bytes` and `random_int` for OS-backed
  randomness; no deterministic test relies on specific random output bytes.

No dependency performs network, filesystem, process, or locale access. HTML
handling is an MVP for common default flags; full entity tables, charset
handling, and the full PHP hash algorithm registry remain known gaps in
`docs/stdlib-known-gaps.md`.

Implemented surface:

- Binary and digest helpers: `bin2hex`, `hex2bin`, `ord`, `chr`, `md5`,
  `sha1`, `crc32`, `hash`, `hash_hmac`, `base64_encode`, and
  `base64_decode`. Malformed `hex2bin` inputs return `false` and emit
  PHP-style warnings for odd length and non-hex payloads.
- Random helpers: `random_bytes` and `random_int` use OS randomness and are
  covered through shape/range differential assertions.
- HTML helpers: default-mode `htmlspecialchars`,
  `htmlspecialchars_decode`, and an `htmlentities` MVP alias.
- URL helpers: `urlencode`, `urldecode`, `rawurlencode`,
  `rawurldecode`, plus `http_build_query` for arrays.

Known gaps for this slice:

- `STDLIB-GAP-HTML-FULL-ENTITIES-FLAGS`
- `STDLIB-GAP-HTTP-BUILD-QUERY-OPTIONS`
- `STDLIB-GAP-HASH-RANDOM-ALGORITHMS`
