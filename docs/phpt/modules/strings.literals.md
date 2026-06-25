# strings.literals

- Priority: 7
- Selected manifest: `tests/phpt/manifests/modules/strings.literals.selected.jsonl`
- Current counts: 0 PASS, 0 SKIP, 9 FAIL, 0 BORK from 9 corpus candidates

## Scope

- string literal decoding
- heredoc/nowdoc
- string interpolation basics

## Non-Scope

- full ext/standard string API

## Relevant PHPT Paths

- `tests/strings/offsets_general.phpt`
- `tests/strings/offsets_chaining_5.phpt`
- `tests/strings/offsets_chaining_3.phpt`
- `tests/strings/offsets_chaining_1.phpt`
- `tests/strings/bug26703.phpt`
- `tests/strings/bug22592.phpt`
- `tests/strings/004.phpt`
- `tests/strings/002.phpt`
- `tests/strings/001.phpt`

## Relevant php-src Source Areas

- `crates/php_lexer/`
- `crates/php_syntax/`
- `crates/php_runtime/`

## Target Gates

- `nix develop -c just verify-frontend`

## Known Gaps

- `runtime-error-or-diagnostic`: 6
- `runtime-unsupported-feature`: 2
- `runtime-output-mismatch`: 1

## Next Step

Separate frontend literal gaps from runtime string builtin gaps.
