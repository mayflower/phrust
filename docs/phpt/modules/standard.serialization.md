# standard.serialization

- Priority: 16
- Selected manifest: `tests/phpt/manifests/modules/standard.serialization.selected.jsonl`
- Corpus baseline: 16 PASS, 2 SKIP, 107 FAIL, 0 BORK from 126 corpus candidates
- focused gate: 23 PASS, 0 FAIL, 0 BORK

## Scope

- `serialize`
- `unserialize`
- Scalar, array, and simple object persistence covered by the selected gate
- Selected upstream scalar, option, and edge-case serialization regressions
  covered by the selected gate
- `wp.core-builtins` reuses scalar and array serialization roundtrips for
  WordPress-style cache payload coverage.

## Non-Scope

- Session module persistence
- PHP `R`/`r` reference identity records
- `allowed_classes` validation diagnostics
- Full magic hook and resource serialization behavior

## Relevant PHPT Paths

- `ext/standard/tests/serialize/002.phpt`
- `ext/standard/tests/serialize/004.phpt`
- `ext/standard/tests/serialize/bug23298.phpt`
- `ext/standard/tests/serialize/bug24063.phpt`
- `ext/standard/tests/serialize/bug31442.phpt`
- `ext/standard/tests/serialize/bug37947.phpt`
- `ext/standard/tests/serialize/bug42919.phpt`
- `ext/standard/tests/serialize/bug43614.phpt`
- `ext/standard/tests/serialize/bug46882.phpt`
- `ext/standard/tests/serialize/bug55798.phpt`
- `ext/standard/tests/serialize/bug68594.phpt`
- `ext/standard/tests/serialize/bug74300.phpt`
- `ext/standard/tests/serialize/bug81142.phpt`
- `ext/standard/tests/serialize/serialization_precision_001.phpt`
- `ext/standard/tests/serialize/serialize_globals_var_refs.phpt`
- `ext/standard/tests/serialize/shm_corruption_coercion_unserialize_options.phpt`
- `ext/standard/tests/serialize/sleep_deref.phpt`
- `ext/standard/tests/serialize/unserializeS.phpt`
- `ext/standard/tests/serialize/unserialize_allowed_classes_option_stringable_value.phpt`
- `ext/standard/tests/serialize/unserialize_neg_iv_edge_cases.phpt`
- `tests/phpt/generated/standard.serialization/serialize-unserialize-scalars-arrays.phpt`
- `tests/phpt/generated/standard.serialization/serialize-unserialize-simple-object.phpt`
- `tests/phpt/generated/standard.serialization/unserialize-reference-record-gap.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/serialization.rs`
- `crates/php_runtime/src/value.rs`
- `crates/php_runtime/src/object/`
- `docs/stdlib-serialization.md`

## Target Gates

- `nix develop -c cargo test -p php_runtime serialization -- --nocapture`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.serialization`
- `nix develop -c just verify-stdlib`

## Evidence

- Narrowed the selected manifest to scalar/array/simple-object serialization
  plus an explicit reference-record known-gap fixture.
- Documented that `R`/`r` reference identity records are intentionally rejected
  as `STDLIB-GAP-SERIALIZE-REFERENCES`.
- Latest focused target run: PASS, 23 selected PHPTs.

## Evidence

- Reused the selected scalar, array, simple-object, and explicit reference-gap
  serialization surface as the selected serialization acceptance gate.
- Latest focused target run: PASS, 23 selected PHPTs.
- Latest oracle-backed stdlib aggregate run: PASS.

## Known Gaps

- `R`/`r` reference identity records are not emitted or reconstructed.
- `allowed_classes` validation diagnostics, magic hooks, resources, and deep
  object/reference graphs remain outside the selected focused gate.
