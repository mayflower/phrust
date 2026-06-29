# phar

- Strategy: read-only local PHAR MVP
- Classification: real-implementation-required for Composer PHAR mode
- Selected manifest: `tests/phpt/manifests/modules/phar.selected.jsonl`

## Implemented Scope

- `extension_loaded("phar")` reports true through the standard-library
  extension registry.
- `class_exists("Phar")`, `class_exists("PharData")`, and
  `class_exists("PharFileInfo")` report true.
- `new Phar($path)` opens and validates a local uncompressed `.phar` archive
  under runtime filesystem capabilities.
- `phar://local.phar/file` supports read-only `file_get_contents`, `fopen`,
  `stream_get_contents`, and `include` for uncompressed file entries.
- `include_once` / `require_once` tracking uses stable synthetic `phar://`
  paths.

## Fixture

- `tests/phpt/generated/phar/platform-checks.phpt` writes a deterministic
  uncompressed archive with `hex2bin()`, reads `data.txt` through
  `file_get_contents` and `fopen`, includes `lib/hello.php`, constructs
  `Phar`, and removes the temporary archive.

## Remaining Gaps

- PHAR archive writing and mutation APIs are not implemented.
- Signature validation/enforcement, compression, tar/zip `PharData`, archive
  iteration, metadata APIs, and `PharFileInfo` object creation remain known
  gaps.
- `phar://` metadata/stat functions are intentionally narrower than full PHP
  stream-wrapper parity.

## Request Filesystem Overlay

The `wp.request-filesystem` overlay exercises `phar://` package reads through
`file_exists`, `is_file`, `file_get_contents`, and include behavior. It keeps
the same read-only local PHAR boundary and does not claim writable archives,
alias registration, or full metadata/stat parity.

## Source References

- `ext/phar/phar.c`
- `ext/phar/phar_object.stub.php`
- `ext/phar/tests/`

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=phar`
- `nix develop -c just composer-smoke`
