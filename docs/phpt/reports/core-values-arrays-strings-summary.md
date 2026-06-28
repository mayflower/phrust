# Core Values Arrays Strings Branch Summary

Prompt 2A through Prompt 2G moved the selected core values, arrays, strings,
standard-library, and bounded mbstring gates to green target runs against PHP
8.5.7 oracle output.

## Oracle Inputs

- Default PHP oracle:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`
- Read-only php-src source tree:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src`
- mbstring-enabled oracle for Prompt 2F only:
  `/tmp/php-src-mbstring-oracle/sapi/cli/php`, built from the same PHP 8.5.7
  checkout with `--enable-mbstring --disable-mbregex`

The default oracle was kept unchanged and does not include mbstring.

## Before And After

| Module | Before | After | Notes |
| --- | ---: | ---: | --- |
| `zend.basic` | 10 PASS | 10 PASS | Kept green while scalar/operator changes landed. |
| `operators.conversions` | 4 PASS | 5 PASS | Added invalid array operand coverage from the PHP oracle. |
| `strings.literals` | 6 PASS / 3 FAIL | 9 PASS | Closed `php://stdout` stream writes, `highlight_string`, and selected formatting output. |
| `arrays.references` | 18 PASS / 58 SKIP / 124 FAIL broad audit | 6 PASS | Replaced mixed broad audit gate with focused Prompt 2C local array/reference/COW fixtures. |
| `standard.arrays` | 10 PASS | 17 PASS | Added selected array builtin coverage through deterministic sort paths. |
| `standard.strings` | 15 PASS | 16 PASS | Added `str_replace`, `strtolower`, and `strtoupper` coverage. |
| `standard.math` | 161 PASS / 11 SKIP | 161 PASS / 11 SKIP | Reused existing selected math surface with no target failures. |
| `standard.variables` | 26 PASS / 1 SKIP | 26 PASS / 1 SKIP | Reused existing selected variable/debug-output surface with no target failures. |
| `standard.serialization` | 5 PASS | 5 PASS | Reused selected scalar, array, and simple object persistence surface. |
| `mbstring` | 3 disabled-stub PASS | 3 bounded UTF-8 MVP PASS | Enabled selected mbstring functions without exposing unsupported functions. |

## Implemented Behavior

- Scalar and operator conversion now covers leading numeric strings, selected
  overflow behavior, scalar comparison, concat, and invalid operand diagnostics.
- Runtime string behavior covers selected literal/offset PHPTs, binary-safe
  output paths, `php://stdout` stream writes, `highlight_string`, and selected
  PHP legacy formatting behavior.
- Core arrays cover ordered storage, key normalization, appends, unset holes,
  `isset`/`empty` dimensions, array element references, COW separation,
  by-value foreach snapshots, and by-reference foreach over local arrays.
- Standard array builtins cover `count`, key/value helpers, merge/slice/splice,
  column/search/unique/range, and deterministic sort/asort/ksort.
- Standard string, math, variable, and serialization selected gates remain
  green with the Prompt 2E additions documented in their module reports.
- mbstring now exposes a bounded UTF-8/ASCII MVP for `mb_strlen`, `mb_substr`,
  `mb_strtolower`, `mb_strtoupper`, `mb_detect_encoding`,
  `mb_check_encoding`, `mb_internal_encoding`, and UTF-8 to UTF-8
  `mb_convert_encoding`.

## Remaining Gaps

- Broad PHPT corpus failures outside the selected manifests remain tracked by
  the module manifests, triage report, and full baseline rather than hidden.
- Object-heavy, SPL, reflection, property-hook, extension, and callback-heavy
  array/reference behavior remains outside the core array/reference gate.
- Full upstream standard string formatting, locale-sensitive behavior,
  uncommon flags, and binary/encoding edge cases remain follow-up work.
- Serialization still does not claim full reference identity records, resource
  handling, magic hooks, `allowed_classes`, or deep object/reference graph
  parity.
- mbstring remains intentionally bounded: no full encoding database, no
  Shift-JIS/EUC-JP parity, no mbregex/Oniguruma, and no unsupported mbstring
  functions faked as present.

## Merge Risks

- Do not replace the focused selected gates with broad mixed corpus audits
  without first separating out-of-scope object, SPL, extension, and frontend
  blockers.
- Keep `third_party/php-src/` read-only. The mbstring oracle used for Prompt 2F
  is a temporary clone under `/tmp`, not a vendored dependency.
- Do not broaden mbstring extension visibility without a reference-backed
  implementation slice and selected PHPTs for each new function.
- Do not move object/callable semantics into the core values/arrays/strings
  branch just to make array or serialization edge cases pass.

## Closeout Verification

All commands below passed on this branch:

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-runtime`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-frontend`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_runtime array`
- `nix develop -c cargo test -p php_runtime reference`
- `nix develop -c cargo test -p php_vm`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zend.basic`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=operators.conversions`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=strings.literals`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=arrays.references`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.arrays`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.math`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.variables`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.serialization`
- `REFERENCE_PHP=/tmp/php-src-mbstring-oracle/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mbstring`
