# Media Archive MIME Current Report

## Selected Modules

- `fileinfo`: deterministic MIME sniffing for media, archives, JSON/XML, and
  text uploads.
- `exif`: JPEG/PNG/GIF/WebP/BMP image size probing plus selected JPEG APP1
  metadata.
- `zlib`: whole-buffer gzip, zlib, and raw deflate helpers.
- `zip`: `ZipArchive` read/list/extract MVP for local plugin and theme
  archives.
- `gd`: bounded `GdImage` load, resize, and JPEG/PNG save workflow.

## Selected PHPT Fixtures

- `tests/phpt/generated/fileinfo/mime-basic.phpt`
- `tests/phpt/generated/exif/jpeg-metadata-basic.phpt`
- `tests/phpt/generated/zlib/compression-basic.phpt`
- `tests/phpt/generated/zip/archive-basic.phpt`
- `tests/phpt/generated/gd/image-basic.phpt`

## Current Failures

- None in the selected target runs. Each focused module gate ran one selected
  generated fixture and reported 0 non-green target outcomes.
- `verify-stdlib`, `verify-phpt`, and `quality-fast` pass for the current
  branch state.

## Implementation Notes

- The implementation uses deterministic in-repo sniffing and Rust image/archive
  crates instead of host platform databases.
- `image` is an approved branch dependency for the bounded JPEG/PNG GD fallback
  and is validated by the dependency policy gate.
- Dependency policy was kept current by removing the unmaintained
  `rustls-pemfile` dependency and using `rustls-pki-types` PEM loading instead.
- `ZipArchive` remains read-only.

## Remaining Gaps

- Full libmagic parity, full EXIF metadata, ZIP mutation, streaming zlib APIs,
  and complete GD drawing/filter/font support are out of scope for this branch.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=fileinfo`
- `nix develop -c just phpt-dev-module MODULE=exif`
- `nix develop -c just phpt-dev-module MODULE=zlib`
- `nix develop -c just phpt-dev-module MODULE=zip`
- `nix develop -c just phpt-dev-module MODULE=gd`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
