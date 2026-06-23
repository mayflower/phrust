# Phase 6 Composer Compatibility

Phase 6 targets offline Composer compatibility for PHP 8.5.7 (`php-8.5.7`).
The required workflow is:

```bash
nix develop -c just composer-smoke
```

## Required Path

- local PSR-4 fixtures: `tests/fixtures/phase6/composer/project`
- generated classmap fixtures
- generated files autoload fixtures
- Composer-style `platform_check.php` fixtures
- source-mode smoke without network, plugins, scripts, or Packagist

## Prompt 06.44 Coverage

Prompt 06.44 wires the CLI include roots into runtime filesystem
capabilities, so `include_path`, `stream_resolve_include_path`,
`include`/`require`, and SPL autoload callbacks see the same bounded local
fixture roots.

The checked-in Composer-style project uses `vendor/autoload.php` to register a
PSR-4-like autoload function. The fixture loads
`Phase6\ComposerProject\App\Greeter` from
`tests/fixtures/phase6/composer/project/src`, verifies that a missing class
does not crash, constructs the autoloaded class, and calls an instance method
defined in the included unit.

Validation:

```bash
nix develop -c scripts/phase6_diff.py --file tests/fixtures/phase6/_harness/composer/composer_project_autoload.php --out target/phase6/diff-composer-project-autoload
nix develop -c just composer-smoke
```

## Prompt 06.45 Generated Fixture

`tests/fixtures/phase6/composer/basic_project` is a checked-in, offline
Composer-like project. It includes:

- `vendor/autoload.php`
- `vendor/composer/autoload_psr4.php`
- `vendor/composer/autoload_classmap.php`
- `vendor/composer/autoload_files.php`
- one PSR-4 class, one classmap class, and one files-autoload helper function

The fixture can be regenerated deterministically without network access:

```bash
nix develop -c just composer-fixture-prepare
```

The differential fixture
`tests/fixtures/phase6/_harness/composer/basic_project_autoload.php` verifies
that `require 'autoload.php'` loads the files helper, autoloads the PSR-4
class, autoloads the classmap class, and handles a missing class without a
runtime crash.

## Prompt 06.46 Autoload Smoke

`just composer-smoke-autoload` runs the dedicated autoload-order fixture through
the Phase 6 differential harness:

```bash
nix develop -c just composer-smoke-autoload
```

The smoke writes reference/Rust snapshot details under
`target/phase6/composer-smoke-autoload`. It verifies files autoload happens
before class method use, PSR-4 and classmap classes load, repeated
`include_once 'autoload.php'` is stable, the autoload stack is not duplicated by
the repeated include, and missing classes remain non-fatal.

## Prompt 06.47 Platform Checks

`just composer-smoke-platform` runs Composer-style platform checks through the
Phase 6 differential harness:

```bash
nix develop -c just composer-smoke-platform
```

The fixture `vendor/composer/platform_check.php` in the offline
`basic_project` verifies `PHP_VERSION_ID`, `PHP_VERSION`, `defined`,
`constant`, `extension_loaded`, `get_loaded_extensions`, `ini_get`,
`class_exists`, `function_exists`, and `version_compare`. It also confirms that
an absent extension such as `mbstring` is reported as not loaded. The companion
`platform_version_compare.php` fixture pins Composer-relevant comparison
operators and prerelease labels against the PHP 8.5.7 reference.

## Prompt 06.48 Process Capability Surface

`just process-capability-smoke` runs a VM-only fixture that proves Composer-facing
process probes are defined but default-off:

```bash
nix develop -c just process-capability-smoke
```

The fixture checks `proc_open`, `popen`, `shell_exec`, `exec`, `passthru`, and
`system` return deterministic failure values and emit controlled
`E_PHP_VM_PROCESS_CAPABILITY_DISABLED` diagnostics instead of launching a shell
or crashing. Differential comparison is intentionally not used for this fixture
because reference PHP would execute the host command.

## Prompt 06.49 Source-Mode Smoke

`just composer-smoke-source` runs an opt-in Composer source checkout smoke:

```bash
nix develop -c just composer-smoke-source
PHASE6_COMPOSER_SOURCE_DIR=/path/to/composer nix develop -c just composer-smoke-source
```

When `PHASE6_COMPOSER_SOURCE_DIR` is unset or missing, the target writes
`target/phase6/composer-source-smoke/report.json` with `status=skip` and exits
successfully. When it is set, the script rejects `composer.phar`, builds the
local `php-vm` binary, and runs the configured source entry
(`$PHASE6_COMPOSER_SOURCE_ENTRY` or `bin/composer`) with `--no-plugins
--version`. `COMPOSER_HOME` and `COMPOSER_CACHE_DIR` point at
`target/phase6/composer-source-smoke`, and no Packagist/network setup is
performed.

Failures write `stdout.txt`, `stderr.txt`, `report.json`, and
`missing-symbols.txt`. The missing-symbol list is sorted by frequency and
currently extracts undefined functions and classes from VM diagnostics so the
next compatibility gap is visible without requiring Composer PHAR support.

## Explicit Boundaries

- Composer source mode is required before `composer.phar`.
- PHAR is optional and governed by ADR-0066. Phase 6 does not implement PHAR
  archive parsing, `phar://`, or stub execution; an optional read-only MVP must
  be accepted separately before any implementation.
- Online Packagist is not a required Phase 6 gate.
- Composer source mode is opt-in through `PHASE6_COMPOSER_SOURCE_DIR`; the
  repository does not vendor Composer source.
- Process and shell functions are disabled by default. Phase 6 includes an
  isolated mock for `shell_exec`, `exec`, `system`, and `passthru`; real process
  execution and process resources remain tracked in
  `PHASE6-GAP-PROCESS-CAPABILITY`.
- Host filesystem access is restricted to deterministic fixture and temporary
  directories.

## Reference

Reference comparison uses pinned PHP 8.5.7 via `REFERENCE_PHP` or
`third_party/php-src/sapi/cli/php`; Phase 6 commands must not silently select a
global system PHP.
