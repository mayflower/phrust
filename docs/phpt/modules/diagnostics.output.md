# diagnostics.output

- Priority: 6
- Selected manifest: `tests/phpt/manifests/modules/diagnostics.output.selected.jsonl`
- Current counts: 5 PASS, 0 SKIP, 0 FAIL, 0 BORK from 5 selected
  generated candidates

## Scope

- warnings
- notices
- fatal formatting
- display_errors
- output channels

## Non-Scope

- exact wording for intentionally unsupported extensions

## Relevant PHPT Paths

- `tests/phpt/generated/diagnostics.output/array-to-string-warning.phpt`
- `tests/phpt/generated/diagnostics.output/builtin-arity-error.phpt`
- `tests/phpt/generated/diagnostics.output/builtin-type-error.phpt`
- `tests/phpt/generated/diagnostics.output/invalid-operand-type-error.phpt`
- `tests/phpt/generated/diagnostics.output/undefined-variable-warning.phpt`

## Relevant php-src Source Areas

- `crates/php_runtime/`
- `crates/php_vm/`

## Target Gates

- `nix develop -c just phpt-module MODULE=diagnostics.output`
- `nix develop -c just verify-runtime`

Last focused run on 2026-06-26:

- Selected module gate:
  `nix develop -c just phpt-module MODULE=diagnostics.output`
  - Reference: 5 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Target: 5 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Source integrity: 24476 php-src manifest entries verified

Covered selected-gate behavior:

- warning formatting and continuation for undefined variables
- warning formatting and continuation for array-to-string conversion
- catchable builtin arity errors
- catchable builtin type errors
- catchable invalid operand `TypeError`

## Known Gaps

Full PHP diagnostic wording parity remains broader than this gate. Exact
messages, stack traces, and channels for unsupported extensions and advanced
runtime features are tracked in the owning feature modules rather than in this
cross-cutting selected diagnostics gate.

## Next Step

Keep the selected diagnostic channel gate green while broader PHP wording parity
is expanded through affected feature modules.
