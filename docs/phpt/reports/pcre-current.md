# PCRE PHPT Current Report

Reference target: PHP 8.5.7 (`php-8.5.7`).

## Selected Harness

| Scope | PASS | SKIP | FAIL | BORK |
| --- | ---: | ---: | ---: | ---: |
| focused PCRE selected manifest | 11 | 0 | 0 | 0 |

Selected manifest: `tests/phpt/manifests/modules/pcre.selected.jsonl`

## Before and After

| Measurement | Corpus | PASS | SKIP | FAIL | BORK |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before the selected gate upstream snapshot | 165 | 41 | 5 | 117 | 0 |
| selected harness | 11 | 11 | 0 | 0 | 0 |
| Full upstream probe, excluding host-locale reference failure | 164 | 50 | 10 | 104 | 0 |

`verify-phpt` now passes against the accepted PHPT baseline with the pinned
reference PHP binary and source-integrity check enabled.

## Top PHPT Files

- `tests/phpt/generated/pcre/preg-match-captures.phpt`
- `tests/phpt/generated/pcre/preg-last-error-state.phpt`
- `tests/phpt/generated/pcre/preg-replace-split-grep-quote.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback-invalid.phpt`
- `ext/pcre/tests/preg_match_basic.phpt`
- `ext/pcre/tests/preg_quote_basic.phpt`
- `ext/pcre/tests/preg_split_basic.phpt`
- `ext/pcre/tests/preg_grep_basic.phpt`
- `ext/pcre/tests/001.phpt`
- `ext/pcre/tests/grep.phpt`

## Top Blockers

- Broad upstream `ext/pcre` still includes unsupported PCRE surfaces such as
  `preg_filter`, `preg_replace_callback_array`, advanced modifiers, callouts,
  `preg_match_all` edge shapes, replacement parity, and UTF-8 diagnostic cases.
- Several upstream failures are warning or stack formatting parity issues rather
  than selected semantic MVP gaps.
- `ext/pcre/tests/locales.phpt` is host-locale-sensitive in the local PHP
  reference build and failed the reference leg during the broad probe, so it is
  excluded from the current focused harness.
- The accepted full PHPT baseline remains broad and intentionally non-green, but
  the selected PCRE module gate and source-integrity check pass.

## Owners

- Runtime PCRE helpers and pattern cache:
  `crates/php_runtime/src/pcre.rs`
- Runtime builtin context and PCRE state:
  `crates/php_runtime/src/builtins/context.rs`
- Runtime PCRE builtins:
  `crates/php_runtime/src/builtins/modules/pcre.rs`
- Callback-backed replacement helpers:
  `crates/php_runtime/src/builtins/modules/core.rs`
- VM request state, by-reference builtin binding, and userland callback
  dispatch:
  `crates/php_vm/src/vm/mod.rs`
