# standard.arrays

- Priority: 12
- Selected manifest: `tests/phpt/manifests/modules/standard.arrays.selected.jsonl`
- Prompt 16.1 baseline: 218 PASS, 7 SKIP, 595 FAIL, 0 BORK from 821 corpus candidates
- Prompt 2D focused gate: 17 PASS, 0 SKIP, 0 FAIL, 0 BORK

## Scope

- Core Prompt 2D array builtins with stable scalar/packed-array behavior
- Focused generated fixtures for `count`, `array_keys`, `array_values`,
  `array_merge`, `array_slice`, `array_key_exists`, `array_splice`,
  `array_column`, `in_array`, `array_search`, `array_unique`, `range`, and
  deterministic `sort`/`asort`/`ksort`

## Non-Scope

- Full upstream array corpus
- Callback-heavy helpers without VM callable dispatch
- Broad Copy-on-Write/reference behavior outside the selected fixtures

## Relevant PHPT Paths

- `tests/phpt/generated/standard.arrays/count-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-keys-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-values-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-merge-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-slice-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-key-exists-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-splice-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-column-smoke.phpt`
- `tests/phpt/generated/standard.arrays/in-array-search-smoke.phpt`
- `tests/phpt/generated/standard.arrays/array-unique-smoke.phpt`
- `tests/phpt/generated/standard.arrays/range-smoke.phpt`
- `tests/phpt/generated/standard.arrays/sort-deterministic-smoke.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/arrays.rs`
- `crates/php_runtime/src/array.rs`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.arrays`
- `nix develop -c just verify-stdlib`

## Prompt 16 Evidence

- Added focused generated array fixtures and selected-manifest coverage.
- Added VM array-cast behavior for arrays, null/uninitialized values, objects,
  and scalar/resource values.
- Latest focused target run: PASS, 10 selected PHPTs.

## Prompt 2D Evidence

- Added generated fixtures for the remaining Prompt 2D builtin list:
  `array_key_exists`, `array_splice`, `array_column`, `in_array`,
  `array_search`, `array_unique`, `range`, and deterministic sort coverage.
- Latest focused target run: PASS, 17 selected PHPTs.
- Latest oracle-backed stdlib verification: PASS, `verify-stdlib` with
  `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

## Known Gaps

- Full upstream array corpus remains larger than the focused Prompt 16 gate.
- Callback-heavy, object, and reference-sensitive array cases need later slices
  before they can be treated as complete.
