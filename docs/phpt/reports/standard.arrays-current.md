# standard.arrays Current Focus Report

Core values/arrays/strings branch focused standard array builtin
verification.

## Scope

- Selected upstream and generated PHPTs for the selected array builtin list.
- `count`, `array_key_exists`, `array_keys`, `array_values`, `array_merge`,
  `array_slice`, `array_splice`, `array_column`, `in_array`, `array_search`,
  `array_unique`, `range`, and deterministic `sort`/`asort`/`ksort`.

## Selected Manifest

- `tests/phpt/manifests/modules/standard.arrays.selected.jsonl`
- 17 selected fixtures: 5 upstream php-src fixtures and 12 generated fixtures.
- New generated outputs were captured from
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

## Before/After

Before this this pass, the selected gate was already green for 10 fixtures
covering `count`, `array_keys`, `array_values`, `array_merge`, and
`array_slice`. This pass promotes the rest of the selected builtin list into
selected PHPT coverage.

| Check | Before | After |
| --- | ---: | ---: |
| `standard.arrays` selected PHPTs | 10 PASS | 17 PASS |

## Implemented Builtins

- `count` and `sizeof`
- `array_key_exists` and `key_exists`
- `array_keys` and `array_values`
- `array_merge`
- `array_slice` and `array_splice`
- `array_column`
- `in_array` and `array_search`
- `array_unique`
- `range`
- deterministic `sort`, `asort`, and `ksort`

## Callback Builtins Status

- VM callable dispatch exists for callback-heavy helpers such as `array_map`,
  `array_filter`, `array_walk`, callback sorting, and multisort paths, but the
  the selected gate PHPT gate keeps callback-heavy breadth as follow-up coverage.

## Remaining Gaps

- Full upstream `ext/standard/tests/array` coverage remains much broader than
  the selected slice.
- Object-heavy, reference-sensitive, and callback-heavy array fixtures should be
  promoted in separate focused modules or follow-up slices.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.arrays`: PASS, reference 17 PASS and target 17 PASS.
- `nix develop -c cargo test -p php_runtime`: PASS, 187 tests.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS.
- The oracle-backed `verify-stdlib` diff reports had no skips: stdlib 36 PASS / 6 known gaps, streams 2 PASS, json-pcre-date 3 PASS, spl-reflection 2 PASS.
