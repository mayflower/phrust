# wp.stdlib

- Strategy: WordPress-focused generated PHPT harness
- Selected manifest: `tests/phpt/manifests/modules/wp.stdlib.selected.jsonl`
- Selected gate: generated fixtures covering common standard helpers, hash,
  filter, bounded text encoding, media detection, zlib, and ZipArchive reads
  and extraction.

## Runtime Contract

- `extension_loaded()` is true only for implemented extension surfaces.
- Hash supports `md5`, `sha1`, `sha256`, `sha384`, `sha512`, `crc32`, and
  `crc32b`, including HMAC for the supported digest algorithms.
- `filter_var` covers selected email, URL, IP, boolean, and sanitization
  filters. `filter_input` is deterministic unavailable because this branch does
  not add SAPI input state.
- `mbstring` and `iconv` remain bounded to UTF-8/ASCII behavior.
- `fileinfo` and `exif` detect common WordPress media types using lightweight
  magic-byte and extension rules, not a full libmagic or EXIF database.
- `zlib` supports selected gzip/zlib roundtrips.
- `ZipArchive` supports local archive open, entry count, name/index reads,
  basic stat, constrained extraction, and close.

## Selected Fixtures

- `tests/phpt/generated/wp.stdlib/common-arrays-strings-variables.phpt`
- `tests/phpt/generated/wp.stdlib/hash-basic.phpt`
- `tests/phpt/generated/wp.stdlib/filter-basic.phpt`
- `tests/phpt/generated/wp.stdlib/text-encoding-basic.phpt`
- `tests/phpt/generated/wp.stdlib/zlib-basic.phpt`
- `tests/phpt/generated/wp.stdlib/fileinfo-exif-basic.phpt`
- `tests/phpt/generated/wp.stdlib/zip-basic.phpt`

## Known Gaps

| Stable ID | Current behavior | Next owner layer |
| --- | --- | --- |
| `PHPT-WP-STDLIB-OPENSSL-GAP` | OpenSSL signing and certificate APIs are not implemented. | future crypto extension layer |
| `PHPT-WP-STDLIB-SODIUM-GAP` | Sodium remains unavailable rather than fake-loaded. | future sodium dependency decision |
| `PHPT-WP-STDLIB-FILTER-MATRIX-GAP` | The selected filter surface does not cover the exhaustive options and flags matrix. | `php_runtime` filter |
| `PHPT-WP-STDLIB-ENCODING-GAP` | Text encoding support is bounded to UTF-8/ASCII. | future encoding table strategy |
| `PHPT-WP-STDLIB-MEDIA-METADATA-GAP` | Fileinfo/EXIF do not implement full libmagic, EXIF tags, or image processing. | media metadata layer |
| `PHPT-WP-STDLIB-ZIP-WRITE-GAP` | ZipArchive writing, encryption, comments, and stream wrappers are out of scope. | archive layer |

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_std`
- `nix develop -c just phpt-dev-module MODULE=wp.stdlib`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
