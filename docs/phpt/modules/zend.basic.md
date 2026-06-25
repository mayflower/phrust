# zend.basic

- Priority: 4
- Selected manifest: `tests/phpt/manifests/modules/zend.basic.selected.jsonl`
- Current counts: 274 PASS, 1 SKIP, 3226 FAIL, 0 BORK from 3509 corpus candidates
- Selected gate status: 6 PASS, 0 SKIP, 0 FAIL, 0 BORK for reference and target

## Scope

- top-level execution
- scalar literals
- numeric literal separators
- echo
- print
- statement sequencing
- top-level return
- top-level exit
- basic var_dump output

## Non-Scope

- dynamic variables
- objects
- extensions
- advanced type system
- exact string-to-float formatting edge cases

## Relevant PHPT Paths

- `Zend/tests/numeric_literal_separator/numeric_literal_separator_001.phpt`
- `tests/phpt/generated/zend.basic/regression-numeric_literal_separator_001-e7854d367777.phpt`
- `tests/phpt/generated/zend.basic/smoke-var-dump-scalars-e7854d367777.phpt`
- `tests/phpt/generated/zend.basic/smoke-echo-print-sequencing-5aec11b01afc.phpt`
- `tests/phpt/generated/zend.basic/smoke-top-level-return-f9f31fdf1b6e.phpt`
- `tests/phpt/generated/zend.basic/smoke-top-level-exit-ff648fb6d646.phpt`

## Relevant php-src Source Areas

- `Zend/tests/`
- `crates/php_vm/`
- `crates/php_runtime/`

## Target Gates

- `nix develop -c just phpt-module MODULE=zend.basic`

## Known Gaps

- `runtime-error-or-diagnostic`: 1386
- `runtime-unsupported-feature`: 1136
- `runtime-output-mismatch`: 653
- `frontend-parse-or-compile`: 43
- `runtime-timeout`: 9

## Next Step

Keep the selected zend.basic gate green while later modules expand runtime semantics.
