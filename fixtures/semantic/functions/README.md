# functions

Purpose: function-like signatures, flags, returns, closure captures, and
generator metadata.

Example rules: duplicate parameters, variadic ordering, defaults,
by-reference parameters, arrow functions, closure `use`, void/never returns,
and generator detection.

Reference classification: accepted for valid signatures; rejected for duplicate
parameters, invalid variadics, invalid closure captures, and invalid returns.

Rust diagnostic IDs: `E_PHP_DUPLICATE_PARAMETER`,
`E_PHP_VARIADIC_PARAMETER_NOT_LAST`, `E_PHP_INVALID_PARAMETER_DEFAULT`,
`E_PHP_CLOSURE_USE_DUPLICATES_PARAMETER`,
`E_PHP_DUPLICATE_CLOSURE_USE_VARIABLE`,
`E_PHP_RETURN_VALUE_FROM_VOID_FUNCTION`, `E_PHP_RETURN_FROM_NEVER_FUNCTION`.

Known gaps: generator runtime return-value behavior and `$this` runtime
availability are deferred.
