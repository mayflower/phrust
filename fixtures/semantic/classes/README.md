# classes

Purpose: class-like HIR, members, class context names, magic methods, and
constructor promotion.

Example rules: extends/implements metadata, methods, properties, constants,
anonymous classes, property hooks, duplicate members, `self`/`parent`/`static`,
magic-method shapes, and promotion restrictions.

Reference classification: accepted for valid class-like declarations; rejected
for duplicate members, invalid promotion, invalid class context names, and
reference-confirmed magic-method errors.

Rust diagnostic IDs: `E_PHP_DUPLICATE_CLASS_MEMBER`,
`E_PHP_INVALID_PROPERTY_PROMOTION`, `E_PHP_INVALID_CLASS_CONTEXT_NAME`,
`E_PHP_INVALID_MAGIC_METHOD_SIGNATURE`.

Known gaps: autoloading, runtime inheritance checks, property hook execution,
and `$this` runtime availability are deferred.
