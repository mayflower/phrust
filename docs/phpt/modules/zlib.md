# zlib

- Priority: compression MVP
- Selected manifest: `tests/phpt/manifests/modules/zlib.selected.jsonl`
- Current focused snapshot: 6 PASS, 0 SKIP, 0 FAIL, 0 BORK from 6 selected
  fixtures

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
- `ext/zlib/tests/gzcompress_basic1.phpt`
- `ext/zlib/tests/gzdeflate_basic1.phpt`
- `ext/zlib/tests/gzdeflate_variation1.phpt`
- `ext/zlib/tests/gzencode_basic1.phpt`
- `ext/zlib/tests/gzuncompress_basic1.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/zlib.rs`
- `crates/php_runtime/src/builtins/registry.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime zlib`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zlib`
- `nix develop -c just verify-phpt`

## Known Gaps

- Keep this layer focused on whole-buffer helpers until runtime streaming APIs
  have a broader owner.
- `gzencode_variation1.phpt` remains an OS-header-specific reference skip on
  Darwin, and `gzinflate_length.phpt` remains outside the selected gate until
  insufficient-memory warning/output parity is implemented.

## Request Filesystem Overlay

The `wp.request-filesystem` overlay adds selected gzip file-handle coverage for
`gzopen`, `gzread`, `gzwrite`, and `gzclose` using deterministic local files.
It does not introduce stream-filter or network zlib contexts.
