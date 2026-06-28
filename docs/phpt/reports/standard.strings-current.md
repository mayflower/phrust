# standard.strings Current Focus Report

Core values/arrays/strings branch Prompt 2E focused standard string builtin
verification.

## Scope

- Selected generated PHPTs for the Prompt 2E standard string builtin list.
- `strlen`, `substr`, `strpos`, `str_contains`, `trim`, `explode`, `implode`,
  `sprintf`, `printf`, `str_replace`, `strtolower`, `strtoupper`, and existing
  `strtok` state coverage.

## Selected Manifest

- `tests/phpt/manifests/modules/standard.strings.selected.jsonl`
- 16 selected generated fixtures.
- New Prompt 2E generated output was captured from
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

## Before/After

Before this Prompt 2E pass, the selected standard string gate was green for 15
fixtures but did not cover `str_replace`, `strtolower`, or `strtoupper`. This
pass promotes those functions into selected PHPT coverage.

| Check | Before | After |
| --- | ---: | ---: |
| `standard.strings` selected PHPTs | 15 PASS | 16 PASS |

## Implemented Builtins

- `strlen`
- `substr`
- `strpos`
- `str_contains`
- `trim`
- `explode`
- `implode`
- `sprintf` and `printf`
- `str_replace`
- `strtolower` and `strtoupper`
- `strtok`

## Remaining Gaps

- Full upstream `ext/standard/tests/strings` coverage remains broader than the
  focused selected slice.
- Additional formatting, encoding, uncommon flag, and locale-sensitive behavior
  remains follow-up work.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings`: PASS, reference 16 PASS and target 16 PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS.
- The oracle-backed `verify-stdlib` run included stdlib, streams,
  json-pcre-date, and spl-reflection PHPT module checks with no unexpected
  failures.
