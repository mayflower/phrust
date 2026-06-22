# Phase 5 Coverage Matrix

This matrix is the final Phase 5 differential coverage snapshot. It was
generated with the pinned PHP 8.5.7 reference binary:

```bash
nix develop -c env REFERENCE_PHP=third_party/php-src/sapi/cli/php just phase5-diff --out target/phase5/diff-reference
```

Result:

```text
total=280 pass=230 fail=0 skip=0 known_gap=50
```

When `REFERENCE_PHP` is not set, `just phase5-diff` skips pass-candidate
runtime comparisons and still reports known-gap fixtures. The required
`verify-phase5` gate is therefore strict when a reference binary is explicitly
provided and skip-explicit when the local reference binary is unavailable.

| Category | Pass | Fail | Known gap | Skip | Fixture root or examples |
| --- | ---: | ---: | ---: | ---: | --- |
| `arrays` | 12 | 0 | 0 | 0 | `fixtures/phase5/arrays/*.php` |
| `callables` | 19 | 0 | 0 | 0 | `fixtures/phase5/callables/*.php` |
| `clone_with` | 5 | 0 | 4 | 0 | `fixtures/phase5/clone_with/*.php` |
| `closures` | 12 | 0 | 0 | 0 | `fixtures/phase5/closures/*.php` |
| `comparisons` | 5 | 0 | 0 | 0 | `fixtures/phase5/comparisons/*.php` |
| `const_expr` | 0 | 0 | 4 | 0 | `fixtures/phase5/const_expr/*.php` |
| `conversions` | 3 | 0 | 4 | 0 | `fixtures/phase5/conversions/*.php` |
| `cow` | 2 | 0 | 1 | 0 | `fixtures/phase5/cow/*.php` |
| `destructors` | 4 | 0 | 1 | 0 | `fixtures/phase5/destructors/*.php` |
| `enums` | 11 | 0 | 2 | 0 | `fixtures/phase5/enums/*.php` |
| `errors` | 5 | 0 | 1 | 0 | `fixtures/phase5/errors/*.php` |
| `fibers` | 11 | 0 | 0 | 0 | `fixtures/phase5/fibers/*.php` |
| `foreach` | 14 | 0 | 2 | 0 | `fixtures/phase5/foreach/*.php` |
| `functions` | 5 | 0 | 2 | 0 | `fixtures/phase5/functions/*.php` |
| `gc` | 0 | 0 | 4 | 0 | `fixtures/phase5/gc/*.php` |
| `generators` | 15 | 0 | 1 | 0 | `fixtures/phase5/generators/*.php` |
| `globals` | 7 | 0 | 0 | 0 | `fixtures/phase5/globals/*.php` |
| `include_eval_autoload` | 14 | 0 | 3 | 0 | `fixtures/phase5/include_eval_autoload/*.php` |
| `known_gaps` | 0 | 0 | 3 | 0 | `fixtures/phase5/known_gaps/*.php` |
| `magic` | 12 | 0 | 2 | 0 | `fixtures/phase5/magic/*.php` |
| `objects` | 23 | 0 | 0 | 0 | `fixtures/phase5/objects/*.php` |
| `pipe` | 6 | 0 | 0 | 0 | `fixtures/phase5/pipe/*.php` |
| `properties` | 6 | 0 | 0 | 0 | `fixtures/phase5/properties/*.php` |
| `property_hooks` | 6 | 0 | 2 | 0 | `fixtures/phase5/property_hooks/*.php` |
| `real_world` | 1 | 0 | 2 | 0 | `fixtures/phase5/real_world/*.php` |
| `reflection` | 11 | 0 | 3 | 0 | `fixtures/phase5/reflection/*.php` |
| `refs` | 5 | 0 | 1 | 0 | `fixtures/phase5/refs/*.php` |
| `regressions` | 2 | 0 | 1 | 0 | `fixtures/phase5/regressions/**/*.php` |
| `traits` | 9 | 0 | 0 | 0 | `fixtures/phase5/traits/*.php` |
| `types` | 5 | 0 | 6 | 0 | `fixtures/phase5/types/*.php` |
| `void_cast` | 0 | 0 | 1 | 0 | `fixtures/phase5/void_cast/*.php` |

## Known-Gap Summary

Every known-gap fixture in the reference-backed report declares a stable
`known_gap=<ID>` in the fixture metadata. The active known-gap groups are:

- Reference/property lvalue gaps: property references, static property
  references, array-element return references, and by-reference temporary
  foreach sources.
- PHP-exact diagnostics and warning channels: numeric-string warning output,
  include warning rendering, undefined-variable warnings, and fatal error text.
- Deferred runtime breadth: standard-library/SPL/Reflection expansion,
  serialization, `ArrayAccess`, public GC APIs, enum serialization, and
  Composer-style autoload/stdlib coverage.
- Deferred execution matrices: constant-expression runtime values, string
  offset COW writes, generator by-reference yields, clone-with restricted
  property rules, property-hook recursion/visibility edges, and destructor/GC
  cycle behavior.

## Unsupported ID Cleanup

The final audit replaced newly discovered generic pass-fixture mismatches with
specific IDs:

- `E_PHP_RUNTIME_NUMERIC_STRING_WARNING_CHANNEL`
- `E_PHP_RUNTIME_TYPEERROR_TEXT_COMPAT`
- `E_PHP_RUNTIME_UNINITIALIZED_PROPERTY_TEXT_COMPAT`
- `E_PHP_RUNTIME_UNION_TYPEERROR_TEXT_COMPAT`

Remaining broad IDs in `docs/phase5-known-gaps.md` are intentionally reserved
for whole Phase 6+ capability areas that Phase 5 does not execute, such as
standard-library breadth, stream wrappers, Zend ABI, SAPI, Opcache, and JIT.
