# SPL Current PHPT Report

Generated: 2026-06-28

Branch: `phpt/b3-spl-reflection`

## Scope

This report covers the selected SPL module and the SPL submodules touched by the
Branch 3 SPL/Reflection implementation scope:

- `spl.interfaces`
- `spl.array-iterator`
- `spl.array-object`
- `spl.fixed-array`
- `spl.object-storage`
- `spl.doubly-linked-list`
- `spl.file`
- `spl.autoload`

## Selected PHPT Results

| Module | Before branch | After branch | Remaining gaps |
| --- | --- | --- | --- |
| `spl.interfaces` | 1 PASS | 1 PASS | full interface inheritance and exact reflection metadata beyond selected methods |
| `spl.array-iterator` | 1 PASS | 6 PASS | flags, serialization, mutation by reference, deep recursive iterator APIs |
| `spl.array-object` | 1 PASS | 2 PASS | flags, object property mode, serialization, nested by-reference writes |
| `spl.fixed-array` | 1 PASS | 2 PASS | exact exception text and serialization |
| `spl.object-storage` | 1 PASS | 2 PASS | info edge cases, serialization, deeper object-key lvalue semantics |
| `spl.doubly-linked-list` | 1 PASS | 6 PASS | iterator mode matrix, serialization, exhaustive exception parity |
| `spl.file` | 1 PASS | 2 PASS | write-through file object semantics, locking, ownership/mode changes, CSV flag matrix, full seek modes |
| `spl.autoload` | 1 PASS | 2 PASS | prepend/throw exactness and default `spl_autoload` namespace/path conventions |
| `spl` aggregate selected | 17 PASS, 2 SKIP, 189 FAIL | 17 PASS, 2 SKIP, 189 FAIL | broad legacy selected failures remain visible |

The focused SPL submodule gates are green. The aggregate `spl` gate remains red
because the selected upstream SPL batch still includes 189 target non-green
outcomes outside the focused Branch 3 scope. This branch did not accept a new
baseline and did not hide those failures.

## Selected PHPT Paths Added Or Kept Green

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

## Closed During Branch 3

- `iterator_count()` and `iterator_to_array()` cover array inputs and the
  existing Traversable/ArrayIterator path.
- Userland classes can implement internal SPL interfaces such as `Countable`,
  and `count($object)` dispatches to their `count()` method.
- `SplObjectStorage` bracket assignment accepts object keys for direct
  ArrayAccess attachment.
- `SplDoublyLinkedList` covers selected upstream `current()`, empty `key()`,
  `isEmpty()`, and `offsetExists()` behavior.
- `SplFileInfo::getExtension()` covers leading-dot basenames such as `.test`.
- `json_encode(new SplFixedArray(...))` emits array-shaped JSON instead of
  internal storage properties.

## Verification

Passed with the pinned PHP 8.5.7 reference binary and branch-local PHPT target:

- `nix develop -c just phpt-dev-module MODULE=spl.interfaces`
- `nix develop -c just phpt-dev-module MODULE=spl.array-iterator`
- `nix develop -c just phpt-dev-module MODULE=spl.array-object`
- `nix develop -c just phpt-dev-module MODULE=spl.fixed-array`
- `nix develop -c just phpt-dev-module MODULE=spl.object-storage`
- `nix develop -c just phpt-dev-module MODULE=spl.doubly-linked-list`
- `nix develop -c just phpt-dev-module MODULE=spl.file`
- `nix develop -c just phpt-dev-module MODULE=spl.autoload`
- `nix develop -c just diff-spl-reflection`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_runtime object`
- `nix develop -c cargo test -p php_vm`

Completed but not green:

- `nix develop -c just phpt-dev-module MODULE=spl`: target 17 PASS, 2 SKIP,
  189 FAIL; reference 206 PASS, 2 SKIP.

Every PHPT module run also verified the pinned `php-src` source-integrity
manifest: 24,475 entries checked, 0 skipped.

## Next Step

Expand selected SPL manifests one subarea at a time after each owning runtime or
metadata gap is implemented. The aggregate selected failures are concentrated in
serialization parity, recursive/caching/tree iterators, heap/priority queue
classes, broader iterator helper functions, by-reference foreach over objects,
and selected filesystem edges such as symlink support.
