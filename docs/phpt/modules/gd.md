# gd

- Priority: bounded GD-compatible media fallback
- Selected manifest: `tests/phpt/manifests/modules/gd.selected.jsonl`
- Current focused snapshot: 1 PASS, 0 SKIP, 0 FAIL, 0 BORK from 1 selected
  generated fixture

## Scope

- `extension_loaded("gd")`, `class_exists("GdImage")`, and `gd_info`
- `imagecreatefromstring`, `imagecreatefromjpeg`, and `imagecreatefrompng`
- `imagecreatetruecolor`, `imagesx`, and `imagesy`
- `imagecopyresampled` for the selected crop/resize/copy path
- `imagejpeg` and `imagepng` file output
- `imagedestroy`

## Non-Scope

- Full upstream `ext/gd` drawing, filter, palette, alpha, font, text, and color
  management APIs
- AVIF/WebP/GIF/BMP/TIFF write support
- Imagick compatibility
- Binary encoder parity beyond readable JPEG and PNG files for the selected
  media fixture

## Selected PHPT Fixtures

- `tests/phpt/generated/gd/image-basic.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/gd.rs`
- `crates/php_runtime/src/builtins/registry.rs`
- `crates/php_std/src/lib.rs`
- `crates/php_std/src/introspection.rs`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime gd`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=gd`
- `nix develop -c just verify-phpt`

## Known Gaps

- Keep full GD parity tracked as `PHPT-DATA-GD`.
- Expand only through focused fixtures and approved image dependency policy.
