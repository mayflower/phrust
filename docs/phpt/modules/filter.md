# filter

- Strategy: validation and sanitization MVP
- Selected manifest: `tests/phpt/manifests/modules/filter.selected.jsonl`
- Selected fixture: `tests/phpt/generated/filter/basic.phpt`

## Implemented Surface

`filter_var` covers `FILTER_DEFAULT`, `FILTER_VALIDATE_EMAIL`,
`FILTER_VALIDATE_URL`, `FILTER_VALIDATE_INT`, `FILTER_VALIDATE_FLOAT`,
`FILTER_VALIDATE_BOOLEAN`, `FILTER_SANITIZE_EMAIL`, and
`FILTER_SANITIZE_URL`.

Unsupported filter identifiers return an explicit builtin value diagnostic
instead of silently accepting unknown behavior.

## Gaps

The full PHP filter flag and option matrix, `filter_input` SAPI source routing,
and locale-specific numeric parsing remain out of scope.

## Target Gates

- `nix develop -c cargo test -p php_runtime filter`
- `nix develop -c just phpt-dev-module MODULE=filter`
