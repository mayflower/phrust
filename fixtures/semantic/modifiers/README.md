# modifiers

Purpose: declaration modifier validation and modifier metadata.

Example rules: class, method, and property modifiers; asymmetric visibility;
readonly promotion; and property hooks.

Reference classification: accepted for valid combinations; rejected for
duplicates or incompatible/invalid combinations.

Rust diagnostic IDs: `E_PHP_DUPLICATE_MODIFIER`,
`E_PHP_INCOMPATIBLE_MODIFIERS`.

Known gaps: runtime property hook behavior is deferred.
