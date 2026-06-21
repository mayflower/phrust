# enums

Purpose: enum HIR, enum cases, backing types, attributes, and enum-specific
diagnostics.

Example rules: unit enums, backed enums, case values, duplicate cases, and
attribute metadata on enums/cases.

Reference classification: accepted for valid enum declarations; rejected for
missing/extra backing values and duplicate cases.

Rust diagnostic IDs: `E_PHP_ENUM_CASE_VALUE_ON_UNIT_ENUM`,
`E_PHP_ENUM_CASE_MISSING_VALUE_ON_BACKED_ENUM`,
`E_PHP_DUPLICATE_CLASS_MEMBER`.

Known gaps: backing-value type compatibility and duplicate backing values are
deferred.
