# SPL and Reflection Current Status

Generated: 2026-06-28

Branch: `phpt/b3-spl-reflection`

Reference target:

- PHP source: `/Volumes/CrucialMusic/src/phrust/third_party/php-src`
- PHP binary:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

## Branch Scope

This branch advances the selected SPL and Reflection PHPT slices from the Branch
3 prompt pack. It keeps the work inside the existing standard-library, VM, and
PHPT tooling boundaries:

- promote focused upstream SPL and Reflection PHPTs only after the covered
  behavior is backed by implemented runtime or metadata;
- keep target evidence branch-local with `CARGO_TARGET_DIR`,
  `PHPT_WORK_DIR`, `TARGET_PHP`, and `PHPT_TOOLS_BIN`;
- compare against the pinned PHP 8.5.7 oracle with fresh reference and target
  PHPT runs; and
- leave the broad aggregate SPL failures visible instead of accepting a new
  baseline.

## SPL Status

| Module | Target result | Notes |
| --- | --- | --- |
| `spl.interfaces` | PASS, 1 selected PHPT | Generated submodule fixture remains green. |
| `spl.array-iterator` | PASS, 6 selected PHPTs | Generated iterator fixture plus focused iterator helper and upstream iterator-count/to-array cases are green. |
| `spl.array-object` | PASS, 2 selected PHPTs | Generated array-object fixture plus `spl_001.phpt` are green. |
| `spl.fixed-array` | PASS, 2 selected PHPTs | Generated fixed-array fixture plus JSON encoding parity are green. |
| `spl.object-storage` | PASS, 2 selected PHPTs | Generated object-storage fixture plus object-key `offsetGet` coverage are green. |
| `spl.doubly-linked-list` | PASS, 6 selected PHPTs | Generated list/stack/queue fixture plus selected current/key/isEmpty/offsetExists cases are green. |
| `spl.file` | PASS, 2 selected PHPTs | Generated file-class fixture plus leading-dot `getExtension()` coverage are green. |
| `spl.autoload` | PASS, 2 selected PHPTs | Generated autoload fixture plus `spl_autoload_003.phpt` are green. |
| `spl` | FAIL, 17 PASS, 2 SKIP, 189 FAIL | Aggregate selected SPL still includes legacy upstream selected failures. |

The aggregate `spl` reference run completed green enough for comparison with
206 PASS and 2 SKIP. The target-side failures remain concentrated in broader
upstream SPL areas that are outside the generated MVP submodule fixtures:
serialization parity, recursive/caching/tree iterators, heap/priority queue
classes, iterator helper functions, by-reference foreach over objects, and
selected filesystem edges such as symlink support.

## SPL Work Completed

- `iterator_count()` and `iterator_to_array()` now cover array inputs and the
  existing Traversable/ArrayIterator path.
- Userland classes can implement internal SPL interfaces such as `Countable`,
  and `count($object)` dispatches to their `count()` method.
- `SplObjectStorage` bracket assignment accepts object keys for direct
  ArrayAccess attachment.
- `SplDoublyLinkedList` covers selected upstream `current()`, empty `key()`,
  `isEmpty()`, and `offsetExists()` behavior.
- `SplFileInfo::getExtension()` covers leading-dot basenames.
- `json_encode(new SplFixedArray(...))` emits array-shaped JSON instead of
  internal storage properties.

## Reflection Status

| Module | Target result | Notes |
| --- | --- | --- |
| `reflection.functions` | PASS, 3 selected PHPTs | Generated function fixture plus upstream extension-name and closure checks are green. |
| `reflection.parameters` | PASS, 3 selected PHPTs | Parameter arginfo/IR metadata, variadic, and position checks are green. |
| `reflection.classes` | PASS, 5 selected PHPTs | Class metadata plus enum, namespace, abstract, and extension-name checks produced 4 PASS and 1 SKIP. |
| `reflection.methods` | PASS, 1 selected PHPT | Method metadata fixture remains green. |
| `reflection.properties` | PASS, 2 selected PHPTs | Property metadata fixture plus upstream modifier coverage are green. |
| `reflection.attributes` | PASS, 1 selected PHPT | Attribute metadata fixture remains green. |
| `reflection.enums` | PASS, 4 selected PHPTs | Enum metadata fixture plus upstream backed-type/case checks are green. |
| `reflection.extensions` | PASS, 3 selected PHPTs | Extension metadata fixture plus upstream name/class-list checks produced 2 PASS and 1 SKIP. |
| `reflection` | PASS, 22 selected PHPTs | Aggregate selected Reflection gate produced 20 PASS, 2 SKIP, and 0 non-green outcomes. |

