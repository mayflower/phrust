# Symbol Table Model

Symbols are semantic identities distinct from CST nodes, AST views, and source
names. Phase 3 symbol IDs should be stable within one analysis result and
serializable for snapshots.

## Name Kinds

PHP resolves names in distinct namespaces:

- class-like names
- function names
- constant names
- namespace names
- member names
- variable names

Class-like names are case-insensitive for lookup. Function and constant lookup
has PHP-specific fallback behavior that must be represented explicitly rather
than hidden as successful resolution.

## Tables

The semantic frontend should expose:

- declaration table
- import table
- symbol table
- resolved-name table
- unresolved/deferred-name records

Cross-file symbol resolution, autoloading, and runtime fallback loading are not
part of Phase 3.
