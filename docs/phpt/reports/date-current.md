# Date PHPT Current Report

focused Date/Time verification.

## Scope

- request-local timezone state
- `time`, `microtime`, `date`, and `gmdate` selected formatting
- `DateTime` and `DateTimeImmutable` runtime constructors and selected methods
- `DateTimeZone` focused registry behavior
- `checkdate()` Gregorian bounds validation
- restricted `strtotime` ISO, timestamp, and day-relative parsing
- `DateInterval` ISO subset properties, formatting, and DateTime add/sub

## Selected Manifest

- `tests/phpt/manifests/modules/date.selected.jsonl`
- 7 generated fixtures under `tests/phpt/generated/date/`
- 2 upstream fixtures: `ext/date/tests/DateTimeZone_getName_basic1.phpt`,
  `ext/date/tests/006.phpt`

## Results

- `nix develop -c cargo fmt --check`: PASS
- `nix develop -c cargo test -p php_runtime datetime`: PASS, 1 test
- `nix develop -c cargo test -p php_runtime date_functions_parse_format_and_use_request_timezone -- --nocapture`: PASS, 1 test
- `nix develop -c cargo test -p php_vm date_ -- --nocapture`: PASS, 2 tests
- `nix develop -c cargo test -p php_runtime checkdate -- --nocapture`: PASS, 1 test
- `nix develop -c cargo test -p php_vm`: PASS, 483 tests
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=date`:
  PASS, reference 9 PASS and target 9 PASS. Source integrity verified
  24,475 php-src manifest entries with 0 skipped host-generated entries.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just diff-json-pcre-date`: PASS, total 3, pass 3, fail 0, skip 0, known_gap 0.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-stdlib`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-phpt`: PASS. Source integrity verified 24,475 php-src manifest entries with 0 skipped host-generated entries.

## Full PHPT

`verify-phpt` passes against the accepted PHPT baseline with the branch-local
`third_party/php-src` checkout. The run validated 21,548 corpus entries, 20,428
known non-green fingerprints, and source integrity for 24,475 php-src manifest
entries with 0 skipped host-generated entries.

## Remaining Gaps

- full timelib natural-language parsing
- full timezone database, historical transition, alias, and DST behavior
- DatePeriod and `createFromFormat`
- complete Date/Time class/interface method and property surfaces
- byte-perfect Date/Time warning and exception text
