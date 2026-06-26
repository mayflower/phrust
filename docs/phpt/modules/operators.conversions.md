# operators.conversions

- Priority: 5
- Selected manifest: `tests/phpt/manifests/modules/operators.conversions.selected.jsonl`
- Current counts: 4 PASS, 0 SKIP, 0 FAIL, 0 BORK from 4 selected
  generated candidates

## Scope

- arithmetic
- bitwise operators
- comparison
- boolean conversion
- numeric-string conversion
- concat
- assignment operators
- increment/decrement
- leading numeric string warnings
- object numeric casts

## Non-Scope

- array union semantics
- array/object concat beyond __toString smoke coverage
- full TypeError/Throwable catch semantics for non-numeric operands
- pipe operator
- nullsafe operator
- property hooks
- fiber error suppression
- performance-only concat stress

## Relevant PHPT Paths

- `tests/phpt/generated/operators.conversions/regression-object_numeric_casts-5a2a4047d1ed.phpt`
- `tests/phpt/generated/operators.conversions/regression-operators_scalar_matrix-8930bdcfc752.phpt`
- `tests/phpt/generated/operators.conversions/regression-string_number_precision-c50f1cd9d9a3.phpt`
- `tests/phpt/generated/operators.conversions/smoke-leading-numeric-arithmetic-warning-417523e69412.phpt`

Each generated PHPT carries provenance for the original `Zend/tests/...` source
and hash in its `--DESCRIPTION--` section and in the selected manifest.

## Relevant php-src Source Areas

- `Zend/tests/`
- `crates/php_runtime/`
- `crates/php_vm/`

## Target Gates

- `nix develop -c just phpt-module MODULE=operators.conversions`

Last focused run on 2026-06-26:

- Selected module gate:
  `nix develop -c just phpt-module MODULE=operators.conversions`
  - Reference: 4 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Target: 4 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Source integrity: 24476 php-src manifest entries verified

Closed selected-gate gaps:

- precision-sensitive numeric-string comparison after `ini_set("precision", 0)`
- PHP-compatible successful warning output for object numeric casts in
  `phrust-php`
- leading numeric string arithmetic warnings
- arithmetic, bitwise, comparison, concat, truthiness, and overflow smoke
  coverage

## Known Gaps

The full 129-candidate source cluster still contains broad PHP feature areas
that are outside this selected scalar conversion gate, including pipe operator,
nullsafe operator, property hooks, fiber error suppression, broad unsupported
operand diagnostics, and performance-only concat stress cases.

## Next Step

Keep the selected scalar conversion gate green while later modules expand
unsupported operand diagnostics and advanced operator families.
