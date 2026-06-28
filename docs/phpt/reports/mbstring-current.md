# mbstring Current Focus Report

moved mbstring from disabled platform stubs to a bounded UTF-8 MVP.

## Policy

`mbstring` is `required-composer` with an MVP implementation for the selected
surface:

- `extension_loaded("mbstring")` is true.
- `function_exists()` is true for `mb_strlen`, `mb_substr`,
  `mb_strtolower`, `mb_strtoupper`, `mb_detect_encoding`,
  `mb_check_encoding`, `mb_internal_encoding`, `mb_convert_encoding`, and
  `mb_strpos`.
- Unsupported mbstring functions outside the selected surface remain absent
  rather than pretending full extension parity.

The MVP covers UTF-8 length, substring, case conversion, UTF-8/ASCII detection,
request-local internal encoding, and UTF-8 to UTF-8 conversion. Legacy
encodings, mbregex, Oniguruma, locale-sensitive behavior beyond the selected
fixtures, and the broader upstream `ext/mbstring` corpus remain known gaps.

## Oracle Note

The default project oracle at
`/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` was built
without mbstring. selected fixture expectations were captured from a temporary
read-only PHP 8.5.7 clone at `/tmp/php-src-mbstring-oracle/sapi/cli/php` built
from the same php-src checkout with `--enable-mbstring --disable-mbregex`.

## Classification

| Category | PHPT ownership |
| --- | --- |
| required-core | none in this branch |
| required-composer | generated platform checks and bounded UTF-8 MVP fixtures |
| required-framework | future guarded probes if a framework fixture proves the need |
| optional | upstream common-function basics after selected fixture parity is expanded |
| out-of-scope | exhaustive encoding conversion, regex, mail/MIME, HTTP/input/output translation, exif dependency cases |
| MVP | selected bounded UTF-8 and ASCII surface |
| real-implementation-required | full mbstring parity outside the selected MVP |
| already-implemented | selected generated PHPTs and runtime functions listed above |

## Selected Manifest

- `tests/phpt/manifests/modules/mbstring.selected.jsonl`
- 3 generated fixtures under `tests/phpt/generated/mbstring/`

## Corpus Snapshot

Committed baseline counts for the broader mbstring-owned corpus:

| Outcome | Count |
| --- | ---: |
| PASS | 3 |
| SKIP | 36 |
| FAIL | 360 |
| BORK | 21 |
| Known non-green | 414 |

## Selected Fixtures

- `tests/phpt/generated/mbstring/platform-checks.phpt`
- `tests/phpt/generated/mbstring/utf8-common-functions.phpt`
- `tests/phpt/generated/mbstring/utf8-encoding-state.phpt`

## Implementation Summary

- Runtime builtin registry exposes only the selected mbstring functions.
- `mb_internal_encoding` is carried through request-local execution state.
- `mb_strlen`, `mb_substr`, `mb_strtolower`, and `mb_strtoupper` use bounded
  UTF-8 behavior backed by focused PHP 8.5.7 oracle output.
- `mb_detect_encoding` and `mb_check_encoding` accept UTF-8 and ASCII aliases.
- `mb_convert_encoding` supports the selected UTF-8 to UTF-8 no-op case only.
- `mb_strpos` supports selected UTF-8/ASCII string position checks.

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `REFERENCE_PHP=/tmp/php-src-mbstring-oracle/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mbstring`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`

## Verification

Latest verification:

- `nix develop -c cargo test -p php_runtime mbstring`: PASS, 2 selected tests.
- `nix develop -c cargo test -p php_std mbstring`: PASS, 1 selected test.
- `nix develop -c cargo test -p php_vm symbol_introspection_exposes_bounded_mbstring_mvp`: PASS, 1 selected test.
- `nix develop -c just phpt-dev-build`: PASS.
- `REFERENCE_PHP=/tmp/php-src-mbstring-oracle/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mbstring`: PASS, reference 3 PASS and target 3 PASS.
