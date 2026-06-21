# const_expr

Purpose: constant-expression candidate collection and conservative structural
validation.

Example rules: literals, arrays, class constant fetches, folded literals,
invalid variables/calls, and PHP 8.5 accepted closures, casts, first-class
callables, and `new`.

Reference classification: accepted for allowed constant expressions; rejected
for structurally invalid forms.

Rust diagnostic IDs: `E_PHP_INVALID_CONST_EXPR`,
`E_PHP_ATTRIBUTE_ARGUMENT_NOT_CONST_EXPR`.

Known gaps: runtime constant lookup and object/value evaluation are deferred.
