# mbstring

- Strategy: bounded UTF-8 MVP
- Selected manifest: `tests/phpt/manifests/modules/mbstring.selected.jsonl`
- Selected gate: 3 generated PHPTs covering platform visibility, UTF-8 common
  functions, UTF-8/ASCII detection, request-local internal encoding, and narrow
  UTF-8 conversion
- Corpus snapshot: 420 `mbstring`-owned candidates in
  `tests/phpt/manifests/phpt-corpus.jsonl`; committed baseline counts are
  3 PASS, 36 SKIP, 360 FAIL, 21 BORK, and 414 known non-green outcomes.

## Decision

Enable a deliberately narrow mbstring surface for Composer and framework probes:

- `mb_strlen`
- `mb_substr`
- `mb_strtolower`
- `mb_strtoupper`
- `mb_detect_encoding`
- `mb_check_encoding`
- `mb_internal_encoding`
- `mb_convert_encoding` for UTF-8 to UTF-8 only
- `mb_strpos`

The implementation uses the existing runtime and Rust standard-library Unicode
primitives. It does not introduce a full encoding database, mbregex, Oniguruma,
locale data, or broad upstream `ext/mbstring` parity.

The default project oracle at
`/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php` was built
without mbstring. The selected selected outputs were captured from a temporary
read-only PHP 8.5.7 clone at `/tmp/php-src-mbstring-oracle/sapi/cli/php` built
from the same php-src checkout with `--enable-mbstring --disable-mbregex`.

## Runtime Contract

- `extension_loaded("mbstring")` returns `true`.
- `function_exists()` returns `true` only for the selected MVP functions.
- Unsupported mbstring functions outside the selected surface remain absent
  rather than returning fake results.
- Supported encodings are `UTF-8` and `ASCII` aliases accepted by the selected
  fixtures.
- `mb_internal_encoding()` is request-local runtime state with default `UTF-8`.
- Unsupported encodings return deterministic unsupported diagnostics or `false`
  at the selected API boundary.

## Required PHPTs

Required for this strategy:

- `tests/phpt/generated/mbstring/platform-checks.phpt`
- `tests/phpt/generated/mbstring/utf8-common-functions.phpt`
- `tests/phpt/generated/mbstring/utf8-encoding-state.phpt`
- `tests/phpt/generated/wp.stdlib/text-encoding-basic.phpt` covers the
  `mb_strpos` addition selected by the WordPress stdlib harness.

These PHPTs keep the enabled surface explicit and reference-backed without
promoting the full upstream mbstring corpus.

## Optional PHPTs

Promote upstream tests only after matching their behavior with focused
reference-backed fixtures:

- UTF-8/common-function basics:
  - `ext/mbstring/tests/mb_strlen.phpt`
  - `ext/mbstring/tests/mb_strlen_basic.phpt`
  - `ext/mbstring/tests/mb_substr.phpt`
  - `ext/mbstring/tests/mb_substr_basic.phpt`
  - `ext/mbstring/tests/mb_strtolower_basic.phpt`
  - `ext/mbstring/tests/mb_strtoupper_basic.phpt`
  - `ext/mbstring/tests/mb_detect_encoding.phpt`
- Encoding error cases for the scoped functions:
  - `ext/mbstring/tests/mb_strlen_error2.phpt`
  - `ext/mbstring/tests/mb_substr_error2.phpt`
  - `ext/mbstring/tests/mb_strtolower_error2.phpt`
  - `ext/mbstring/tests/mb_strtoupper_error2.phpt`
  - `ext/mbstring/tests/mb_detect_encoding_empty_encoding_list.phpt`

## Out-of-Scope PHPTs

Out of scope for this MVP:

- Exhaustive encoding verification/conversion suites such as
  `*_encoding.phpt`, `utf_encodings.phpt`, `sjis*_encoding.phpt`,
  `euc*_encoding.phpt`, `iso2022*_encoding.phpt`, `cp*_encoding.phpt`,
  `ucs2_encoding.phpt`, and `ucs4_encoding.phpt`.
- Conversion APIs outside the selected UTF-8 no-op case, including
  `mb_convert_encoding*`, `mb_convert_kana`, `mb_convert_variables`,
  `mb_encode_mimeheader`, and `mb_decode_mimeheader`.
- mbstring regex and Oniguruma behavior, including `mb_regex_encoding*` and
  related regex/callback tests.
- HTTP/input/output encoding translation, `zend.multibyte`, mail/mime helpers,
  mobile carrier encodings, security regression tests for mbfl internals, and
  full bug-report regression coverage.
- `ext/exif` tests that require mbstring only as an implementation dependency.

## Unicode and Encoding Gaps

| Stable ID | Reference behavior summary | Current phrust behavior | Fixture path | Next owner layer |
| --- | --- | --- | --- | --- |
| `PHPT-MBSTRING-UNSUPPORTED-GRAPHEME-LENGTH` | PHP mbstring counts according to selected mbstring encoding tables and has broader invalid-sequence handling. | `mb_strlen` counts Rust `char` values for valid UTF-8 and bytes for valid ASCII. | `tests/phpt/generated/mbstring/utf8-common-functions.phpt` | `php_runtime` mbstring implementation |
| `PHPT-MBSTRING-UNSUPPORTED-FULL-CASE-MAPPING` | PHP case mapping follows mbstring tables for selected encodings. | `mb_strtolower` and `mb_strtoupper` use Rust Unicode case mapping for selected UTF-8 examples only. | `tests/phpt/generated/mbstring/utf8-common-functions.phpt` | `php_runtime` mbstring implementation |
| `PHPT-MBSTRING-UNSUPPORTED-DETECT-ENCODING-MATRIX` | `mb_detect_encoding` depends on PHP's full supported encoding list, aliases, detection order, strict mode, and invalid-sequence handling. | Detection is limited to explicit UTF-8 and ASCII candidates. | `tests/phpt/generated/mbstring/utf8-encoding-state.phpt` | `php_runtime` mbstring implementation |
| `PHPT-MBSTRING-UNSUPPORTED-LEGACY-ENCODINGS` | Shift-JIS, EUC-JP, ISO-2022-JP, Big5, GB18030, ISO-8859 variants, CP932, UTF-7, and related encodings require mbstring conversion tables. | Only UTF-8/ASCII checks and UTF-8 to UTF-8 conversion are enabled. | `tests/phpt/generated/mbstring/utf8-encoding-state.phpt` | future encoding library or table strategy |
| `PHPT-MBSTRING-UNSUPPORTED-MBREGEX` | PHP mbstring regex uses Oniguruma-backed behavior. | No mbregex APIs are exposed. | none selected | future mbregex layer |

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `REFERENCE_PHP=/tmp/php-src-mbstring-oracle/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mbstring`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-phpt`
