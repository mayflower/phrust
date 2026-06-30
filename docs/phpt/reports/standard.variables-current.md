# standard.variables Current Focus Report

Core values/arrays/strings branch focused standard variable builtin
verification.

## Scope

- Selected PHPTs for variable inspection and debug-output builtins.
- `gettype`, `is_*`, `var_dump`, `print_r`, and `var_export` behavior covered
  by the focused gate.

## Selected Manifest

- `tests/phpt/manifests/modules/standard.variables.selected.jsonl`
- 33 selected fixtures.

## Current Status

| Check | Result |
| --- | ---: |
| `standard.variables` selected PHPTs | 32 PASS / 1 SKIP / 0 FAIL |

## Implemented Builtins

- `gettype`
- `is_*` predicates covered by the selected gate
- `var_dump`
- `print_r`
- `var_export`

## Remaining Gaps

- Anonymous-class `get_debug_type`, `settype`, full object visibility, magic
  behavior, reference formatting, and broader debug-output matrix coverage
  remain outside this selected gate.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.variables`: PASS, 33 selected fixtures with 32 PASS / 1 SKIP and no non-green target outcomes.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS.
