# standard.strings

- Priority: 13
- Selected manifest: `tests/phpt/manifests/modules/standard.strings.selected.jsonl`
- Corpus baseline: 352 PASS, 42 SKIP, 308 FAIL, 0 BORK from 727 corpus candidates
- focused gate: 37 PASS, 0 SKIP, 0 FAIL, 0 BORK

## Scope

- Common binary-safe string helpers
- URL query parsing through `parse_str`
- URL query building through `http_build_query`
- Promoted upstream formatting, substring replacement, and word wrapping PHPTs
- Focused generated fixtures for length, substring, search, trimming,
  split/join, formatted output, replacement, ASCII case conversion, and
  tokenizer state

## Non-Scope

- Full upstream string corpus
- Complete formatting matrix
- Charset/encoding-heavy behavior

## Relevant PHPT Paths

- `tests/phpt/generated/standard.strings/strlen-substr-binary-smoke.phpt`
- `tests/phpt/generated/standard.strings/strpos-contains-smoke.phpt`
- `tests/phpt/generated/standard.strings/trim-explode-implode-smoke.phpt`
- `tests/phpt/generated/standard.strings/printf-sprintf-smoke.phpt`
- `tests/phpt/generated/standard.strings/strtok-state-smoke.phpt`
- `tests/phpt/generated/standard.strings/str-replace-case-smoke.phpt`
- `ext/standard/tests/http/http_build_query/http_build_query.phpt`
- `ext/standard/tests/strings/substr_replace.phpt`
- `ext/standard/tests/strings/substr_replace_array.phpt`
- `ext/standard/tests/strings/sprintf_basic2.phpt`
- `ext/standard/tests/strings/sprintf_basic3.phpt`
- `ext/standard/tests/strings/sprintf_basic4.phpt`
- `ext/standard/tests/strings/sprintf_basic5.phpt`
- `ext/standard/tests/strings/sprintf_basic6.phpt`
- `ext/standard/tests/strings/vsprintf_basic1.phpt`
- `ext/standard/tests/strings/vsprintf_basic2.phpt`
- `ext/standard/tests/strings/printf_basic2.phpt`
- `ext/standard/tests/strings/wordwrap.phpt`
- `ext/standard/tests/strings/wordwrap_basic.phpt`
- `ext/standard/tests/strings/wordwrap_error.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/strings.rs`
- `crates/php_runtime/src/context.rs`
- `crates/php_runtime/src/value.rs`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings`
- `nix develop -c just verify-stdlib`

## Evidence

- Added generated coverage for `str_replace`, `strtolower`, and `strtoupper`
  to complete the selected standard string builtin list.
- Promoted upstream `http_build_query`, `substr_replace`, `sprintf`,
  `vsprintf`, `printf`, and `wordwrap` coverage.
- Latest focused target run: PASS, 37 selected PHPTs.
- Latest oracle-backed stdlib aggregate run: PASS.

## Known Gaps

- The full upstream string corpus remains larger than this focused slice.
- Additional formatting, encoding, flag, and uncommon helper behavior remains
  backlog work.
- `parse_str` shares the request parser's selected query-decoding surface; the
  exhaustive bracket-name matrix remains backlog work.
- Object-to-string paths in `http_build_query`, flag-sensitive
  `htmlspecialchars`/decode behavior, and `str_ireplace` remain follow-up
  candidates.
