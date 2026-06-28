# WordPress Stdlib Current Report

## Selected Functions

- Standard helpers: selected common array, string, and variable functions.
- Hash: `hash`, `hash_hmac`, `hash_equals`, `hash_algos`, plus existing
  `md5` and `sha1` wrappers.
- Filter: `filter_var`, deterministic unavailable `filter_input`, selected
  validation/sanitization constants and flags.
- Text: bounded `mb_*` UTF-8 helpers and `iconv` UTF-8/ASCII helpers.
- Compression/archive: `gzencode`, `gzdecode`, `gzcompress`,
  `gzuncompress`, `zlib_decode`, and `ZipArchive` read/extract MVP.
- Media: `finfo_open`, `finfo_file`, `finfo_buffer`, `mime_content_type`,
  `exif_imagetype`, and `getimagesize` for selected image formats.

## Current Failures

- None in the selected `wp.stdlib` target run. The reference oracle used for
  local verification skips optional-extension fixtures when its PHP binary was
  built without those extensions; the target executes and passes them.

## Implementation Order

1. Hash, filter, iconv, zlib, fileinfo, and exif module wiring.
2. Hash algorithm expansion and `hash_algos`/`hash_equals`.
3. Bounded filter validation and sanitization.
4. `mb_strpos` and iconv UTF-8/ASCII helpers.
5. zlib compression helpers.
6. lightweight fileinfo/EXIF media recognition.
7. ZipArchive local read/extract MVP.
8. `wp.stdlib` generated PHPT harness and docs.

## Remaining Gaps

- OpenSSL and sodium APIs are not implemented in this branch.
- Full encoding databases, libmagic parity, full EXIF tags, ZipArchive writing,
  and exhaustive filter options remain out of scope.
