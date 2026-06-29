# ctype

- Strategy: ASCII C-locale MVP
- Selected manifest: `tests/phpt/manifests/modules/ctype.selected.jsonl`
- Selected fixture: `tests/phpt/generated/ctype/basic.phpt`

## Implemented Surface

The runtime exposes the common `ctype_*` predicates used by dependency probes:
`ctype_alnum`, `ctype_alpha`, `ctype_cntrl`, `ctype_digit`, `ctype_graph`,
`ctype_lower`, `ctype_print`, `ctype_punct`, `ctype_space`, `ctype_upper`, and
`ctype_xdigit`.

The implementation is deterministic ASCII behavior. Empty strings return
`false`.

## Gaps

Locale-sensitive classification and broader PHP coercion edge cases remain out
of scope for this slice.

## Target Gates

- `nix develop -c cargo test -p php_runtime ctype`
- `nix develop -c just phpt-dev-module MODULE=ctype`
