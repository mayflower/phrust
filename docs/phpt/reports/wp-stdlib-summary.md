# WordPress Stdlib Summary

This branch adds a WordPress-focused stdlib harness and implementation slice for
high-usage PHP helpers across standard functions, hash, filter, text encoding,
media detection, zlib, and zip archive reads.

## Implemented Coverage

- `wp.stdlib` selected PHPT manifest with 7 generated fixtures.
- Dedicated runtime modules for `hash`, `filter`, `iconv`, `zlib`,
  `fileinfo`, and `exif`.
- ZipArchive MVP in the VM for local archive open, count, read, stat,
  constrained extraction, and close.
- `php_std` metadata for the newly implemented extension surfaces.

## Merge Risks

- `ZipArchive` support is intentionally read/extract only and uses the VM's
  internal object path.
- `filter_input` returns `NULL` because no SAPI input model is introduced.
- Media detection is lightweight and deterministic, not full upstream parity.

## Required Closeout Gates

- `nix develop -c just fmt`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_std`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
