# SPL, Reflection, and Composer Certification Current Report

Date: 2026-06-30

Oracle: `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

PHP source: `/Volumes/CrucialMusic/src/phrust/third_party/php-src`

## Selected SPL

Baseline reproduction:

| Run | PASS | SKIP | FAIL | Non-green |
| --- | ---: | ---: | ---: | ---: |
| Reference `spl` | 205 | 3 | 0 | 0 |
| Target `spl` before changes | 77 | 1 | 130 | 130 |
| Target `spl` after changes | 82 | 1 | 125 | 125 |

The target gate remains red, but the selected module now has five additional
net passing outcomes. Current targeted rows verified as passing include:

- `ext/spl/tests/spl_limit_iterator_check_limits.phpt`
- `ext/spl/tests/spl_iterator_iterator_constructor.phpt`
- `ext/spl/tests/spl_iterator_to_array_error.phpt`
- `ext/spl/tests/spl_iterator_caching_count_basic.phpt`

The implemented SPL slice covers constructor validation and catchable
diagnostics for `IteratorIterator` and `LimitIterator`, plus VM-level argument
type validation for `iterator_count()` and `iterator_to_array()`. The latter
two now produce the right `TypeError` text, but the upstream PHPT rows remain
red because the fatal output still lacks PHP's exact builtin stack frame and
source-location formatting. It also routes one-dimensional SPL ArrayAccess
offset read/write/append/isset/empty/unset for the covered iterator/container
objects and models `CachingIterator::FULL_CACHE` count progression during
foreach.

Current non-green target clusters:

| Cluster | Count |
| --- | ---: |
| Iterator adapters and iterator method surfaces | 64 |
| Heap and priority queue behavior | 17 |
| Recursive iterators and tree iterators | 17 |
| File objects | 4 |
| Object storage | 4 |
| Serialization and unserialization | 3 |
| Other diagnostics or isolated gaps | 16 |

Mismatch categories in the after run:

| Category | Count |
| --- | ---: |
| StdoutMismatch | 92 |
| RuntimeExitMismatch | 27 |
| UnsupportedFeature | 5 |
| CompileMismatch | 1 |

## Reflection

The selected `reflection` module remains green:

| Run | PASS | SKIP | FAIL | Non-green |
| --- | ---: | ---: | ---: | ---: |
| Reference `reflection` | 22 | 0 | 0 | 0 |
| Target `reflection` | 22 | 0 | 0 | 0 |

Reflection coverage expanded with bounded
`ReflectionAttribute::newInstance()` support. The VM now resolves userland
attribute classes from the request class table, constructs the attribute object,
and invokes the userland constructor with folded positional metadata arguments.
Full parity is still a known gap for target masks, repeatability validation,
autoload-sensitive lookup, named arguments, internal attributes, and exact
diagnostics.

## Composer And Source Mode

Composer/app smokes:

| Gate | Status |
| --- | --- |
| `just composer-smoke` | PASS: total=5 pass=5 fail=0 skip=0 known_gap=0 |
| `just composer-smoke-autoload` | PASS: total=1 pass=1 fail=0 skip=0 known_gap=0 |
| `just composer-smoke-platform` | PASS: total=2 pass=2 fail=0 skip=0 known_gap=0 |
| `just composer-smoke-source` | SKIP (clean): set `PHRUST_STDLIB_COMPOSER_SOURCE_DIR` to a local Composer source checkout |
| `just process-capability-smoke` | PASS |

Source mode now extracts missing functions, classes, methods, constants,
Reflection method markers, SPL method markers, runtime diagnostic IDs, and
warning/fatal lines. It writes structured `missing_symbols` and `diagnostics`
arrays to `target/stdlib/composer-source-smoke/report.json` and a
frequency-sorted `missing-symbols.txt` when source mode is configured.

No Packagist, Composer PHAR, plugins/scripts, network access, FPM, or external
PHP subprocess execution is introduced in the required gates.

## PHPT Promotion

No manifest rows were added or removed in this pass. The improved rows were
already selected in the `spl` module and became green through implementation.

Rows deliberately not promoted:

- `ext/spl/tests/iterator_count.phpt`: target produces the correct `TypeError`
  message text, but exact fatal stack/source formatting remains a diagnostics
  gap.
- `ext/spl/tests/iterator_to_array.phpt`: same diagnostics-formatting gap.
- Wider CachingIterator exception behavior, recursive iterator,
  heap/priority-queue, object-storage, file-object, and serialization rows
  remain visible in the selected `spl` module instead of being reclassified as
  skips.
- `ext/spl/tests/iterator_027.phpt` remains red; the current bridge supports
  one-dimensional offset operations, but this row needs deeper iterator state
  parity.

## Validation Snapshot

Focused Rust gates:

- `nix develop -c cargo test -p php_runtime spl` PASS
- `nix develop -c cargo test -p php_runtime reflection` PASS
- `nix develop -c cargo test -p php_vm reflection` PASS
- `nix develop -c cargo test -p php_vm objects` PASS
- `nix develop -c cargo test -p php_vm foreach` PASS
- `nix develop -c cargo test -p php_std` PASS

Selected PHPT modules:

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 ... just phpt-dev-module MODULE=spl` FAIL, reference 205 pass / 3 skip / 0 fail and target 82 pass / 1 skip / 125 fail.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 ... just phpt-dev-module MODULE=reflection` PASS.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 ... just phpt-dev-module MODULE=wp.core-language` PASS.

Remaining owners:

- SPL iterator adapters and recursive/tree iterators need method-surface and
  mutation/position parity.
- Heap/priority queue rows need corruption/write-lock/ordering parity.
- Object storage and nested/by-reference ArrayAccess rows need lvalue bridge
  work beyond the current MVP.
- Runtime diagnostics need PHP-style builtin stack frames/source positions for
  uncaught builtins to promote the `iterator_count()` and `iterator_to_array()`
  fatal rows.
