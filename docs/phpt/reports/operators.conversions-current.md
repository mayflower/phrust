# operators.conversions Current Focus Report

Core values/arrays/strings branch focused scalar and operator verification.

## Scope

- Numeric string classification and arithmetic conversion.
- Leading numeric string warnings.
- Scalar arithmetic, bitwise, comparison, concat, truthiness, and overflow.
- Object numeric casts that warn and produce PHP-compatible fallback values.
- Invalid operand TypeError behavior through the runtime diagnostic channel.

## Selected Manifest

- `tests/phpt/manifests/modules/operators.conversions.selected.jsonl`
- 5 generated fixtures under `tests/phpt/generated/operators.conversions/`

## Selected Fixtures

- `tests/phpt/generated/operators.conversions/regression-object_numeric_casts-5a2a4047d1ed.phpt`
- `tests/phpt/generated/operators.conversions/regression-operators_scalar_matrix-8930bdcfc752.phpt`
- `tests/phpt/generated/operators.conversions/regression-string_number_precision-c50f1cd9d9a3.phpt`
- `tests/phpt/generated/operators.conversions/smoke-leading-numeric-arithmetic-warning-417523e69412.phpt`
- `tests/phpt/generated/operators.conversions/smoke-invalid-array-add-operand-ce09515759f4.phpt`

## Before/After

The branch already had a green 4-fixture selected operator/conversion gate before
this Prompt 2A pass. This pass added the missing dedicated invalid-operand PHPT
from `Zend/tests/add_004.phpt` and expanded the selected gate to 5 fixtures.

| Check | Before | After |
| --- | ---: | ---: |
| `operators.conversions` selected PHPTs | 4 PASS | 5 PASS |
| `zend.basic` selected PHPTs | 10 PASS | 10 PASS |

## Remaining Scalar Gaps

- The selected invalid-operand fixture covers `array + int`.
- Broader unsupported operand matrices, including bitwise array operands and
  full non-numeric string TypeError parity, remain outside this selected passing
  slice until their runtime diagnostic paths match the PHP oracle.

## Verification

Latest branch verification:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=operators.conversions`: PASS, reference 5 PASS and target 5 PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zend.basic`: PASS, reference 10 PASS and target 10 PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c cargo test -p php_runtime`: PASS, 186 tests.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c cargo test -p php_vm`: PASS, 352 tests.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-runtime`: PASS.
