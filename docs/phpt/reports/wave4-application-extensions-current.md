# Application Extension PHPT Current State

Branch: `phpt/wave4-application-extension-promotion`

Oracle: PHP 8.5.7 from
`/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`.

## Inventory

| Module | Selected outcomes | Current state |
| --- | ---: | --- |
| `json` | 15 PASS | Selected gate is green after promoting 5 upstream rows and adding `JSON_NUMERIC_CHECK` support. |
| `pcre` | 11 PASS | Selected gate is green after promoting 6 upstream rows. |
| `date` | 9 PASS | Selected gate is green after promoting 2 upstream rows and adding `checkdate()`. |
| `spl` | 76 PASS, 1 SKIP, 131 FAIL | Broad selected gate remains red, but the upstream `spl_autoload_bug48541.phpt`, `spl_autoload_010.phpt`, and `spl_autoload_013.phpt` rows now pass after preserving evaluated `eval()` concat operands, honoring autoload prepend order, delaying function-local class declarations until execution, and exposing closure/autoload callback metadata in PHP's expected shape. |
| `reflection` | 22 PASS | Selected aggregate gate is green. |
| `mysqli` | 5 PASS | Default-off selected gate is green after promoting mysqlnd metadata probes. |
| `pdo` | 4 PASS | Selected gate is green after promoting driver-list coverage. |
| `pdo_sqlite` | 3 PASS | Selected gate is green. |
| `sqlite3` | 2 PASS | Selected gate is green. |
| `curl` | 5 PASS, 2 SKIP | Selected gate is green with optional live network cases skipped. |
| `openssl` | 4 PASS | Selected gate is green after promoting cipher metadata probes. |
| `xml` | 5 PASS | Selected gate is green after promoting parser option coverage. |
| `simplexml` | 4 PASS | Selected gate is green. |
| `intl` | 5 PASS | Selected gate is green after promoting ICU constant probes. |
| `phar` | 2 PASS, 4 SKIP | Selected gate is green with optional compression/signature probes skipped when unsupported. |
| `session` | 7 PASS | Selected gate is green after promoting request-local session metadata coverage. |
| `wp.db-network` | 7 PASS, 3 SKIP | Default DB/network gate is green with optional live cases skipped. |

Every reference-backed module run listed above verified the pinned php-src
source-integrity manifest when it ran in this checkout.

## Before/After Selected Counts

| Extension | Before | After | Delta |
| --- | ---: | ---: | ---: |
| JSON | 10 PASS | 15 PASS | +5 upstream PHPTs |
| PCRE | 5 PASS | 11 PASS | +6 upstream PHPTs |
| Date | 7 PASS | 9 PASS | +2 upstream PHPTs |
| SPL | 18 PASS, 1 SKIP, 189 FAIL | 76 PASS, 1 SKIP, 131 FAIL | +3 upstream autoload PHPTs in the current selected aggregate; broad aggregate remains blocked |
| Reflection | 22 PASS | 22 PASS | unchanged |
| DB/network modules | 25 PASS, 5 SKIP | 34 PASS, 5 SKIP | +9 upstream PHPTs; optional live rows stayed default-off |
| XML/SimpleXML/Intl | 10 PASS | 14 PASS | +4 upstream PHPTs |
| PHAR/session | 2 PASS | 9 PASS, 4 SKIP | +11 upstream PHPTs; optional PHAR capability rows skip when unsupported |

The DB/network count includes `mysqli`, `pdo`, `pdo_sqlite`, `sqlite3`,
`curl`, `openssl`, and `wp.db-network`.

## Implementation Notes

- `json_encode()` now honors `JSON_NUMERIC_CHECK` for whole PHP numeric
  strings by reusing the runtime numeric-string classifier.
- `checkdate()` is registered in the date builtin module and in the standard
  library extension descriptor.
- `mysqli` exposes mysqlnd client info/version helpers used by upstream
  metadata probes.
- `curl` exposes escape/unescape, multi-error text, and additional
  `curl_version()` metadata while keeping live network execution default-off.
- `openssl` exposes selected cipher method and IV-length metadata without
  claiming full encryption or certificate parity.
- `xml` parser resources now retain selected parser options and support
  `xml_parser_create_ns()`.
- `intl` exposes ICU version constants and selected locale/error-message
  metadata.
- `Phar` exposes selected static supported-compression and supported-signature
  metadata; unavailable optional capabilities remain visible as upstream skips.
- `session` exposes request-local cache, module, save-path, and write-close
  helpers.
