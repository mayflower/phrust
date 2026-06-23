# Phase 4 Runtime Reference Oracle

Phase 4 runtime comparisons use the pinned PHP `8.5.7` CLI through the
`REFERENCE_PHP` environment variable. If `REFERENCE_PHP` is missing, the
runtime reference oracle reports `skipped`. If it is explicitly set but not
usable, the oracle reports an error so CI cannot silently compare against an
invalid reference.

The fixture diff pipeline that compares this reference side with the Rust VM is
documented in `docs/phase4-reference-diff.md`.

## Controlled Environment

The PHP reference runner clears the inherited environment and sets only:

| Variable | Value |
| --- | --- |
| `LC_ALL` | `C` |
| `LANG` | `C` |
| `NO_COLOR` | `1` |
| `PHP_INI_SCAN_DIR` | empty string |

This keeps runtime stderr and locale-sensitive output deterministic.

Rust VM fixture execution follows the same boundary in its own
`RuntimeContext`: host environment variables are not imported implicitly.
`php-vm run <file> [-- arg ...]` seeds only controlled CLI `$argc`, `$argv`,
`$_SERVER['argc']`, and `$_SERVER['argv']`; request-style superglobals are
empty arrays until a future request/SAPI layer exists.

## Stderr Normalization

Runtime stderr normalization is implemented in
`php_testkit::normalize_output::normalize_runtime_stderr`.

Rules:

| Rule | Replacement |
| --- | --- |
| CRLF or CR line endings | LF |
| Fixture path | `{file}` |
| PHP binary path | `{php}` |
| `on line <number>` | `on line <line>` |
| Decimal duration-looking values | `<duration>` |

The runtime comparison JSON shape is defined by
`php_testkit::runtime_fixture::RuntimeComparisonResult` and includes
`reference`, `rust`, `status`, `diagnostic_ids`, and `known_gap_id`.

## Runtime Fixture Diff

`just runtime-diff` builds `php_vm_cli` and the `compare-runtime` testkit
command, then writes reports under `target/phase4/runtime-diff`. The runner
discovers `fixtures/runtime/**/*.php`, applies optional `phase4-runtime`
metadata comments, runs the Rust VM for every fixture, and runs the PHP
reference side when `REFERENCE_PHP` is configured.

Each fixture JSON records the fixture path, reference side, Rust side,
comparison status, diagnostic IDs, known-gap ID, and a concise mismatch
message. The aggregate `runtime-report.json` summarizes `pass`, `fail`,
`skipped`, and `known_gap` counts. Files under `target/phase4/` are generated
reports and must not be committed.

## VM Trace and Debug Dumps

VM instruction tracing is off by default. Enable it through `VmOptions::trace`
or the CLI flag:

```bash
nix develop -c cargo run -p php_vm_cli -- run --trace fixtures/runtime/valid/variables/assignment.php
```

The trace is written to stderr after runtime diagnostics so normal stdout stays
byte-for-byte identical to an untraced run. Each line is deterministic and
contains the trace step, function name and `FunctionId`, block id, instruction
id and kind, stack depth, output length, initialized locals, and initialized
registers. Trace lines deliberately avoid memory addresses.

`php-vm dump-ir <file> --with-source` prefixes the deterministic IR snapshot
with numbered source lines for fixture debugging. `just vm-trace-smoke` writes
trace and IR debug files under `target/phase4/failures/`; that directory is a
generated artifact location and must not be committed.

## PHPT Smoke Runner

`just phpt-smoke` runs the selected local `.phpt` fixtures in
`fixtures/phpt_smoke` through the Rust VM. It does not vendor PHP's upstream
suite and does not attempt to replace `run-tests.php`. Developers may pass an
extra local test file or directory with `run-phpt-smoke --extra-phpt <path>`;
that path is optional and is not part of CI.

Supported sections are `--TEST--`, `--FILE--`, `--EXPECT--`, `--EXPECTF--`,
`--SKIPIF--`, and `--INI--`. `--SKIPIF--` fixtures are reported as skipped and
the skip PHP is not executed. `--INI--` fixtures are reported as known gaps
because Phase 4 has no per-test INI model. Other PHPT sections are skipped
instead of interpreted.

The `EXPECTF` subset is deliberately small and tested in `php_testkit::phpt`:
`%%`, `%s`, `%S`, `%d`, `%i`, and `%w`. Generated PHP files and the aggregate
`phpt-smoke-report.json` are written under `target/phase4/phpt-smoke`.

Phase 6 adds `just phase6-phpt-smoke` for selected upstream extension PHPTs.
That gate reads `fixtures/phase6/phpt_extension_manifest.toml`, references
`third_party/php-src` paths without vendoring the PHPT suite, reports missing
upstream files as explicit skips, and writes a path-normalized report under
`target/phase6/phpt-extension-smoke`.
