# standard.math Current Focus Report

Core values/arrays/strings branch focused standard math verification.

## Scope

- Selected upstream and generated PHPTs for math and numeric standard builtins.
- Existing math coverage is reused as the selected math acceptance
  surface.

## Selected Manifest

- `tests/phpt/manifests/modules/standard.math.selected.jsonl`
- 172 selected fixtures.

## Current Status

| Check | Result |
| --- | ---: |
| `standard.math` selected PHPTs | 161 PASS / 11 SKIP / 0 FAIL |

## Implemented Builtins

- Math module builtins already present in the selected module gate, including
  trigonometric, hyperbolic, logarithmic, exponential, rounding, base
  conversion, and numeric edge-case coverage.

## Remaining Gaps

- The 11 selected SKIPs are preserved selected-gate skips, not target failures.
- Broader numeric edge cases and cross-layer blockers remain backlog work when
  expanding beyond this focused module.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.math`: PASS, 172 selected fixtures with no non-green target outcomes.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`: PASS.
