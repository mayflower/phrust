# types

Purpose: HIR type lowering and reference-safe type diagnostics.

Example rules: parameter, return, and property types; nullable types; union,
intersection, and DNF types; special names such as `self`, `parent`, and
`static`.

Reference classification: accepted for valid contexts; rejected for invalid
`void`, `never`, `static`, `self`, `parent`, `callable`, and duplicate type
alternatives.

Rust diagnostic IDs: `E_PHP_INVALID_TYPE_VOID_CONTEXT`,
`E_PHP_INVALID_TYPE_NEVER_CONTEXT`, `E_PHP_INVALID_TYPE_STATIC_CONTEXT`,
`E_PHP_INVALID_TYPE_SELF_CONTEXT`, `E_PHP_INVALID_TYPE_PARENT_CONTEXT`,
`E_PHP_INVALID_TYPE_CALLABLE_CONTEXT`, `E_PHP_DUPLICATE_TYPE_ALTERNATIVE`.

Known gaps: type compatibility and runtime class existence checks are deferred.
