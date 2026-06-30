# spl

- Priority: 20
- Selected manifest: `tests/phpt/manifests/modules/spl.selected.jsonl`
- generated counts: 8 PASS, 0 SKIP, 0 FAIL, 0 BORK from 8 selected fixtures
- Aggregate selected counts after the latest autoload promotion: 76 PASS, 1 SKIP, 131 FAIL, 0 BORK from 208 selected fixtures
- Full upstream corpus baseline: 39 PASS, 3 SKIP, 478 FAIL, 0 BORK from 520 corpus candidates

## Scope

- generated SPL MVP submodule fixtures
- Submodules:
  - `spl.interfaces`
  - `spl.array-iterator`
  - `spl.array-object`
  - `spl.fixed-array`
  - `spl.object-storage`
  - `spl.doubly-linked-list`
  - `spl.file`
  - `spl.autoload`

## Non-Scope

- full SPL API parity
- broad upstream `ext/spl` corpus parity
- heaps, priority queues, directory iterators, caching iterators, recursive iterator iterator, observer subject APIs, and serialization parity

## Selected PHPT Paths

- `tests/phpt/generated/spl.interfaces/interface-method-surface.phpt`
- `tests/phpt/generated/spl.array-iterator/iterator-mvps.phpt`
- `tests/phpt/generated/spl.array-iterator/iterator-helpers.phpt`
- `ext/spl/tests/iterator_to_array_array.phpt`
- `ext/spl/tests/iterator_count_array.phpt`
- `ext/spl/tests/spl_006.phpt`
- `ext/spl/tests/gh19577.phpt`
- `tests/phpt/generated/spl.array-object/array-object-mvp.phpt`
- `ext/spl/tests/spl_001.phpt`
- `tests/phpt/generated/spl.fixed-array/fixed-array-mvp.phpt`
- `ext/spl/tests/splfixedarray_json_encode.phpt`
- `tests/phpt/generated/spl.object-storage/object-storage-mvp.phpt`
- `ext/spl/tests/SplObjectStorage/SplObjectStorage_offsetGet.phpt`
- `tests/phpt/generated/spl.doubly-linked-list/linear-containers-mvp.phpt`
- `ext/spl/tests/SplDoublyLinkedList_current.phpt`
- `ext/spl/tests/SplDoublyLinkedList_key.phpt`
- `ext/spl/tests/SplDoublyLinkedList_isEmpty_empty.phpt`
- `ext/spl/tests/SplDoublyLinkedList_isEmpty_not-empty.phpt`
- `ext/spl/tests/SplDoublyLinkedList_offsetExists_success.phpt`
- `tests/phpt/generated/spl.file/file-classes-mvp.phpt`
- `ext/spl/tests/spl_fileinfo_getextension_leadingdot.phpt`
- `tests/phpt/generated/spl.autoload/autoload-mvp.phpt`
- `ext/spl/tests/spl_autoload_003.phpt`
- `ext/spl/tests/spl_autoload_010.phpt`
- `ext/spl/tests/spl_autoload_013.phpt`
- `ext/spl/tests/spl_autoload_bug48541.phpt`

## Relevant php-src Source Areas

- `ext/spl/php_spl.c`
- `ext/spl/spl_array.c`
- `ext/spl/spl_directory.c`
- `ext/spl/spl_dllist.c`
- `ext/spl/spl_fixedarray.c`
- `ext/spl/spl_observer.c`

## Target Gates

