# PHPT Source Lookup

The PHPT source symbol index is a navigation aid for php-src behavior
research. It records common C/Zend entry points such as `PHP_FUNCTION`,
`ZEND_FUNCTION`, `PHP_METHOD`, `ZEND_METHOD`, class entries, module entries, and
Zend source files.

Use it to find relevant source files and line numbers:

```bash
nix develop -c just phpt-source-lookup SYMBOL=strlen
nix develop -c just phpt-source-lookup SYMBOL=ArrayObject::offsetGet
```

The index is not permission to transliterate php-src C functions into Rust.
Behavior should be understood from Original PHPT cases, Reference PHP output,
and concise source notes under `docs/phpt/php-src-behavior/`.

Regenerate the index together with the source hash manifest:

```bash
nix develop -c just phpt-source-index
```
