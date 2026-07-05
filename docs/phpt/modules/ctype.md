# ctype

- Strategy: complete upstream ext/ctype selected target plus local smoke rows
- Selected manifest: `tests/phpt/manifests/modules/ctype.selected.jsonl`
- Selected fixtures:
  - `tests/phpt/generated/ctype/basic.phpt`
  - `tests/phpt/generated/ctype/fallbacks.phpt`
  - `ext/ctype/tests/*.phpt`

## Implemented Surface

The runtime exposes the common `ctype_*` predicates used by dependency probes:
`ctype_alnum`, `ctype_alpha`, `ctype_cntrl`, `ctype_digit`, `ctype_graph`,
`ctype_lower`, `ctype_print`, `ctype_punct`, `ctype_space`, `ctype_upper`, and
`ctype_xdigit`.

The selected rows cover the full upstream PHP 8.5.7 `ext/ctype` PHPT target set,
plus local smoke fixtures for ASCII byte classification and PHP 8.5's legacy
non-string fallback behavior for integer codepoints, out-of-range integers,
object diagnostics, and false-returning non-string values.

The implementation is deterministic ASCII behavior. Empty strings return
`false`.

## Gaps

`ext/ctype/tests/lc_ctype_inheritance.phpt` is selected but skips on the local
host when the required `de_DE` locale is unavailable.

## Target Gates

- `nix develop -c cargo test -p php_runtime ctype`
- `nix develop -c just phpt-dev-module MODULE=ctype`

Last focused upstream target sweep: 48 PASS, 1 SKIP, 0 FAIL for the 49 upstream
`ext/ctype` rows.
