# PHPT Green 6B Extension Harvest

Branch: `phpt-green/extension-harvest-json-pcre-date`

Oracle used for strict PHPT runs: PHP 8.5.7 from
`/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

The branch-local oracle was bootstrapped and built, but its generated
`main/php_config.h` did not match the php-src source-integrity manifest on this
Darwin host. The sibling pinned oracle was therefore used for strict PHPT
inventory and promotion gates; those runs verified all 24,475 manifest entries.

## Fresh Inventory

Initial selected gates were run with reuse disabled before promotion. All listed
modules were green for reference and target:

| Module | Initial selected green rows |
| --- | ---: |
| `json` | 24 |
| `pcre` | 11 |
| `date` | 9 |
| `xml` | 5 |
| `simplexml` | 4 |
| `intl` | 5 |
| `phar` | 6 |
| `session` | 7 |
| `mysqli` | 5 |
| `pdo` | 4 |
| `pdo_sqlite` | 3 |
| `sqlite3` | 2 |
| `curl` | 7 |
| `openssl` | 4 |
| `wp.db-network` | 10 |

## Promoted Rows

- `ext/date/tests/DateInterval_format.phpt`
- `ext/phar/tests/phar_get_supportedcomp3.phpt`

## Current Selected Counts

| Module | Selected outcomes after promotion | Notes |
| --- | ---: | --- |
| `json` | 24 PASS | unchanged |
| `pcre` | 11 PASS | unchanged |
| `date` | 10 PASS | `DateInterval::format()` now supports selected padded upstream fields. |
| `xml` | 5 PASS | unchanged |
| `simplexml` | 4 PASS | unchanged |
| `intl` | 5 PASS | unchanged |
| `phar` | 3 PASS, 4 SKIP | zlib-only `Phar::getSupportedCompression()` now executes as PASS; bz2 and optional OpenSSL rows remain skipped when unavailable. |
| `session` | 7 PASS | unchanged |
| `mysqli` | 5 PASS | unchanged |
| `pdo` | 4 PASS | unchanged |
| `pdo_sqlite` | 3 PASS | unchanged |
| `sqlite3` | 2 PASS | unchanged |
| `curl` | 5 PASS, 2 SKIP | unchanged |
| `openssl` | 4 PASS | unchanged |
| `wp.db-network` | 7 PASS, 3 SKIP | unchanged |

## Implementation Notes

- `DateInterval::format()` handles uppercase padded interval fields
  `%Y`, `%M`, `%D`, `%H`, `%I`, and `%S` alongside the existing unpadded
  fields.
- `Phar::getSupportedCompression()` now derives `GZ` and `BZIP2` from the
  enabled standard-library extension registry instead of advertising BZIP2
  unconditionally.
- No selected rows were removed, and no reference PASS row was converted to a
  target SKIP.

## Probed But Not Promoted

These rows were run against reference and target with reuse disabled and left
out because they still expose broader behavior or byte-parity gaps:

- `ext/json/tests/json_decode_error.phpt`
- `ext/json/tests/json_decode_exceptions.phpt`
- `ext/json/tests/pass001.phpt`
- `ext/json/tests/serialize.phpt`
- `ext/json/tests/json_exceptions_error_clearing.phpt`
- `ext/pcre/tests/preg_match_all_basic.phpt`
- `ext/pcre/tests/002.phpt`
- `ext/pcre/tests/003.phpt`
- `ext/pcre/tests/grep2.phpt`
- `ext/pcre/tests/preg_replace_basic.phpt`
- `ext/pcre/tests/preg_replace.phpt`
- `ext/date/tests/DateTimeZone_construct_basic.phpt`
- `ext/date/tests/DateTimeZone_compare.phpt`
- `ext/date/tests/DateTimeZone_getOffset_basic1.phpt`
- `ext/date/tests/DateTime_format_basic1.phpt`
- `ext/date/tests/DateTime_construct_basic1.phpt`
- `ext/intl/tests/locale_get_primary_language.phpt`
- `ext/session/tests/session_id_basic2.phpt`

## Verification

Focused checks:

- `nix develop -c cargo test -p php_runtime interval_format_supports_unpadded_and_padded_fields`
- `nix develop -c cargo test -p php_vm phar_supported_compression_follows_loaded_capabilities`
- `nix develop -c just phpt-dev-build`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 PHPT_MANIFEST=target/phpt-6b-probes/date-candidates.jsonl nix develop -c just phpt-dev-module MODULE=date` (expected non-green probe bundle: 1 PASS, 5 FAIL target)
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 PHPT_MANIFEST=target/phpt-6b-probes/phar-candidates.jsonl nix develop -c just phpt-dev-module MODULE=phar`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=date`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=phar`

Final required checks:

- `nix develop -c cargo fmt --check`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c cargo test -p php_std`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=<module>` for `json`, `pcre`, `date`, `xml`, `simplexml`, `intl`, `phar`, `session`, `mysqli`, `pdo`, `pdo_sqlite`, `sqlite3`, `curl`, `openssl`, and `wp.db-network`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-phpt`
