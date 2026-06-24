# Official run-tests.php Cross-check

`just phpt-official-smoke-phase9` runs a small PHPT list through both the Rust
PHPT runner and php-src's official `run-tests.php` wrapper.

The Rust runner remains the primary structured runner for Phase 9 because it
emits stable JSONL results, fingerprints, and module-oriented reports. The
official runner is used as a compatibility cross-check for selected lists.

## Command

```bash
nix develop -c just phpt-official-smoke-phase9
```

By default the command uses:

- `PHP_SRC_DIR=third_party/php-src-8.5.7` when present, otherwise
  `third_party/php-src`;
- `REFERENCE_PHP=$PHP_SRC_DIR/sapi/cli/php`;
- `PHPT_MANIFEST=tests/phpt/manifests/phase9-original-smoke.jsonl`;
- `TARGET_PHP=$REFERENCE_PHP` when `TARGET_PHP` is not set.

Set `TARGET_PHP` to a PHP CLI-compatible Rust target once that target supports
the required command-line surface. Until then, the default reference-as-target
mode verifies the wrapper, manifest conversion, artifact isolation, and result
comparison without claiming Rust CLI compatibility.

## Artifact Isolation

All generated files are written below `target/phpt-work/official-smoke/`:

- `official/test-list.txt` for the generated `run-tests.php -r` list;
- `official/all.txt` for the official all-status output;
- `official/failed.txt` for failed tests;
- `official/run-tests.log`, `stdout.log`, and `stderr.log`;
- `rust/results.jsonl` and `rust/summary.md` for the Rust runner output.

The wrapper passes `--temp-source` and `--temp-target` to `run-tests.php` and
checks `git -C $PHP_SRC_DIR status --short` after the run. Any source checkout
mutation fails the command.

## Result Agreement

The smoke list is intentionally restricted to simple Original PHPTs that should
be PASS for the selected target. The command fails if the Rust runner records a
non-green outcome or if `run-tests.php` writes any failed test list.
