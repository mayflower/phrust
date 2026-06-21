# attributes

Purpose: attribute metadata and constant-expression validation for attribute
arguments.

Example rules: attributes on classes, functions, methods, parameters,
properties, enum cases, and PHP 8.5 constant-expression forms.

Reference classification: accepted for valid attributes; rejected for invalid
non-constant attribute arguments.

Rust diagnostic IDs: `E_PHP_ATTRIBUTE_ARGUMENT_NOT_CONST_EXPR`.

Known gaps: attribute class resolution and instantiation are deferred.