- SPL submodules: `nix develop -c just phpt-dev-module MODULE=spl.<submodule>`
- Aggregate selected: `nix develop -c just phpt-dev-module MODULE=spl`
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just verify-phpt`
- `nix develop -c just verify-stdlib`

The SPL submodule gates are green. The aggregate `spl` gate remains red
because the pre-existing upstream selected SPL batch still has 131 target
non-green outcomes; the latest autoload promotion did not add BORKs or new
fixture failures.

## Subarea Failure Snapshot

| Subarea | Before FAIL/BORK | Selected After FAIL/BORK | Remaining gaps |
| --- | ---: | ---: | --- |
| `spl.interfaces` | unknown from full corpus split | 0/0 | full interface inheritance and exact reflection metadata beyond selected methods |
| `spl.array-iterator` | unknown from full corpus split | 0/0 | flags, serialization, mutation by reference, deep recursive iterator APIs |
| `spl.array-object` | unknown from full corpus split | 0/0 | flags, object property mode, serialization, nested by-reference writes |
| `spl.fixed-array` | unknown from full corpus split | 0/0 | exact exception text and serialization |
| `spl.object-storage` | unknown from full corpus split | 0/0 | info edge cases, serialization, lvalue object-key bracket semantics |
| `spl.doubly-linked-list` | unknown from full corpus split | 0/0 | iterator mode matrix, serialization, exhaustive exception parity |
| `spl.file` | unknown from full corpus split | 0/0 | write-through file object semantics, locking, ownership/mode changes, CSV flag matrix, full seek modes |
| `spl.autoload` | unknown from full corpus split | 0/0 | throw exactness, destructor ordering, and default `spl_autoload` namespace/path conventions |
| aggregate legacy selected batch | 196/0 | 131/0 | heaps, caching iterators, serialization, remaining advanced autoload, catchable constructor `ValueError`s, FPM/daemon-style tests, full file APIs |

## Known Gaps

- `runtime-error-or-diagnostic`: 361 upstream SPL corpus candidates
- `runtime-unsupported-feature`: 71 upstream SPL corpus candidates
- `runtime-output-mismatch`: 60 upstream SPL corpus candidates
- `frontend-parse-or-compile`: 1 upstream SPL corpus candidate
- `STDLIB-GAP-SPL-INTERFACE-METHOD-SURFACES`
- `STDLIB-GAP-SPL-AUTOLOAD-ADVANCED`
- `STDLIB-GAP-SPL-OBJECT-HASH-PARITY`
- `STDLIB-GAP-SPL-ITERATOR-MUTATION-EDGES`
- `STDLIB-GAP-SPL-ITERATOR-FULL-API`
- `STDLIB-GAP-SPL-CONTAINER-FULL-API`
- `STDLIB-GAP-SPL-CONTAINER-NESTED-ARRAYACCESS`
- `STDLIB-GAP-SPL-FILE-FULL-API`
- `STDLIB-GAP-SPL-FILE-CSV-FLAGS`

## Implemented Surface

- `SplFileInfo::getExtension()` now covers leading-dot basenames such as `.test`.
- `json_encode(new SplFixedArray(...))` now emits array-shaped JSON instead of internal storage properties.
- Userland classes can implement internal SPL interfaces such as `Countable`,
  and `count($object)` dispatches to their `count()` method.
- `iterator_count()` and `iterator_to_array()` now cover array inputs and the
  existing Traversable/ArrayIterator MVP path.
- `SplObjectStorage` bracket assignment now accepts object keys for direct
  ArrayAccess attachment.
- `SplDoublyLinkedList` now covers selected upstream `isEmpty()`, empty
  `key()`, `current()`, and `offsetExists()` behavior.
- `eval()` now preserves concatenated code operands through HIR/IR lowering, so
  autoload callbacks can dynamically declare the requested class in
  `spl_autoload_bug48541.phpt`.
- Conditional class declarations inside function bodies are registered only
  when their declaration statement executes, and `spl_autoload_register()`
  honors the `prepend` flag used by `spl_autoload_010.phpt`.
- Closure debug metadata now includes parameter state, and
  `spl_autoload_functions()` exposes invokable object callbacks in the shape
  expected by `spl_autoload_013.phpt`.

## Next Step

Expand the selected manifests with upstream `ext/spl` tests one subarea at a time after each documented gap is closed.
