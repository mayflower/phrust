# declarations

Purpose: declaration-table collection for top-level and conditional symbols.

Example rules: functions, constants, class-like declarations, conditional
functions, duplicate functions, and conservative same-file duplicate classes.

Reference classification: duplicate functions are rejected by PHP lint.
Duplicate classes are a Rust semantic-only reject for deterministic same-file
declaration tables.

Rust diagnostic IDs: `E_PHP_DUPLICATE_DECLARATION`.

Known gaps: cross-file duplicates, autoloading, and include-defined symbols are
not collected.
