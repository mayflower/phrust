# Semantic Reference Oracle

The semantic reference oracle compares Rust frontend acceptance with the pinned
PHP 8.5.7 CLI where possible.

## Reference Command

The primary acceptance oracle remains PHP lint:

```bash
php -l file.php
```

The harness may also collect structured stderr/stdout metadata for diagnostics,
but it must not execute PHP programs.

The initial oracle entrypoint is:

```bash
scripts/reference_php_frontend_json.py --file fixtures/semantic/valid/minimal.php
```

The canonical smoke command is:

```bash
nix develop -c just semantic-reference-smoke
```

The normalized JSON contains stable harness fields:

```json
{
  "classification": "accepted",
  "exit_code": 0,
  "file": "fixtures/semantic/valid/minimal.php",
  "mode": "lint_compile_frontend",
  "ok": true,
  "oracle": "php-lint",
  "php_version": "8.5.7",
  "stderr": "",
  "stdout": "No syntax errors detected in ..."
}
```

`classification` is `accepted`, `rejected`, or `timeout`. Reference lint
subprocesses are bounded by `REFERENCE_PHP_TIMEOUT_SECONDS` and default to 30
seconds so fixture comparison cannot hang indefinitely.

## Skip Policy

If no PHP reference binary is available, reference-dependent checks must skip
with a clear message. If `REFERENCE_PHP` is explicitly set, failures to run the
reference are hard failures.

## Diff Status

Semantic diffing must distinguish:

- accepted by both
- rejected by both
- Rust accepts and reference rejects
- Rust rejects and reference accepts
- reference unavailable
- explicit known gap
- skipped

Known gaps must be explicit entries and must not hide unrelated mismatches.

Current harness commands:

```bash
nix develop -c just semantic-reference-smoke
nix develop -c just semantic-fixtures
nix develop -c just semantic-diff
```
