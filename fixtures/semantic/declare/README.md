# declare

Purpose: `declare(...)` HIR metadata and file-level directive summaries.

Example rules: `strict_types`, `ticks`, `encoding`, invalid strict_types
values, and strict_types placement.

Reference classification: accepted for valid directives; rejected for invalid
strict_types value or placement.

Rust diagnostic IDs: `E_PHP_INVALID_STRICT_TYPES_DECLARE`,
`E_PHP_STRICT_TYPES_DECLARE_NOT_FIRST`.

Known gaps: runtime effects of strict_types, ticks, and encoding are deferred.
