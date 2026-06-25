# operators.conversions

- Priority: 5
- Selected manifest: `tests/phpt/manifests/modules/operators.conversions.selected.jsonl`
- Current counts: 7 PASS, 0 SKIP, 0 FAIL, 0 BORK from 7 selected cases

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

- `tests/phpt/generated/operators.conversions/regression-operators_scalar_matrix-8930bdcfc752.phpt`
- `tests/phpt/generated/operators.conversions/regression-string_number_precision-c50f1cd9d9a3.phpt`
- `tests/phpt/generated/operators.conversions/regression-object_numeric_casts-5a2a4047d1ed.phpt`
- `tests/phpt/generated/operators.conversions/smoke-leading-numeric-arithmetic-warning-417523e69412.phpt`
- `Zend/tests/add_005.phpt`
- `Zend/tests/div_001.phpt`
- `Zend/tests/concat/concat_002.phpt`

## Relevant php-src Source Areas

- `Zend/tests/`
- `crates/php_runtime/`
- `crates/php_vm/`

## Target Gates

- `nix develop -c just phpt-module MODULE=operators.conversions`

## Known Gaps

- Array union (`+`) remains outside this scalar conversion gate.
- Full non-numeric operand `TypeError` catchability remains outside this gate.
- Array/object concat warnings beyond the existing object `__toString` smoke remain outside this gate.
- Pipe, nullsafe, property-hook, fiber, and performance-only PHPTs are routed to their own modules.

## Next Step

Keep the selected scalar conversion gate green while later modules expand arrays, objects, and diagnostics.
