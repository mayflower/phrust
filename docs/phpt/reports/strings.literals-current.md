# strings.literals Current Focus Report

Core values/arrays/strings branch focused string literal and formatting
verification.

## Scope

- Selected upstream `tests/strings` literal and offset fixtures.
- `php://stdout` writes through formatted stream builtins.
- `highlight_string()` output and return-mode behavior for selected upstream
  fixtures.
- Legacy PHP float formatting for the selected `%-0 width.precision f` shape.

## Selected Manifest

- `tests/phpt/manifests/modules/strings.literals.selected.jsonl`
- 9 upstream php-src fixtures under `tests/strings/`

## Selected Fixtures

- `tests/strings/offsets_general.phpt`
- `tests/strings/offsets_chaining_5.phpt`
- `tests/strings/offsets_chaining_3.phpt`
- `tests/strings/offsets_chaining_1.phpt`
- `tests/strings/bug26703.phpt`
- `tests/strings/bug22592.phpt`
- `tests/strings/004.phpt`
- `tests/strings/002.phpt`
- `tests/strings/001.phpt`

## Before/After

Before this this pass, the selected target run had 6 PASS and 3 FAIL:
`highlight_string()` was undefined in two upstream fixtures, and `fprintf()` to
`php://stdout` did not contribute to PHPT stdout comparison.

| Check | Before | After |
| --- | ---: | ---: |
| `strings.literals` selected PHPTs | 6 PASS / 3 FAIL | 9 PASS |

## Closed Behaviors

- `fprintf()`, `vfprintf()`, and `fwrite()` mirror successful writes to
  `php://stdout` into the request output buffer.
- `highlight_string()` is implemented through the existing lexer and supports
  output mode plus `return: true` mode for the selected PHP highlighting shapes.
- `sprintf("%-010.2f", 2.5)` matches PHP's legacy fractional zero-fill output.

## Remaining String Gaps

- The highlighter is intentionally token-driven but bounded to the selected
  upstream coverage; broader comments, attributes, heredoc, and complex
  interpolation highlighting should be promoted with focused PHPTs.
- Broader heredoc/nowdoc and interpolation edge cases remain follow-up selected
  slices.

## Verification

Latest branch verification:

- `nix develop -c cargo test -p php_runtime highlight_string_renders_php_style_markup`: PASS, 1 test.
- `nix develop -c cargo test -p php_runtime formatting_builtins_cover_common_printf_surface`: PASS, 1 test.
- `nix develop -c cargo test -p php_runtime formatting_builtins_report_missing_args_and_stream_writes`: PASS, 1 test.
- `nix develop -c cargo test -p php_runtime`: PASS, 187 tests.
- `nix develop -c just phpt-dev-build`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=strings.literals`: PASS, reference 9 PASS and target 9 PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings`: PASS, reference 15 PASS and target 15 PASS.