Reflection is currently healthier than SPL in the selected gates: every
submodule and the aggregate selected module pass. Future Reflection work should
still promote upstream PHPTs only when their metadata can be read from real
arginfo, semantic, IR, runtime, or class metadata.

## Reflection Work Completed

- Added `ReflectionParameter::getPosition()` for internal and userland
  parameters using real parameter position metadata.
- Added `ReflectionMethod::getModifiers()` for internal and userland methods
  using metadata-backed public/protected/private/static/final/abstract bits.
- Registered the generated-arginfo-backed Reflection extension in the standard
  library registry and loaded-extension snapshot.
- Preserved case-insensitive `ReflectionExtension` lookup while reporting the
  canonical display name `Reflection`.
- Promoted focused upstream Reflection PHPTs for functions, parameters, classes,
  properties, enums, and extensions.

An upstream `ReflectionMethod_getModifiers_basic.phpt` probe still depends on
object stringification/interpolation and invocation behavior outside this branch
scope, so it was not promoted into the selected manifest.

## Verification

Branch-local gates run with the pinned reference PHP, branch-local PHPT target
binaries, and fresh reference/target results:

- `nix develop -c just phpt-dev-module MODULE=spl.interfaces`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.array-iterator`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.array-object`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.fixed-array`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.object-storage`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.doubly-linked-list`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.file`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl.autoload`: PASS.
- `nix develop -c just phpt-dev-module MODULE=spl`: FAIL, target 189
  non-green outcomes.
- `nix develop -c just phpt-dev-module MODULE=reflection.functions`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.parameters`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.classes`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.methods`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.properties`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.attributes`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.enums`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection.extensions`: PASS.
- `nix develop -c just phpt-dev-module MODULE=reflection`: PASS, 22
  selected PHPTs with 0 non-green outcomes.
- `nix develop -c just diff-spl-reflection`: PASS, 2 pass, 0 fail.
- `nix develop -c just verify-stdlib`: PASS.
- `nix develop -c just verify-phpt`: PASS.
- `cargo test -p php_std`: PASS.
- `cargo test -p php_runtime`: PASS.
- `cargo test -p php_runtime object`: PASS.
- `cargo test -p php_vm`: PASS.

Every PHPT module run also verified the pinned `php-src` source-integrity
manifest: 24,475 entries checked, 0 skipped.

## Branch Closeout Impact

| Area | Before branch | After branch | Impact |
| --- | --- | --- | --- |
| SPL focused submodules | Generated MVP fixtures green, 1 selected PHPT per submodule | All touched SPL submodules green with 1-6 selected PHPTs each | Focused upstream coverage promoted for array iterators, containers, file info, and autoload without hiding aggregate SPL failures |
| SPL aggregate selected | 17 PASS, 2 SKIP, 189 FAIL | 17 PASS, 2 SKIP, 189 FAIL | Aggregate remains red; selected branch fixtures and promoted close-scope tests pass, while broad legacy SPL gaps stay visible |
| Reflection focused submodules | Generated MVP fixtures green, 1 selected PHPT per submodule | All touched Reflection submodules green with 1-5 selected PHPTs each | Reflection coverage expanded from 8 selected fixtures to 22 selected PHPTs |
| Reflection aggregate selected | 8 PASS | 20 PASS, 2 SKIP, 0 non-green | Aggregate Reflection remains green after upstream promotion |
| Standard-library diff | SPL/Reflection diff slice available | `diff-spl-reflection` passes, 2 pass, 0 fail | Registry-backed SPL/Reflection stdlib behavior remains aligned with the diff smoke |

## Merge Risks

- Object branch work can conflict with the VM object/ArrayAccess dispatch paths
  touched for `SplObjectStorage` and SPL container offset reads/writes.
- Future object-core changes should preserve object identity semantics for
  `SplObjectStorage`; this branch intentionally does not use class names or
  debug strings as object keys.
- Reflection branch work should continue reading metadata from arginfo,
  frontend/IR, runtime class tables, or the standard-library registry; adding
  fake Reflection surfaces would invalidate the selected PHPTs promoted here.
- Enum, magic, clone, trait, invocation, and object stringification work remains
  owned by other branches. Upstream Reflection cases that depend on those
  behaviors should not be promoted here until those owning layers land.
- SPL aggregate failures are still expected after this branch. They should not
  be baseline-accepted as part of merging this focused SPL/Reflection slice.

## Remaining Gaps

The broad selected `spl` aggregate remains red and should stay visible until the
owning gaps are implemented. The current failure set is concentrated in
serialization parity, recursive/caching/tree iterators, heap/priority queue
classes, broader iterator helper functions, by-reference foreach over objects,
and selected filesystem edges such as symlink support.

The next SPL work should continue one subarea at a time, promoting upstream PHPTs
only after the covered behavior is implemented and the focused submodule gate is
green.
