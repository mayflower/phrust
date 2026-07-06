# arrays.references

- Priority: 8
- Selected manifest: `tests/phpt/manifests/modules/arrays.references.selected.jsonl`
- Current selected counts: 6 PASS, 0 SKIP, 0 FAIL, 0 BORK from 6 generated core fixtures

## Scope

- ordered arrays
- key normalization
- append
- unset holes
- isset/empty array dimensions
- references
- copy-on-write
- foreach by-value snapshots
- foreach by-reference over local arrays

## Non-Scope

- SPL collection classes
- object property references
- by-reference method returns
- ReflectionReference
- extension-specific iterable objects

## Relevant PHPT Paths

- `tests/phpt/generated/arrays.references/core-key-normalization-append-unset.phpt`
- `tests/phpt/generated/arrays.references/core-isset-empty-dimensions.phpt`
- `tests/phpt/generated/arrays.references/core-cow-separation-on-write.phpt`
- `tests/phpt/generated/arrays.references/core-foreach-by-value-snapshot.phpt`
- `tests/phpt/generated/arrays.references/core-foreach-by-reference-local.phpt`
- `tests/phpt/generated/arrays.references/core-array-element-references.phpt`

## Relevant php-src Source Areas

- `crates/php_runtime/src/array.rs`
- `crates/php_runtime/src/reference.rs`
- `crates/php_ir/src/lower/mod.rs`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime array`
- `nix develop -c cargo test -p php_runtime reference`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=arrays.references`
- `nix develop -c just verify-runtime`

## Known Gaps

- The previous broad 200-fixture audit still has 124 target failures and 58 target skips against a green Reference PHP run. Those failures cluster around by-reference method returns, object property references, SPL and extension iterables, ReflectionReference, property hooks, and extension availability.
- Null used as an array offset currently normalizes to the empty-string key in unit coverage, but PHP 8.5 also emits a deprecation notice. That diagnostic is not part of the selected selected PHPT gate.
- By-reference foreach remains intentionally limited to simple local array variables; nonlocal sources are a stable known gap.

## Next Step

Keep the selected core selected gate green while promoting broader php-src reference, object, SPL, and extension cases into later focused modules.
