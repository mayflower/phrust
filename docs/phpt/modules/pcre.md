# pcre

- Priority: 18.6 focused harness
- Selected manifest: `tests/phpt/manifests/modules/pcre.selected.jsonl`
- Prompt 18 close gate: 5 PASS, 0 SKIP, 0 FAIL, 0 BORK from 5 selected fixtures
- Upstream corpus snapshot before Prompt 18: 41 PASS, 5 SKIP, 117 FAIL, 0 BORK
  from 165 corpus candidates

## Scope

- `preg_match` captures, named captures, `PREG_OFFSET_CAPTURE`, and offset-base
  handling
- `preg_match_all` set-order capture arrays with named groups and unmatched
  nulls
- `preg_last_error` and `preg_last_error_msg` request-local state across VM
  builtin calls
- `preg_replace` simple replacements, backrefs, limits, and by-reference count
- `preg_split` delimiter capture and no-empty flags
- `preg_grep` positive matching
- `preg_quote` delimiter escaping
- `preg_replace_callback` named function, closure, and invalid-callable dispatch

## Non-Scope

- Full upstream `ext/pcre` corpus promotion
- PCRE JIT, callouts, `(*MARK)`, and every PHP modifier edge
- `preg_filter`, `preg_replace_callback_array`, array/method callback edge cases
- Byte-perfect warning text, stack formatting, UTF-8 edge diagnostics, and
  locale-sensitive ctype behavior

## Selected PHPT Fixtures

- `tests/phpt/generated/pcre/preg-match-captures.phpt`
- `tests/phpt/generated/pcre/preg-last-error-state.phpt`
- `tests/phpt/generated/pcre/preg-replace-split-grep-quote.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback-invalid.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/pcre.rs`
- `crates/php_runtime/src/builtins/context.rs`
- `crates/php_runtime/src/builtins/modules/pcre.rs`
- `crates/php_runtime/src/builtins/modules/core.rs`
- `crates/php_vm/src/vm/mod.rs`

## Target Gates

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=pcre`
- `nix develop -c cargo test -p php_runtime pcre`
- `nix develop -c cargo test -p php_vm`

## Prompt 18 Evidence

- Replaced the broad 165-test selected manifest with a focused five-test PCRE
  harness before broad upstream promotion.
- Added generated oracle fixtures for match/match_all capture shape, PCRE
  last-error state, replace/split/grep/quote behavior, callback dispatch, and
  invalid callback diagnostics.
- PCRE last-error state is now owned by VM execution state and shared with each
  builtin context for the request.
- Capture conversion now emits named keys and absolute offsets for selected
  `preg_match` and `preg_match_all` paths.
- `preg_replace` count is bound by reference through the VM builtin argument
  bridge.
- `preg_replace_callback` uses real VM callable dispatch for named functions and
  closures.

## Known Gaps

- Full upstream `ext/pcre` still has unsupported feature, warning parity,
  callback-array, UTF-8, and locale-sensitive cases.
- Direct runtime-registry callback use remains limited to internal callables;
  userland callback dispatch is covered through the VM path.
- Fatal stack formatting for invalid callbacks is intentionally matched by
  regex in the selected PHPT while VM diagnostic source-span parity remains a
  broader diagnostics task.

## Next Step

Expand from the generated Prompt 18 harness into upstream `ext/pcre` PHPT
promotion after the remaining warning, UTF-8, locale, callback-array, and
advanced PCRE gaps close.
