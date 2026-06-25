# diagnostics.output

- Priority: 6
- Selected manifest: `tests/phpt/manifests/modules/diagnostics.output.selected.jsonl`
- Current counts: 5 PASS, 0 SKIP, 0 FAIL, 0 BORK from 5 corpus candidates

## Scope

- warnings
- notices
- fatal formatting
- display_errors
- error_reporting
- file and line output
- output channels
- warning continuation
- catchable builtin diagnostics
- catchable invalid operand diagnostics

## Non-Scope

- full php.ini parsing
- extension loading
- complete Zend fatal backtrace formatting

## Relevant PHPT Paths

- `tests/phpt/generated/diagnostics.output/undefined-variable-warning.phpt`
- `tests/phpt/generated/diagnostics.output/array-to-string-warning.phpt`
- `tests/phpt/generated/diagnostics.output/invalid-operand-type-error.phpt`
- `tests/phpt/generated/diagnostics.output/builtin-arity-error.phpt`
- `tests/phpt/generated/diagnostics.output/builtin-type-error.phpt`

## Relevant php-src Source Areas

- `crates/php_runtime/`
- `crates/php_vm/`

## Target Gates

- `nix develop -c just verify-runtime`
- `nix develop -c just phpt-module MODULE=diagnostics.output`

## Known Gaps

- CLI diagnostic wording is centralized for selected warnings and fatal channels,
  but full Zend uncaught exception/backtrace wording is still intentionally
  outside this module.
- File/line display uses the runtime source map available to the executed
  instruction. Some builtin-origin diagnostics still report the best available
  call source until instruction-level builtin spans are threaded through every
  dispatch path.
- Full php.ini parsing and extension-origin diagnostics remain outside the
  current runtime boundary.

## Next Step

Close remaining exact wording and uncaught fatal formatting gaps while preserving centralized diagnostic rendering.
