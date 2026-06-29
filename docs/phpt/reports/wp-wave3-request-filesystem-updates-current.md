# WP Wave 3 Request Filesystem Updates

## Scope

- Added focused `wp.request-filesystem` PHPT coverage for WordPress-like
  request/filesystem update primitives.
- Extended runtime support for permission/stat helpers, request-local umask,
  stream context defaults, gzip file handles, and `phar://` file existence
  probes.
- Multipart upload parsing and successful upload moves are owned by
  `php_server` tests and `server-smoke`; the PHPT module includes a skip marker
  because CLI PHPT cannot populate request-local upload metadata.

## Selected Fixtures

- `tests/phpt/generated/wp.request-filesystem/platform-surface.phpt`
- `tests/phpt/generated/wp.request-filesystem/local-permission-stat.phpt`
- `tests/phpt/generated/wp.request-filesystem/temp-directory-stream-context.phpt`
- `tests/phpt/generated/wp.request-filesystem/gzip-file-handles.phpt`
- `tests/phpt/generated/wp.request-filesystem/package-archive-media.phpt`
- `tests/phpt/generated/wp.request-filesystem/upload-cli-surface.phpt`
- `tests/phpt/generated/wp.request-filesystem/multipart-transport-only.phpt`

## Current Counts

- Before this branch: no `wp.request-filesystem` selected module.
- Current selected target: 7 fixtures.
- Current reference run: 4 PASS, 3 SKIP, 0 FAIL, 0 BORK.
- Current target run: 6 PASS, 1 SKIP, 0 FAIL, 0 BORK.
- Current status: selected PHPT gate green on reference and target.

## Validation

- `nix develop -c cargo test -p php_runtime resource`: PASS
- `nix develop -c cargo test -p php_runtime`: PASS
- `nix develop -c cargo test -p php_vm include`: PASS
- `nix develop -c cargo test -p php_vm`: PASS
- `nix develop -c cargo test -p php_server`: PASS
- `nix develop -c cargo test -p php_std`: PASS
- `nix develop -c cargo fmt`: PASS
- `nix develop -c just server-smoke`: PASS
- `nix develop -c just phpt-dev-build`: PASS
- `nix develop -c just verify-runtime`: PASS
- `nix develop -c just verify-stdlib`: PASS
- `nix develop -c just verify-server`: PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just verify-phpt`: PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.request-filesystem`: PASS, reference 4 PASS/3 SKIP and target 6 PASS/1 SKIP
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=filesystem.streams`: PASS, reference 11 PASS and target 11 PASS
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=phar`: PASS, reference 1 PASS and target 1 PASS

## Known Deviations

- `disk_free_space()` and `disk_total_space()` return deterministic positive
  capability-backed values for allowed existing paths instead of host filesystem
  capacity.
- `dir()` is registered and routes through the directory resource model; full
  PHP `Directory` object method parity is not claimed by the selected fixture.
- Multipart upload registry success is verified through server transport tests,
  not CLI PHPT.
- Reference skips `gzip-file-handles.phpt` and `package-archive-media.phpt`
  with the local sibling PHP binary because the relevant optional extensions
  are unavailable there; the target passes both fixtures.
- Source-integrity checks skip in this checkout because no pinned local
  `third_party/php-src` checkout is present. The sibling PHP 8.5.7 reference
  binary was used explicitly for PHPT comparison.
- `verify-stdlib` records reference status as skipped unless `REFERENCE_PHP`
  is set for that specific stdlib coverage preflight; the stdlib docs,
  coverage, tests, and diffs completed successfully.
