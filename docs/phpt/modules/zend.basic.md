# zend.basic

- Priority: 4
- Selected manifest: `tests/phpt/manifests/modules/zend.basic.selected.jsonl`
- Current counts: 434 PASS, 40 SKIP, 3027 FAIL, 0 BORK from 3509 corpus candidates

## Scope

- top-level execution
- scalar literals
- numeric literal separators
- echo
- print
- statement sequencing
- top-level return
- top-level exit
- basic var_dump output

## Non-Scope

- dynamic variables
- objects
- extensions
- advanced type system
- exact string-to-float formatting edge cases

## Relevant PHPT Paths

- `tests/phpt/generated/zend.basic/regression-frameless_jmp_002-5695f7d89408.phpt`
- `tests/phpt/generated/zend.basic/regression-frameless_jmp_005-03f40d7fa877.phpt`
- `tests/phpt/generated/zend.basic/regression-numeric_literal_separator_001-e7854d367777.phpt`
- `tests/phpt/generated/zend.basic/smoke-array_self_add_globals-03f80836cf16.phpt`
- `tests/phpt/generated/zend.basic/smoke-echo-print-sequencing-5aec11b01afc.phpt`
- `tests/phpt/generated/zend.basic/smoke-frameless_jmp_003-58f09445c012.phpt`
- `tests/phpt/generated/zend.basic/smoke-top-level-exit-ff648fb6d646.phpt`
- `tests/phpt/generated/zend.basic/smoke-top-level-return-f9f31fdf1b6e.phpt`
- `tests/phpt/generated/zend.basic/smoke-var-dump-scalars-e7854d367777.phpt`
- `tests/phpt/generated/zend.basic/smoke-variable_with_integer_name-48644f4034d7.phpt`

Each generated PHPT carries provenance for the original `Zend/tests/...` source
and hash in its `--DESCRIPTION--` section and in the selected manifest.

## Relevant php-src Source Areas

- `Zend/tests/`
- `crates/php_vm/`
- `crates/php_runtime/`

## Target Gates

- `nix develop -c just phpt-module MODULE=zend.basic`

## Current Module Gate Status

Last focused run on 2026-06-26:

- Selected module gate:
  `nix develop -c just phpt-module MODULE=zend.basic`
  - Reference: 10 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Target: 10 PASS, 0 SKIP, 0 FAIL, 0 BORK
  - Source integrity: 24476 php-src manifest entries verified

Closed basis gaps:

- namespaced calls to global internal functions, e.g. `namespace Foo; strlen(...)`
- integer braced variable names, e.g. `${10}`
- PHP array union for `array + array`
- numeric literal separators
- echo/print sequencing
- top-level return and exit
- basic scalar `var_dump` output

## Known Gaps

- `runtime-error-or-diagnostic`: 1386
- `runtime-unsupported-feature`: 1133
- `runtime-output-mismatch`: 653
- `frontend-parse-or-compile`: 43
- `runtime-timeout`: 9

These are full-baseline clusters outside the current selected zend.basic gate.
The previous broad selected manifest also pulled in WeakMap/WeakReference,
advanced type declarations and variance, traits, advanced parameter defaults,
INI parsing helpers, exact floating-point string formatting, and diagnostic
wording parity; those belong to later functional modules and are not part of
this selected basis gate.

## Next Step

Continue with `operators.conversions` after keeping the selected zend.basic gate
green.
