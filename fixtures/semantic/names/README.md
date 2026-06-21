# names

Purpose: import collection and static name-resolution metadata.

Example rules: class, function, and const imports; group imports; duplicate
aliases; fully qualified names; namespaced function fallback metadata.

Reference classification: accepted except duplicate import aliases.

Rust diagnostic IDs: `E_PHP_DUPLICATE_USE_ALIAS`.

Known gaps: runtime fallback lookup is metadata only.
