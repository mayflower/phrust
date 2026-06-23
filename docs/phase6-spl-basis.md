# Phase 6 SPL Basis

Reference target: PHP 8.5.7 (`php-8.5.7`).

Prompt 06.36 enables the SPL extension by default for the Phase 6 runtime
registry and exposes the Composer-facing SPL basis:

- `spl_autoload_register`
- `spl_autoload_unregister`
- `spl_autoload_functions`
- `spl_autoload_call`
- `spl_object_id`
- `spl_object_hash`
- `Traversable`, `Iterator`, `IteratorAggregate`, `ArrayAccess`, `Countable`,
  and `Serializable`
- SPL `LogicException` and `RuntimeException` hierarchy classes

Autoload stack state remains owned by the VM execution state. Runtime builtin
registry entries exist for symbol discovery, but direct non-VM calls to the
autoload functions return a deterministic VM-context-required diagnostic.

`spl_object_id` and `spl_object_hash` are standalone runtime builtins backed by
the stable runtime object identity. The hash is a deterministic 32-character
lowercase hexadecimal rendering of that identity.

The VM accepts SPL exception classes as internal throwable catch types and maps
their parent hierarchy for catch matching and `instanceof`.

Prompt 06.37 adds an internal-object MVP for core SPL iterator classes:

- `ArrayIterator`
- `RecursiveArrayIterator`
- `IteratorIterator`
- `LimitIterator`
- `EmptyIterator`
- `AppendIterator`

The MVP snapshots array/object sources into runtime iterator objects and supports
the methods required by Phase 6 foreach interop: `rewind()`, `valid()`,
`current()`, `key()`, `next()`, `count()`, `getArrayCopy()`, and
`AppendIterator::append()`/`addIterator()`. The VM recognizes these objects for
foreach, `instanceof` checks against the relevant SPL interfaces/classes, and
`count()` over Countable iterator MVP objects.

Prompt 06.38 adds Composer/framework-facing SPL container MVPs:

- `ArrayObject`
- `SplFixedArray`
- `SplObjectStorage`
- `SplDoublyLinkedList`
- `SplStack`
- `SplQueue`

`ArrayObject` and `SplFixedArray` support one-dimensional ArrayAccess reads and
writes, Countable behavior, foreach iteration, and their common storage methods.
`SplObjectStorage` stores attached objects by runtime object identity and
supports attach/detach/contains/count/foreach plus info access through the
method API. The list/stack/queue classes use simple internal vector storage for
push/pop/shift/unshift/top/bottom/count and foreach.

Prompt 06.39 adds SPL file class MVPs:

- `SplFileInfo`
- `SplFileObject`
- `SplTempFileObject`

`SplFileInfo` exposes path, basename, realpath, size, mtime, and simple file
predicates through the VM's existing root-constrained filesystem capability
policy. `SplFileObject` loads allowed local files into deterministic line
storage and supports `fgets()`, `fgetcsv()` with a simple delimiter MVP,
`rewind()`, `eof()`, and foreach over lines. `SplTempFileObject` exposes an empty
`php://temp`-style in-memory MVP for path and size checks.

## Known Gaps

The following gaps are tracked in `docs/known-gaps-phase6.md`:

- `PHASE6-GAP-SPL-INTERFACE-METHOD-SURFACES`
- `PHASE6-GAP-SPL-AUTOLOAD-ADVANCED`
- `PHASE6-GAP-SPL-OBJECT-HASH-PARITY`
- `PHASE6-GAP-SPL-ITERATOR-MUTATION-EDGES`
- `PHASE6-GAP-SPL-ITERATOR-FULL-API`
- `PHASE6-GAP-SPL-CONTAINER-FULL-API`
- `PHASE6-GAP-SPL-CONTAINER-NESTED-ARRAYACCESS`
- `PHASE6-GAP-SPL-FILE-FULL-API`
- `PHASE6-GAP-SPL-FILE-CSV-FLAGS`
