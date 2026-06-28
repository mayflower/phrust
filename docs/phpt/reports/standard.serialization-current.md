# standard.serialization Current Focus Report

Core values/arrays/strings branch focused standard serialization
verification.

## Scope

- Selected PHPTs for `serialize` and `unserialize`.
- Scalar, array, and simple object persistence covered by the selected gate.

## Selected Manifest

- `tests/phpt/manifests/modules/standard.serialization.selected.jsonl`
- 5 selected fixtures.

## Current Status

| Check | Result |
| --- | ---: |
| `standard.serialization` selected PHPTs | 5 PASS / 0 FAIL |

## Implemented Builtins

- `serialize`
- `unserialize`
- Scalar value persistence
- Array value persistence
- Simple object persistence in the selected fixture surface

## Remaining Gaps

- `R`/`r` reference identity records are intentionally documented as
  `STDLIB-GAP-SERIALIZE-REFERENCES`.
- `allowed_classes`, magic hooks, resources, and deep object/reference graphs
  remain outside the selected focused gate.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.serialization`: PASS, reference 5 PASS and target 5 PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS.
