# control_flow

Purpose: compile-time control-context validation.

Example rules: `break`, `continue`, top-level `return`, `yield`, and basic
`goto`/label handling.

Reference classification: accepted for valid contexts; rejected for invalid
break/continue/yield and missing labels.

Rust diagnostic IDs: `E_PHP_BREAK_NOT_IN_LOOP_OR_SWITCH`,
`E_PHP_CONTINUE_NOT_IN_LOOP_OR_SWITCH`,
`E_PHP_INVALID_BREAK_CONTINUE_LEVEL`,
`E_PHP_RETURN_OUTSIDE_ALLOWED_CONTEXT`,
`E_PHP_RETURN_VALUE_FROM_VOID_FUNCTION`,
`E_PHP_RETURN_FROM_NEVER_FUNCTION`,
`E_PHP_YIELD_OUTSIDE_FUNCTION`, `E_PHP_GOTO_LABEL_NOT_FOUND`.

Known gaps: `goto-invalid-known-gap.php` documents the current missing jump
into-loop/switch restriction.
