# zlib

- Priority: compression MVP
- Selected manifest: `tests/phpt/manifests/modules/zlib.selected.jsonl`
- Current focused snapshot: 1 PASS, 0 SKIP, 0 FAIL, 0 BORK from 1 selected
  generated fixture

## Scope

- Existing gzip/zlib helpers: `gzencode`, `gzdecode`, `gzcompress`,
  `gzuncompress`, and `zlib_decode`
- Added raw and selectable encoding helpers: `gzdeflate`, `gzinflate`, and
  `zlib_encode`
- Constants: `ZLIB_ENCODING_RAW`, `ZLIB_ENCODING_GZIP`, and
  `ZLIB_ENCODING_DEFLATE`

## Non-Scope

- Streaming zlib contexts
- Complete compression-level validation and warning parity
- Obscure `zlib_encode` encodings outside the selected constants

## Selected PHPT Fixtures

- `tests/phpt/generated/zlib/compression-basic.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/zlib.rs`
- `crates/php_runtime/src/builtins/registry.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime zlib`
- `nix develop -c just phpt-dev-module MODULE=zlib`
- `nix develop -c just verify-phpt`

## Known Gaps

- Keep this layer focused on whole-buffer helpers until runtime streaming APIs
  have a broader owner.