- No selected-manifest rows were removed or reclassified to hide failures.

## Promoted Upstream PHPTs

### JSON

- `ext/json/tests/json_encode_pretty_print.phpt`
- `ext/json/tests/json_encode_numeric.phpt`
- `ext/json/tests/pass002.phpt`
- `ext/json/tests/pass003.phpt`
- `ext/json/tests/json_encode_pretty_print2.phpt`

### PCRE

- `ext/pcre/tests/preg_match_basic.phpt`
- `ext/pcre/tests/preg_quote_basic.phpt`
- `ext/pcre/tests/preg_split_basic.phpt`
- `ext/pcre/tests/preg_grep_basic.phpt`
- `ext/pcre/tests/001.phpt`
- `ext/pcre/tests/grep.phpt`

### Date

- `ext/date/tests/DateTimeZone_getName_basic1.phpt`
- `ext/date/tests/006.phpt`

### SPL

- `ext/spl/tests/spl_autoload_bug48541.phpt`
- `ext/spl/tests/spl_autoload_010.phpt`
- `ext/spl/tests/spl_autoload_013.phpt`

### DB, cURL, and OpenSSL

- `ext/mysqli/tests/mysqli_get_client_info.phpt`
- `ext/mysqli/tests/functions/mysqli_get_client_info.phpt`
- `ext/mysqli/tests/mysqli_get_client_version.phpt`
- `ext/pdo/tests/pdo_drivers_basic.phpt`
- `ext/curl/tests/curl_escape.phpt`
- `ext/curl/tests/curl_multi_strerror_001.phpt`
- `ext/curl/tests/curl_version_basic_001.phpt`
- `ext/openssl/tests/openssl_get_cipher_methods.phpt`
- `ext/openssl/tests/gh19994.phpt`

### XML and Intl

- `ext/xml/tests/xml_parser_get_option_variation3.phpt`
- `ext/xml/tests/xml_parser_set_option_basic.phpt`
- `ext/intl/tests/intl_icu_data_version_constant.phpt`
- `ext/intl/tests/intl_icu_version_constant.phpt`

### PHAR and Session

- `ext/phar/tests/phar_get_supportedcomp1.phpt`
- `ext/phar/tests/phar_get_supportedcomp2.phpt`
- `ext/phar/tests/phar_get_supportedcomp4.phpt`
- `ext/phar/tests/phar_get_supported_signatures_002.phpt`
- `ext/phar/tests/phar_get_supported_signatures_002a.phpt`
- `ext/session/tests/session_cache_expire_basic.phpt`
- `ext/session/tests/session_cache_limiter_basic.phpt`
- `ext/session/tests/session_module_name_basic.phpt`
- `ext/session/tests/session_name_basic.phpt`
- `ext/session/tests/session_save_path_basic.phpt`
- `ext/session/tests/session_write_close_basic.phpt`

## Probed But Not Promoted

These upstream rows were target-probed and left out because they exposed
unsupported behavior or warning/diagnostic parity gaps:

- `ext/json/tests/json_decode_error.phpt`
- `ext/json/tests/json_decode_exceptions.phpt`
- `ext/json/tests/pass001.phpt`
- `ext/json/tests/serialize.phpt`
- `ext/json/tests/json_exceptions_error_clearing.phpt`
- `ext/pcre/tests/preg_match_all_basic.phpt`
- `ext/pcre/tests/002.phpt`
- `ext/pcre/tests/003.phpt`
- `ext/pcre/tests/grep2.phpt`
- `ext/pcre/tests/preg_replace_basic.phpt`
- `ext/pcre/tests/preg_replace.phpt`
- `ext/date/tests/DateInterval_format.phpt`
- `ext/date/tests/DateTimeZone_construct_basic.phpt`
- `ext/date/tests/DateTimeZone_compare.phpt`
- `ext/date/tests/DateTimeZone_getOffset_basic1.phpt`
- `ext/date/tests/DateTime_format_basic1.phpt`
- `ext/date/tests/DateTime_construct_basic1.phpt`
- `ext/intl/tests/locale_get_primary_language.phpt`
- `ext/session/tests/session_id_basic2.phpt`
- `ext/phar/tests/phar_get_supportedcomp3.phpt`

Additional SimpleXML, XML, and Intl candidates were probed and left out when
they required broader DTD/declaration parsing, SAX callback state, libxml error
buffers, ICU locale harness helpers, or optional PHAR compression state outside
this wave.

## Verification

Passed:

