# PHPT Green 6A SPL Aggregate Closure Current

## Branch

- Branch: `phpt-green/spl-aggregate-closure`
- Source prompt: `~/Downloads/phpt-green-6A-spl-aggregate-closure.md`
- Checkout note: this checkout's `third_party/` directory is empty, so the
  initial inventory used the existing read-only oracle at
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src`.

## Fresh Inventory

Run with:

```bash
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 \
nix develop -c just phpt-dev-module MODULE=<module>
```

| Module | Reference | Target | Report |
| --- | ---: | ---: | --- |
| `spl` | 205 PASS, 3 SKIP | 77 PASS, 1 SKIP, 130 FAIL | `/tmp/phrust-6a-spl/module-runs/spl` |
| `spl.array-iterator` | 6 PASS | 6 PASS | `/tmp/phrust-6a-spl-array-iterator/module-runs/spl.array-iterator` |
| `spl.array-object` | 2 PASS | 2 PASS | `/tmp/phrust-6a-spl-array-object/module-runs/spl.array-object` |
| `spl.fixed-array` | 2 PASS | 2 PASS | `/tmp/phrust-6a-spl-fixed-array/module-runs/spl.fixed-array` |
| `spl.object-storage` | 2 PASS | 2 PASS | `/tmp/phrust-6a-spl-object-storage/module-runs/spl.object-storage` |
| `spl.doubly-linked-list` | 6 PASS | 6 PASS | `/tmp/phrust-6a-spl-dll/module-runs/spl.doubly-linked-list` |
| `spl.file` | 2 PASS | 2 PASS | `/tmp/phrust-6a-spl-file/module-runs/spl.file` |

## Top Target Failure Clusters

- Iterator and recursive iterator behavior: most of the aggregate failures,
  including `iterator_001.phpt`, `iterator_003.phpt`,
  `multiple_iterator_001.phpt`, `recursive_tree_iterator_001.phpt`,
  `recursiveiteratoriterator_beginiteration_basic.phpt`, and
  `spl_iterator_caching_count_basic.phpt`.
- Heap and priority queue behavior: corruption, mutation, and dump-shape rows
  such as `heap_004.phpt`, `heap_008.phpt`, `heap_012.phpt`,
  `heap_corruption.phpt`, `heap_next_write_lock.phpt`, `pqueue_002.phpt`,
  `pqueue_004.phpt`, and `spl_pq_top_error_corrupt.phpt`.
- SPL serialization and debug-output parity: `unserialize.phpt`,
  `unserialize_errors.phpt`, `serialize_property_tables.phpt`,
  `gh16588.phpt`, `gh16589.phpt`, and object dump shape mismatches for SPL
  containers.
- File and stream edges: `gh20101.phpt`, `gh20678.phpt`, and
  `gh17516.phpt`.
- By-reference foreach and unsupported VM edges: representative rows include
  `iterator_069.phpt`, `heap_009.phpt`, and
  `spl_iterator_recursive_getiterator_error.phpt`.

## Initial High-Yield Direction

The focused submodule manifests are already green, so the first implementation
target should be rows in the broad aggregate `spl.selected.jsonl` that exercise
low-risk helper behavior rather than full recursive/tree iterator internals.
Good first candidates are `iterator_count.phpt`, `iterator_to_array.phpt`,
constructor arity/validation rows, and small CachingIterator/LimitIterator
diagnostics where behavior can be made deterministic without changing broad
object semantics.

## Implementation Progress

All focused runs below used the read-only PHP oracle at
`/Volumes/CrucialMusic/src/phrust/third_party/php-src` because this checkout's
`third_party/` directory is intentionally empty.

| Cluster | Focused PHPTs | Result |
| --- | --- | --- |
| Iterator helper argument validation and `LimitIterator` bounds | `iterator_count.phpt`, `iterator_to_array.phpt`, `spl_limit_iterator_check_limits.phpt` | Reference 3 green, target 3 green |
| SPL constructor arity and catchable constructor diagnostics | `spl_iterator_iterator_constructor.phpt`, `iterator_056.phpt`, `iterator_062.phpt`, `recursive_tree_iterator_002.phpt` | Reference 4 green, target 4 green |
| `ArrayObject` iterator-class handling and serialize property envelope | `iterator_024.phpt`, `serialize_property_tables.phpt` | Reference green, target green |
| Heap and priority queue iteration keys | `heap_005.phpt`, `heap_006.phpt`, `heap_007.phpt`, `pqueue_003.phpt` | Reference 4 green, target 4 green |
| SPL file string conversion and negative truncate diagnostics | `gh9883.phpt`, `gh9883-extra.phpt`, `gh17463.phpt` | Reference 3 green, target 3 green |
| Regex and caching iterator argument diagnostics | `regexIterator_setMode_error.phpt`, `spl_caching_iterator_constructor_flags.phpt` | Reference 2 green, target 2 green |
| `RecursiveTreeIterator` non-recursive source diagnostic | `recursive_tree_iterator_003.phpt` | Reference green, target green |
| `CachingIterator` full-cache access diagnostics | `spl_iterator_caching_count_error.phpt`, `spl_iterator_caching_getcache_error.phpt` | Reference 2 green, target 2 green |
| `CachingIterator` string modes and string-cast diagnostics | `spl_cachingiterator___toString_basic.phpt`, `iterator_036.phpt`, `iterator_037.phpt` | Target 3 green |
| `CachingIterator` full-cache count progression | `spl_iterator_caching_count_basic.phpt` | Target green |
| `CachingIterator` full-cache cache mutation and key string mode | `iterator_045.phpt`, `iterator_046.phpt` | Target 2 green |
| `EmptyIterator` invalid access diagnostics | `iterator_030.phpt` | Target green |
| `LimitIterator` absolute seek and range diagnostics | `iterator_032.phpt`, `gh18421.phpt` | Target 2 green |
| `NoRewindIterator` no-op rewind behavior | `iterator_012.phpt` | Target green |
| SPL `ArrayAccess` subclass `empty()` dispatch | `gh18018.phpt` | Target green |
| SPL iterator wrapper unknown-method diagnostics | `gh16574.phpt` | Target green |
| Adjacent upstream SPL `gh*` promotion probe | `gh14290.phpt` | Reference and target green; promoted |

The `ArrayObject` probe also included `iterator_025.phpt` and
`iterator_026.phpt`. They remain red because they require deeper
`RecursiveIteratorIterator` lifecycle hooks and `RecursiveCachingIterator`
`hasNext()` / array-to-string warning parity, which are larger recursive
iterator semantics gaps rather than narrow container initialization fixes.

The final adjacent upstream probe covered 24 real upstream rows from
`ext/spl/tests/gh12721.phpt` through `ext/spl/tests/gh18322.phpt` using a
temporary manifest at `/tmp/phrust-6a-spl-adjacent-probe.jsonl`. Reference PHP
reported 22 PASS and 2 SKIP; the target reported 5 PASS and 19 FAIL. Only
`ext/spl/tests/gh14290.phpt` was both reference/target clean and not already
selected, so it was the only promoted row.

## Aggregate SPL Counts

| Stage | Reference | Target | Report |
| --- | ---: | ---: | --- |
| Fresh inventory | 205 PASS, 3 SKIP | 77 PASS, 1 SKIP, 130 FAIL | `/tmp/phrust-6a-spl/module-runs/spl` |
| After iterator helper and `LimitIterator` fixes | 205 PASS, 3 SKIP | 80 PASS, 1 SKIP, 127 FAIL | `/tmp/phrust-6a-spl-after-first-fix/module-runs/spl` |
| After constructor arity/catchability fixes | 205 PASS, 3 SKIP | 84 PASS, 1 SKIP, 123 FAIL | `/tmp/phrust-6a-spl-after-constructors/module-runs/spl` |
| After `ArrayObject` / SPL serialize envelope fixes | 205 PASS, 3 SKIP | 86 PASS, 1 SKIP, 121 FAIL | `/tmp/phrust-6a-spl-after-arrayobject/module-runs/spl` |
| After heap/PQ key and file diagnostics fixes | 205 PASS, 3 SKIP | 93 PASS, 1 SKIP, 114 FAIL | `/tmp/phrust-6a-spl-after-heap-file/module-runs/spl` |
| After regex/caching diagnostic fixes | 205 PASS, 3 SKIP | 95 PASS, 1 SKIP, 112 FAIL | `/tmp/phrust-6a-spl-after-diagnostics/module-runs/spl` |
| After `RecursiveTreeIterator` source diagnostic fix | 205 PASS, 3 SKIP | 96 PASS, 1 SKIP, 111 FAIL | `/tmp/phrust-6a-spl-after-recursive-tree-type/module-runs/spl` |
| After `CachingIterator` full-cache diagnostics | 205 PASS, 3 SKIP | 98 PASS, 1 SKIP, 109 FAIL | `/tmp/phrust-6a-spl-after-caching-cache-errors/module-runs/spl` |
| After `CachingIterator` string-conversion fixes | 205 PASS, 3 SKIP | 102 PASS, 1 SKIP, 105 FAIL | `/tmp/phrust-6a-spl-after-caching-tostring/module-runs/spl` |
| After `CachingIterator` full-cache count fix | 205 PASS, 3 SKIP | 103 PASS, 1 SKIP, 104 FAIL | `/tmp/phrust-6a-spl-after-caching-count-basic/module-runs/spl` |
| After `CachingIterator` full-cache mutation fixes | 205 PASS, 3 SKIP | 104 PASS, 1 SKIP, 103 FAIL | `/tmp/phrust-6a-spl-after-caching-cache-mutation/module-runs/spl` |
| After `EmptyIterator` and `LimitIterator` range fixes | 205 PASS, 3 SKIP | 107 PASS, 1 SKIP, 100 FAIL | `/tmp/phrust-6a-spl-after-limit-empty/module-runs/spl` |
| After `NoRewindIterator` no-op rewind fix | 205 PASS, 3 SKIP | 108 PASS, 1 SKIP, 99 FAIL | `/tmp/phrust-6a-spl-after-norewind/module-runs/spl` |
| After SPL `ArrayAccess` and unknown-method dispatch fixes | 205 PASS, 3 SKIP | 110 PASS, 1 SKIP, 97 FAIL | `/tmp/phrust-6a-spl-after-arrayaccess-unknown/module-runs/spl` |
| After adjacent upstream `gh14290.phpt` promotion | 206 PASS, 3 SKIP | 111 PASS, 1 SKIP, 97 FAIL | `/tmp/phrust-6a-spl-after-promotion/module-runs/spl` |

Net movement for the selected aggregate `spl` module is +34 PASS and -33 FAIL,
with one newly promoted upstream row. No PHPT rows were removed from the
selected manifests, and no new skip rows were introduced.

## Remaining Top Failure Clusters

- Recursive iterator lifecycle callbacks and tree formatting:
  `iterator_021.phpt` through `iterator_026.phpt`,
  `recursiveiteratoriterator_beginiteration_basic.phpt`,
  `recursiveiteratoriterator_enditeration_basic.phpt`,
  `recursiveiteratoriterator_nextelement_basic.phpt`, and
  `recursive_tree_iterator_001.phpt`, `recursive_tree_iterator_002.phpt`,
  and `recursive_tree_iterator_004.phpt` through
  `recursive_tree_iterator_008.phpt`.
- Caching and recursive caching iterator behavior:
  `iterator_044.phpt` and deeper cache lifecycle/counting parity.
- Heap and priority queue corruption, mutation, and dump shape:
  `heap_004.phpt`, `heap_008.phpt`, `heap_009.phpt`, `heap_011.phpt`,
  `heap_012.phpt`, `heap_corruption.phpt`, `heap_next_write_lock.phpt`,
  `spl_pq_top_error_corrupt.phpt`, `pqueue_002.phpt`, and `pqueue_004.phpt`.
- Full SPL serialization/unserialization parity:
  `unserialize.phpt`, `unserialize_errors.phpt`, `gh16588.phpt`,
  `gh16589.phpt`, `gh16479.phpt`, and dump-shape rows for SPL containers.
- Object foreach by-reference and broader object/ArrayAccess VM gaps:
  `iterator_027.phpt`, `iterator_069.phpt`, `heap_009.phpt`,
  `spl_iterator_recursive_getiterator_error.phpt`, `gh18322.phpt`, and
  `gh19094.phpt`.
- File/resource edges outside the narrow SPL string/truncate fixes:
  `gh20101.phpt`, `gh20678.phpt`, and `gh17516.phpt`.

## Final Validation

The final broad `spl` run is intentionally still red because this branch kept
remaining aggregate failures visible instead of hiding, deleting, or converting
them to skips. The final target movement is recorded from:

- Reference summary:
  `/tmp/phrust-6a-spl-after-promotion/module-runs/spl/reference/summary.md`
- Target summary:
  `/tmp/phrust-6a-spl-after-promotion/module-runs/spl/target/summary.md`

Final checks run:

```bash
nix develop -c cargo fmt --check
git diff --check
nix develop -c cargo test -p php_vm
nix develop -c cargo test -p php_runtime
nix develop -c cargo test -p php_std
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 \
nix develop -c just phpt-dev-module MODULE=spl
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
nix develop -c just verify-stdlib
REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php \
PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src \
nix develop -c just verify-phpt
```

All final checks passed except the expected broad `spl` aggregate run, which
exited non-zero with the visible remaining target failures listed above.
`tests/phpt/manifests/modules/spl.selected.jsonl` was updated only to promote
the reference/target-clean `gh14290.phpt` row. `docs/stdlib/known-gaps.md` and
`docs/known_gaps/runtime.jsonl` were not changed because no known-gap evidence
changed.
