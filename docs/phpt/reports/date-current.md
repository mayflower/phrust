# Date PHPT Current Report

Prompt 19 focused Date/Time verification.

## Scope

- request-local timezone state
- `time`, `microtime`, `date`, and `gmdate` selected formatting
- `DateTime` and `DateTimeImmutable` runtime constructors and selected methods
- `DateTimeZone` focused registry behavior
- restricted `strtotime` ISO, timestamp, and day-relative parsing
- `DateInterval` ISO subset properties, formatting, and DateTime add/sub

## Selected Manifest

- `tests/phpt/manifests/modules/date.selected.jsonl`
- 7 generated fixtures under `tests/phpt/generated/date/`

## Results

- `nix develop -c cargo fmt --check`: PASS
- `nix develop -c cargo test -p php_runtime datetime`: PASS, 1 test
- `nix develop -c cargo test -p php_runtime date_functions_parse_format_and_use_request_timezone -- --nocapture`: PASS, 1 test
- `nix develop -c cargo test -p php_vm date_ -- --nocapture`: PASS, 2 tests
- `nix develop -c cargo test -p php_vm`: PASS, 347 tests
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_DISABLE_REFERENCE_REUSE=1 PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=date`: PASS, reference 7 PASS and target 7 PASS. Source integrity skipped because this checkout has no repo-local pinned php-src checkout.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just diff-json-pcre-date`: PASS, total 3, pass 3, fail 0, skip 0, known_gap 0.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-stdlib`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-phpt`: PASS. Source integrity skipped because this checkout has no repo-local pinned php-src checkout.

## Full PHPT

Not run. The full regression script requires a php-src tree for the committed
corpus and always runs source-integrity verification. This checkout has no
repo-local `third_party/php-src` or `third_party/php-src-8.5.7` tree. The
sibling clean `php-8.5.7` checkout at
`/Volumes/CrucialMusic/src/phrust/third_party/php-src` provides the reference
CLI used above, but it does not satisfy this branch's committed
`tests/phpt/manifests/php-src-hashes.jsonl` entry for `main/build-defs.h`
(`manifest=3410` bytes, checkout=`1871` bytes), so a full run cannot complete
the final source-integrity gate from this checkout.

## Remaining Gaps

- full timelib natural-language parsing
- full timezone database, historical transition, alias, and DST behavior
- DatePeriod and `createFromFormat`
- complete Date/Time class/interface method and property surfaces
- byte-perfect Date/Time warning and exception text
