# Phase 4 Runtime Fixture Matrix

Runtime fixtures are small, focused programs grouped by behavior:

| Directory | Purpose |
| --- | --- |
| `valid/scalars` | scalar output, expressions, comparisons, casts |
| `valid/variables` | locals, assignment, compound assignment, inc/dec |
| `valid/superglobals` | deterministic CLI `$argc`, `$argv`, and controlled superglobal arrays |
| `valid/control_flow` | branches, loops, switch, match, returns |
| `valid/functions` | calls, params, defaults, variadics, recursion, closures |
| `valid/arrays` | literals, indexed/string keys, dim ops, isset/empty/unset, `var_dump` |
| `valid/foreach` | by-value foreach forms and mutation snapshot behavior |
| `valid/includes` | local include/require MVP fixtures and include helper files |
| `valid/objects` | class table, properties, methods, clone, clone-with MVP |
| `valid/exceptions` | throw, catch, finally MVP behavior |
| `valid/php85` | PHP 8.5 pipe and clone-with runtime cases |
| `corpus_smoke` | tiny self-contained programs that resemble real PHP shapes without vendored projects |
| `invalid/*` | focused expected Rust runtime or compile failures |
| `known_gaps/*` | explicit unsupported/deferred runtime categories |

`just runtime-fixtures` runs the Rust VM-only regression gate without requiring
a PHP reference binary.

`just runtime-corpus-smoke` runs the self-contained corpus under
`corpus_smoke`, checks expected stdout and expected error exits, then compiles
and executes every `.php` file under `valid`. It writes a CI-friendly status
summary to `target/phase4/runtime-corpus-smoke/report.txt` with
`pass`, `known_gap`, `fail`, and `skip` counts.

Optional local Composer or application checkouts can be smoke-compiled by
setting `PHRUST_RUNTIME_CORPUS_EXTRA_PATHS` to a colon-separated list of local
directories. This path is intentionally optional, skipped in CI by default, and
limited by `PHRUST_RUNTIME_CORPUS_EXTRA_LIMIT` so external projects are never
vendored or required for Phase 4 validation.

Phase 4 does not claim Composer or framework compatibility. Large framework
smokes remain out of scope until the runtime grows the required language and
autoload surface.

`just runtime-diff` runs `compare-runtime`, writes per-fixture JSON plus
`runtime-report.json` under `target/phase4/runtime-diff`, and compares valid
fixtures against `REFERENCE_PHP` when configured. The runner records
`pass`, `fail`, `skipped`, and `known_gap` statuses and includes exit code,
stdout, normalized stderr/diagnostics, diagnostic IDs, and known-gap IDs.

Optional per-fixture metadata can be placed near the top of a fixture:

```php
// phase4-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_GLOBALS_ALIAS_MATRIX
// phase4-runtime: expect=pass args=alpha,beta normalize=path_lines php_ref_required=true
```

Supported keys are `expect`, `known_gap`, `args`, `normalize`, and
`php_ref_required`. Metadata is intentionally minimal so fixture behavior stays
visible in the PHP source.
