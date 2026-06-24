# performance JIT Fixtures

These fixtures document the source shapes covered by the `php_jit`
eligibility tests.

- `eligible-int-add.php`: primitive int leaf function shape accepted by the
  conservative eligibility analyzer.
- `rejected-dynamic.php`: calls and array operations rejected by the analyzer.

The JIT remains default-off and these fixtures are not native-code execution
tests.
