# PHPT Binary Discovery

PHPT commands use these environment variables:

- `REFERENCE_PHP`: Reference PHP CLI binary. Defaults to
  `$PHP_SRC_DIR/sapi/cli/php`.
- `TARGET_PHP`: Target PHP CLI compatibility binary. Defaults to
  `target/debug/phrust-php`.
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

The target smoke builds `target/debug/phrust-php` when no explicit `TARGET_PHP`
is provided and fails if the selected target does not support the required
PHP CLI metadata and `-r` execution surface.

## Target CLI

`phrust-php` is the PHPT target binary. It is separate from `php-vm`, which
remains the developer CLI for compile, IR, report, and VM debugging commands.

Supported PHPT-facing CLI surface:

- `phrust-php -v` and `phrust-php --version`
- `phrust-php -h` and `phrust-php --help`
- `phrust-php -r '<code>'`
- `phrust-php -n`, `-q`, `-c <path>`, `--repeat <n>`
- repeated `-d key=value`
- `phrust-php -f <file> [--] [args...]`
- `phrust-php <file> [--] [args...]`
- source code from stdin when no file or `-r` is provided
- script stdin through `STDIN`
- `$argc`, `$argv`, `$_SERVER['argc']`, and `$_SERVER['argv']`

Mapped INI values currently include:

- `include_path`
- `display_errors`
- `error_reporting`

Unknown `-d` values are accepted as deterministic no-ops for PHPT runner noise;
`opcache.*` settings are ignored explicitly. `phrust-php` maps compile,
runtime, unsupported, and usage failures to PHP-style status `255`.

Current boundaries:

- no php.ini file parser for `-c`
- no extension loading
- no CGI, POST, multipart, or web-SAPI emulation
- no Zend ABI or Opcache behavior
