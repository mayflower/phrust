# WordPress Language VM Core Status

This status note tracks the reduced WordPress-facing language and VM core
fixtures under `fixtures/runtime_semantics/wp_language_vm/`.

## Implementation Summary

- Added the `wp_language_vm` runtime-semantics category and `just wp-language-vm`
  gate.
- Added reduced fixtures for assignment-valued expressions, compound
  assignment, null-coalescing assignment, dynamic access in conditions,
  destructuring targets, static dispatch, visibility, magic static/instance
  dispatch, truthiness, `isset`/`empty`/coalesce, array lvalues, by-reference
  arguments, static locals, and class static property lvalues.
- Extended the runtime-semantics report schema for this category with
  `wordpress_error_class`, `fixture_id`, `diagnostic_ids`,
  `primary_diagnostic`, `unsupported_operation`, and `reduced_fixture`.
- Fixed lowering for nested destructuring, property `??=`, quiet array-dim
  coalesce fetches, and array-dim compound assignment single-evaluation.
- Fixed late-static binding display spelling for inherited static method calls
  while keeping normalized names for lookup and scope checks.

## Fixtures And Error Classes

The reduced category requires each fixture to declare:

- `wordpress_error_class=frontend_lowering`
- `wordpress_error_class=runtime_dispatch`
- `wordpress_error_class=runtime_semantics`

The current category has 17 pass-candidate fixtures:

| Error class | Fixtures |
| --- | ---: |
| `frontend_lowering` | 5 |
| `runtime_dispatch` | 7 |
| `runtime_semantics` | 5 |

## Diagnostics

The diff report records structured diagnostic IDs from `runtime-diagnostic:`
JSON lines and best-effort `E_PHP_*` IDs from stderr/stdout. Passing fixtures
have no primary diagnostic or unsupported operation.

## Tests

Latest reference-backed gate:

```bash
nix develop -c env REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php just wp-language-vm
```

Result:

```text
total=17 pass=17 fail=0 skip=0 known_gap=0
```

Supporting checks:

```bash
nix develop -c cargo test -p php_ir
nix develop -c cargo test -p php_vm
```

Both checks passed in this worktree.

## Known Gaps

No fixture in `wp_language_vm` is currently marked as a known gap. Broader
runtime semantics gaps remain tracked in `docs/runtime/semantics-known-gaps.md`
and `docs/runtime/known-gaps.md`.

## Merge Notes

- The category is additive and does not change the existing aggregate
  runtime-semantics category list unless that aggregate explicitly selects
  `wp_language_vm`.
- `target/runtime-semantics/wp-language-vm/` is generated output and should not
  be committed.
