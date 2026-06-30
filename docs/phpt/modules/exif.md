# exif

- Priority: media metadata MVP
- Selected manifest: `tests/phpt/manifests/modules/exif.selected.jsonl`
- Current focused snapshot: 3 PASS, 0 SKIP, 0 FAIL, 0 BORK from 3 selected
  fixtures

## Scope

- `exif_imagetype`
- `getimagesize` and `getimagesizefromstring` for selected JPEG/PNG/GIF/WebP/BMP
  size probing
- `exif_read_data` for selected JPEG APP1 TIFF metadata:
  `ImageWidth`, `ImageLength`, `Orientation`, `Make`, `Model`, and `DateTime`

## Non-Scope

- Complete EXIF, IPTC, XMP, MakerNote, GPS, and thumbnail metadata parsing
- All upstream `exif_read_data` section and flag behavior
- Image validation beyond deterministic header and APP1 parsing

## Selected PHPT Fixtures

- `tests/phpt/generated/exif/jpeg-metadata-basic.phpt`
- `ext/exif/tests/exif_imagetype_basic.phpt`
- `ext/exif/tests/exif_imagetype_error.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/exif.rs`
- `crates/php_runtime/src/builtins/registry.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime exif`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=exif`
- `nix develop -c just verify-phpt`

## Known Gaps

- Full metadata parity remains out of scope. Add fields only through fixtures
  that exercise concrete WordPress media behavior.