- `nix develop -c cargo fmt --check`
- `nix develop -c cargo test -p php_semantics`
- `nix develop -c cargo test -p php_runtime json -- --nocapture`
- `nix develop -c cargo test -p php_runtime checkdate -- --nocapture`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_std`
- `nix develop -c cargo test -p php_ir`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c cargo test -p php_runtime session`
- `nix develop -c cargo test -p php_server`
- `nix develop -c cargo build -q -p php_vm_cli --bin phrust-php`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=json`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=pcre`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=date`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mysqli`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=pdo`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=pdo_sqlite`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=sqlite3`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=curl`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=openssl`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=xml`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=simplexml`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=intl`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=phar`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=session`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.db-network`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_PASS=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-fast MODULE=spl FILE=ext/spl/tests/spl_autoload_bug48541.phpt`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_PASS=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-fast MODULE=spl FILE=ext/spl/tests/spl_autoload_010.phpt`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_PASS=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-fast MODULE=spl FILE=ext/spl/tests/spl_autoload_013.phpt`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=spl.autoload`
- `REFERENCE_PHP=$PWD/third_party/php-src/sapi/cli/php PHP_SRC_DIR=$PWD/third_party/php-src nix develop -c just verify-stdlib`
- `REFERENCE_PHP=$PWD/third_party/php-src/sapi/cli/php PHP_SRC_DIR=$PWD/third_party/php-src nix develop -c just verify-runtime`
- `REFERENCE_PHP=$PWD/third_party/php-src/sapi/cli/php PHP_SRC_DIR=$PWD/third_party/php-src nix develop -c just verify-server`
- `REFERENCE_PHP=$PWD/third_party/php-src/sapi/cli/php PHP_SRC_DIR=$PWD/third_party/php-src nix develop -c just verify-phpt`

`verify-stdlib` included `diff-json-pcre-date` and `diff-spl-reflection`.
`verify-server` included the server smoke gate.

Inventory gates run before the promotion:

- `nix develop -c just phpt-dev-module MODULE=json`
- `nix develop -c just phpt-dev-module MODULE=pcre`
- `nix develop -c just phpt-dev-module MODULE=date`
- `nix develop -c just phpt-dev-module MODULE=spl`
- `nix develop -c just phpt-dev-module MODULE=reflection`
- `nix develop -c just phpt-dev-module MODULE=mysqli`
- `nix develop -c just phpt-dev-module MODULE=pdo`
- `nix develop -c just phpt-dev-module MODULE=pdo_sqlite`
- `nix develop -c just phpt-dev-module MODULE=sqlite3`
- `nix develop -c just phpt-dev-module MODULE=curl`
- `nix develop -c just phpt-dev-module MODULE=openssl`
- `nix develop -c just phpt-dev-module MODULE=xml`
- `nix develop -c just phpt-dev-module MODULE=simplexml`
- `nix develop -c just phpt-dev-module MODULE=intl`
- `nix develop -c just phpt-dev-module MODULE=phar`
- `nix develop -c just phpt-dev-module MODULE=session`
- `nix develop -c just phpt-dev-module MODULE=wp.db-network`

Failed:

- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=spl` now reports 76 PASS, 1 SKIP, and 131 FAIL in the target run.

`spl` is the only non-green selected module gate. It is not broadened in this
slice; the current failures remain visible instead of being skipped or
reclassified.

## Optional Live I/O

No live DB or network environment was enabled during this wave. The default
module runs kept the optional live cases skipped:

- `curl`: 5 PASS, 2 SKIP.
- `phar`: 2 PASS, 4 SKIP.
- `wp.db-network`: 7 PASS, 3 SKIP.

## Remaining Gaps

- JSON: `JsonSerializable`, invalid UTF-8, recursion, and complete
  error/warning parity.
- PCRE: advanced modifiers, `preg_filter`, callback arrays, `preg_match_all`
  edge shapes, replacement parity, UTF-8 diagnostics, and locale behavior.
- Date: full timelib parsing, complete timezone database behavior, DatePeriod,
  `createFromFormat`, advanced interval behavior, and byte-perfect diagnostics.
- SPL: missing heap, queue, doubly-linked-list, caching, recursive iterator,
  filesystem link, and serialization parity.
- DB/network: optional live MySQL and cURL cases remain default-off and require
  explicit local environment variables.
- XML/Intl: full SAX callbacks, libxml global error buffers, ICU locale data,
  and complete grapheme/normalization behavior remain deferred.
- PHAR/session: writable archives, signature enforcement, custom session
  handlers, persistent session storage, and INI policy parity remain deferred.
