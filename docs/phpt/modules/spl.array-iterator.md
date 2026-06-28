# spl.array-iterator

- Priority: 20
- Selected manifest: `tests/phpt/manifests/modules/spl.array-iterator.selected.jsonl`
- Current selected counts: 6 PASS, 0 SKIP, 0 FAIL, 0 BORK

## Scope

- `ArrayIterator`
- `IteratorIterator`
- `RecursiveArrayIterator`
- `LimitIterator`
- `EmptyIterator`
- `AppendIterator`
- `current`, `key`, `next`, `rewind`, `valid`, `count`, `foreach`, simple wrapping, `iterator_count`, and `iterator_to_array`

## Non-Scope

- flags
- serialization
- live mutation edge cases
- recursive child APIs beyond selected tests

## Selected PHPT Paths

- `tests/phpt/generated/spl.array-iterator/iterator-mvps.phpt`
- `tests/phpt/generated/spl.array-iterator/iterator-helpers.phpt`
- `ext/spl/tests/iterator_to_array_array.phpt`
- `ext/spl/tests/iterator_count_array.phpt`
- `ext/spl/tests/spl_006.phpt`
- `ext/spl/tests/gh19577.phpt`

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=spl.array-iterator`
- `nix develop -c just diff-spl-reflection`

## Known Gaps

- `STDLIB-GAP-SPL-ITERATOR-MUTATION-EDGES`
- `STDLIB-GAP-SPL-ITERATOR-FULL-API`

## Coverage

The selected fixtures cover deterministic array-backed iteration, iterator
wrapping, limit slicing, empty iterator invalidity, append composition, basic
recursive array iterator metadata, and the VM Traversable helper path used by
`iterator_count()` and `iterator_to_array()` for arrays, `ArrayIterator`,
`AppendIterator`, and `LimitIterator`.
