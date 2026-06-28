# arrays.references Current Focus Report

Core values/arrays/strings branch focused array/reference/COW
verification.

## Scope

- Generated PHPTs under `tests/phpt/generated/arrays.references/`.
- Ordered array key normalization, append next-key tracking, and unset holes.
- `isset()` and `empty()` over local array dimensions.
- Array element references and append-by-reference.
- Copy-on-write separation for scalar and nested array writes.
- `foreach` by-value snapshots and by-reference iteration over local arrays.

## Selected Manifest

- `tests/phpt/manifests/modules/arrays.references.selected.jsonl`
- 6 generated core selected fixtures with expected output captured from
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

## Selected Fixtures

- `tests/phpt/generated/arrays.references/core-key-normalization-append-unset.phpt`
- `tests/phpt/generated/arrays.references/core-isset-empty-dimensions.phpt`
- `tests/phpt/generated/arrays.references/core-cow-separation-on-write.phpt`
- `tests/phpt/generated/arrays.references/core-foreach-by-value-snapshot.phpt`
- `tests/phpt/generated/arrays.references/core-foreach-by-reference-local.phpt`
- `tests/phpt/generated/arrays.references/core-array-element-references.phpt`

## Before/After

Before this this pass, the broad selected audit had Reference PHP green
for 200 fixtures while the target had 18 PASS, 58 SKIP, and 124 FAIL. That set
mixed the selected gate local-array semantics with by-reference method returns, object
property references, SPL and extension iterables, ReflectionReference, property
hooks, and extension availability.

| Check | Before | After |
| --- | ---: | ---: |
| `arrays.references` selected PHPTs | 18 PASS / 58 SKIP / 124 FAIL | 6 PASS |

## Supported Cases

- Ordered array storage preserves insertion order while overwriting existing
  keys in place.
- Numeric-string keys normalize to integer keys where PHP does, while
  non-canonical numeric strings remain string keys.
- Appends keep the next integer key after explicit integer keys and unset
  holes.
- `isset()` and `empty()` handle present, null, missing, and nested dimensions.
- Array assignment separates on write for scalar and nested writes.
- Array element references write through cells, and unsetting the array
  dimension does not destroy the alias cell.
- By-value `foreach` uses a snapshot when the source array mutates.
- By-reference `foreach` over local arrays mutates elements, sees appended
  entries, and preserves the lingering loop reference.

## Remaining Explicit Gaps

- The old broad audit remains useful as discovery data but is not the selected
  gate. It still includes out-of-scope object, SPL, reflection, property-hook,
  and extension cases.
- Null array offsets do not yet emit PHP 8.5's deprecation notice in the
  selected runtime behavior.
- By-reference foreach over nonlocal expressions remains a known lowering gap.

## Verification

Latest branch verification:

- `nix develop -c cargo test -p php_runtime array`: PASS, 47 tests.
- `nix develop -c cargo test -p php_runtime reference`: PASS, 21 tests.
- `nix develop -c cargo test -p php_vm`: PASS, 352 tests.
- `nix develop -c just phpt-dev-build`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=arrays.references`: PASS, reference 6 PASS and target 6 PASS.
- `nix develop -c just verify-runtime`: PASS.
