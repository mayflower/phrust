# Runtime Diagnostics And Conversions

This document maps the central runtime diagnostic, event, numeric-string,
conversion, and comparison surfaces used by `php_runtime` and `php_vm`.

## Diagnostic/Event Model

`crates/php_runtime/src/diagnostic.rs` owns the shared event carrier:

- `RuntimeDiagnostic` stores the stable diagnostic ID, severity, PHP source
  span, stack frames, optional payload, and optional PHP-reference
  classification.
- `RuntimeEventKind` normalizes diagnostics into PHP-visible event families:
  warning, notice, deprecation, catchable exception, fatal error, and
  unsupported feature.
- `PhpReferenceClassification` maps diagnostics to PHP concepts such as
  `TypeError`, `ValueError`, `ArgumentCountError`, `DivisionByZeroError`,
  `UnhandledMatchError`, warnings, deprecations, fatal errors, and explicit
  unsupported behavior.
- `RuntimeDiagnostic::php_reference_or_inferred()` preserves explicitly set
  classifications and infers known legacy IDs such as
  `E_PHP_RUNTIME_BUILTIN_TYPE` and `E_PHP_VM_UNHANDLED_MATCH`.

High-impact constructor helpers live beside the model:

- `undefined_variable_warning`
- `array_to_string_warning`
- `leading_numeric_string_warning`
- `non_numeric_string_type_error`
- `type_error_mvp`
- `value_error_mvp`
- `argument_count_error_mvp`
- `division_by_zero_mvp`
- `unhandled_match_error_mvp`
- `undefined_function`
- `unsupported_feature`

PHP-formatted warning output is rendered by
`crates/php_runtime/src/error_output.rs` and emitted by the VM through
`emit_vm_diagnostic`. Builtin warning/deprecation emission in
`crates/php_runtime/src/builtins/context.rs` records the same shared
classification data.

## Conversion APIs

`crates/php_runtime/src/convert.rs` owns supported PHP conversion and
comparison behavior. The existing short names remain available, and the
explicit PHP semantic names are exported for new code:

- `to_bool_php`
- `to_int_php`
- `to_float_php`
- `to_string_php`
- `to_array_php`
- `to_object_php`
- `to_number_php`
- `to_arithmetic_number_php`
- `identical_php`
- `equal_php`
- `compare_php`

Supported scalar casts cover null, booleans, integers, floats, strings, arrays,
resources, and references where the runtime value model already has enough
information. Unsupported object casts and object numeric conversions remain
stable gaps instead of guessing object-storage behavior outside the VM.

## Numeric Strings And Comparisons

`crates/php_runtime/src/numeric_string.rs` classifies PHP strings as:

- full integer strings;
- full float strings;
- leading numeric strings;
- non-numeric strings.

Arithmetic conversion uses the classification and preserves a
`leading_numeric_string` flag so VM callers can emit
`A non-numeric value encountered` while still using the numeric prefix.
Comparison uses PHP 8 style behavior: string/string and number/string
comparisons use numeric comparison only for full numeric strings, while leading
numeric strings fall back to string comparison outside arithmetic.

## VM Integration

`crates/php_vm/src/vm/mod.rs` routes shared runtime diagnostics into observable
PHP behavior:

- undefined variables emit PHP-formatted warnings and continue as null;
- array-to-string string contexts emit `Warning: Array to string conversion`
  and produce `Array`;
- leading numeric-string arithmetic emits the non-numeric warning and continues;
- non-numeric arithmetic maps to `TypeError`;
- selected builtin arity/type/value diagnostics map to
  `ArgumentCountError`, `TypeError`, and `ValueError`;
- division by zero maps to `DivisionByZeroError`;
- unmatched `match` expressions map uncaught output to `UnhandledMatchError`;
- unsupported features stay explicit diagnostics.

## Known Gaps

Remaining compatibility gaps are tracked in `docs/runtime/known-gaps.md` and
`docs/known_gaps/runtime.jsonl`. The active gaps for this area are exact
engine wording, full stack trace formatting, catch matching for the lowered
no-arm `match` runtime-error path, complete builtin arginfo parity,
resource/extension conversion matrices, and object conversion behavior that
requires broader object/standard-library support.
