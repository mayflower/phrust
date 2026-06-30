# standard.variables

- Priority: 15
- Selected manifest: `tests/phpt/manifests/modules/standard.variables.selected.jsonl`
- Corpus baseline: 23 PASS, 74 SKIP, 348 FAIL, 0 BORK from 446 corpus candidates
- focused gate: 32 PASS, 1 SKIP, 0 FAIL, 0 BORK

## Scope

- Variable inspection and conversion builtins covered by the selected focused
  gate
- Debug output formatting for scalar, string, and array values

## Non-Scope

- General VM symbol-table redesign
- Complete object/reference rendering matrix

## Relevant PHPT Paths

- `tests/phpt/generated/standard.variables/`
- Selected upstream `ext/standard/tests/array/` and
  `ext/standard/tests/general_functions/` cases in the manifest

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/core.rs`
- `crates/php_runtime/src/value.rs`
- `crates/php_runtime/src/object/`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.variables`
- `nix develop -c just verify-stdlib`

## Evidence

- Kept the selected variable-inspection slice green after the standard-core
  follow-up changes.
- Latest focused target run: PASS, 33 selected PHPTs with 32 PASS / 1 SKIP
  and no non-green target outcomes.

## Evidence

- Reused the selected variable-inspection and debug-output surface as the
  the selected gate variables acceptance gate.
- Latest focused target run: PASS, 33 selected PHPTs with 32 PASS / 1 SKIP
  and no non-green target outcomes.
- Latest oracle-backed stdlib aggregate run: PASS.

## Known Gaps

- `get_debug_type_basic.phpt` remains outside this selected gate because
  anonymous class execution is still a known frontend/runtime gap.
- `gettype_settype_basic.phpt` remains outside this selected gate because
  `settype` is not registered.
- Full `var_dump`/`print_r` object visibility, magic behavior, and reference
  formatting remain outside this selected gate.
