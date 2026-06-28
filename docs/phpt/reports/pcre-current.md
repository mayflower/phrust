# PCRE PHPT Current Report

Reference target: PHP 8.5.7 (`php-8.5.7`).

## Selected Harness

| Scope | PASS | SKIP | FAIL | BORK |
| --- | ---: | ---: | ---: | ---: |
| focused PCRE selected manifest | 5 | 0 | 0 | 0 |

Selected manifest: `tests/phpt/manifests/modules/pcre.selected.jsonl`

## Before and After

| Measurement | Corpus | PASS | SKIP | FAIL | BORK |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before the selected gate upstream snapshot | 165 | 41 | 5 | 117 | 0 |
| selected harness | 5 | 5 | 0 | 0 | 0 |
| Full upstream probe, excluding host-locale reference failure | 164 | 50 | 10 | 104 | 0 |

`PHPT_RUN_FULL=1 just phpt-full-regression` was attempted with the pinned
reference PHP binary. It was stopped after more than 17 minutes in the
timeout-heavy late SAPI/FPM tail before producing a final summary, so it is not
counted as proof for this report.

## Top PHPT Files

- `tests/phpt/generated/pcre/preg-match-captures.phpt`
- `tests/phpt/generated/pcre/preg-last-error-state.phpt`
- `tests/phpt/generated/pcre/preg-replace-split-grep-quote.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback.phpt`
- `tests/phpt/generated/pcre/preg-replace-callback-invalid.phpt`

## Top Blockers

- Broad upstream `ext/pcre` still includes unsupported PCRE surfaces such as
  `preg_filter`, `preg_replace_callback_array`, advanced modifiers, callouts,
  and UTF-8 diagnostic edge cases.
- Several upstream failures are warning or stack formatting parity issues rather
  than selected semantic MVP gaps.
- `ext/pcre/tests/locales.phpt` is host-locale-sensitive in the local PHP
  reference build and failed the reference leg during the broad probe, so it is
  excluded from the current focused harness.
- Full regression proof still needs a completed full-suite run; the selected
  PCRE module gate and source-integrity check pass.

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
