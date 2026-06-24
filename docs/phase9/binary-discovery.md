# Phase 9 Binary Discovery

Phase 9 commands use these environment variables:

- `REFERENCE_PHP`: Reference PHP CLI binary. Defaults to
  `$PHP_SRC_DIR/sapi/cli/php`.
- `TARGET_PHP`: Target PHP CLI compatibility binary. Defaults to
  `target/debug/phrust-php` when present, with `target/debug/php-vm` only used
  to report missing compatibility during early Phase 9.
- `PHP_SRC_DIR`: pinned php-src checkout. Defaults to
  `third_party/php-src-8.5.7` when present, otherwise `third_party/php-src`.
- `PHPT_WORK_DIR`: generated PHPT work area. Defaults to `target/phpt-work`.

Reference smoke:

```bash
nix develop -c just phpt-reference-smoke-phase9
```

Target smoke:

```bash
nix develop -c just phpt-target-smoke-phase9
```

The target smoke may report `PHASE9_TARGET_CLI_COMPAT` until the Phase 9 CLI
SAPI work provides PHP-compatible `-v` and `-r` behavior. This is an explicit
TODO diagnostic, not a silent pass.
