# Operators Conversions Behavior Notes

Prompt 09.12 covers the selected PHPT batch for arithmetic and bitwise
operators, comparisons, scalar conversions, numeric strings, concat,
increment/decrement, and invalid operand warnings.

## Source Notes

- `Zend/zend_operators.c` is the central behavior source for scalar
  conversion and operator helpers. Relevant anchors include `ZEND_ADD`,
  `ZEND_SUB`, `ZEND_MUL`, `ZEND_DIV`, `ZEND_MOD`, `ZEND_CONCAT`,
  `zend_compare`, `increment_function`, `decrement_function`, and
  `is_numeric_string_ex`.
- `Zend/zend_vm_def.h` contains the VM handlers for arithmetic, concat,
  comparison, boolean conversion, and increment/decrement opcodes. The
  selected PHPTs exercise these through ordinary userland expressions.
- `Zend/tests/string_to_number_comparison.phpt` captures PHP 8 numeric-string
  comparison behavior: full numeric strings compare numerically, leading
  numeric strings compare lexically against numbers, non-finite float labels
  compare by their PHP string forms, and `ini_set('precision', 0)` changes
  float string conversion before comparison.
- `Zend/tests/type_coercion/type_casts/cast_to_int.phpt` and
  `cast_to_double.phpt` show object numeric casts warning while returning
  `1` or `1.0`. The object's `__toString` result is not used for these numeric
  casts.
- `Zend/tests/in-de-crement/increment_001_64bit.phpt` shows 64-bit integer
  increment overflow promoting to float and `var_dump` using PHP's scientific
  double display for the overflowed value.

## Implementation Notes

- `php_ir` now lowers bitwise binary and compound-assignment operators into
  explicit IR operations, and `php_semantics` recognizes their token text
  instead of falling back to a placeholder `binary` operator.
- `php_vm` executes integer arithmetic overflow by promoting add/sub/mul
  results to float. Bitwise integer and string operations are implemented for
  the selected module, including string bytewise `&`, `|`, and `^`.
- `php_std` exposes the core constants needed by the selected PHPTs:
  `PHP_INT_MAX`, `PHP_INT_MIN`, `PHP_INT_SIZE`, `INF`, and `NAN`.
- Runtime conversion now formats non-finite floats as PHP strings for loose
  comparison, keeps `NAN == "NAN"` false, and honors request-local
  `precision=0` for float-to-string conversion inside VM execution.
- `json_encode` now drops `.0` for finite integral floats unless
  `JSON_PRESERVE_ZERO_FRACTION` is set, matching the reference output used by
  `string_to_number_comparison.phpt`.
- VM object casts to `int` and `float` emit a PHP-visible warning and return
  `1`/`1.0`, matching the selected upstream cast fixtures.
