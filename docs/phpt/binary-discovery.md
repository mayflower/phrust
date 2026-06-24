# PHPT Binary Discovery

PHPT commands use these environment variables:

- `REFERENCE_PHP`: Reference PHP CLI binary. Defaults to
  `$PHP_SRC_DIR/sapi/cli/php`.
- `TARGET_PHP`: Target PHP CLI compatibility binary. Defaults to
  `target/debug/phrust-php` when present, with `target/debug/php-vm` only used
  to report missing compatibility during early PHPT.
- `PHP_SRC_DIR`: pinned php-src checkout. Defaults to
  `third_party/php-src-8.5.7` when present, otherwise `third_party/php-src`.
- `PHPT_WORK_DIR`: generated PHPT work area. Defaults to `target/phpt-work`.

Reference smoke:

```bash
nix develop -c just phpt-reference-smoke
```

Target smoke:

```bash
nix develop -c just phpt-target-smoke
```

The target smoke may report `PHPT_TARGET_CLI_COMPAT` until the PHPT CLI
SAPI work provides PHP-compatible `-v` and `-r` behavior. This is an explicit
TODO diagnostic, not a silent pass.
