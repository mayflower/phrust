# PHPT Triage

Baseline `20260624T210848Z` covers 21548 PHPTs: 1056 PASS, 64 SKIP, 19973 FAIL, 455 BORK.

Per-module PASS/SKIP counts are based on the latest available full-run results.

## Top Failing Modules

| Module | Priority | Corpus | PASS | SKIP | FAIL | BORK | Known non-green |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| phpt.foundation | 1 | 0 | 0 | 0 | 0 | 0 | 0 |
| phpt.runner | 2 | 0 | 0 | 0 | 0 | 437 | 437 |
| phpt.cli | 3 | 350 | 2 | 0 | 274 | 0 | 275 |
| zend.basic | 4 | 3509 | 274 | 1 | 3226 | 0 | 3227 |
| operators.conversions | 5 | 129 | 11 | 0 | 118 | 0 | 118 |
| diagnostics.output | 6 | 0 | 0 | 0 | 0 | 0 | 0 |
| strings.literals | 7 | 9 | 0 | 0 | 9 | 0 | 9 |
| arrays.references | 8 | 273 | 13 | 0 | 260 | 0 | 260 |
| functions.callables | 9 | 887 | 46 | 2 | 817 | 0 | 818 |
| objects.classes | 10 | 2136 | 136 | 0 | 1999 | 0 | 2000 |
| filesystem.streams | 11 | 1194 | 28 | 4 | 1100 | 0 | 1100 |
| standard.arrays | 12 | 821 | 85 | 0 | 735 | 0 | 735 |
| standard.strings | 13 | 727 | 81 | 0 | 621 | 0 | 621 |
| standard.math | 14 | 171 | 8 | 0 | 163 | 0 | 163 |
| standard.variables | 15 | 446 | 8 | 2 | 435 | 0 | 435 |
| standard.serialization | 16 | 126 | 10 | 0 | 115 | 0 | 115 |
| json | 17 | 88 | 9 | 0 | 79 | 0 | 79 |
| pcre | 18 | 165 | 36 | 1 | 126 | 0 | 126 |
| date | 19 | 687 | 11 | 1 | 675 | 0 | 675 |
| spl | 20 | 520 | 26 | 1 | 493 | 0 | 493 |

## Top Failure Clusters

| Cluster | Count |
| --- | ---: |
| runtime-error-or-diagnostic | 11402 |
| runtime-unsupported-feature | 6185 |
| runtime-output-mismatch | 2315 |
| needs-triage | 320 |
| frontend-parse-or-compile | 187 |
| runtime-timeout | 19 |

## Top Unsupported Feature Guesses

| Guess | Count |
| --- | ---: |
| runtime-unsupported-feature | 6185 |

## BORK Subclasses

| Subclass | Count |
| --- | ---: |
| malformed-or-non-utf8-phpt | 313 |
| missing-target-cli-capability | 96 |
| unsupported-section | 21 |
| other-bork | 11 |
| unsupported-file-external | 6 |
| malformed-or-incomplete-phpt | 5 |
| unsupported-expectation | 2 |
| unsupported-runner-io | 1 |

## Next Module Candidates

| Rank | Module | Reason |
| ---: | --- | --- |
| 1 | phpt.runner | 437 non-green, leverage 98 |
| 2 | phpt.cli | 274 non-green, leverage 96 |
| 3 | zend.basic | 3226 non-green, leverage 94 |
| 4 | operators.conversions | 118 non-green, leverage 92 |
| 5 | strings.literals | 9 non-green, leverage 88 |
| 6 | arrays.references | 260 non-green, leverage 86 |
| 7 | functions.callables | 817 non-green, leverage 84 |
| 8 | objects.classes | 1999 non-green, leverage 82 |
| 9 | filesystem.streams | 1100 non-green, leverage 80 |
| 10 | standard.arrays | 735 non-green, leverage 78 |

## Raw Corpus Module Counts

| Module | Corpus | PASS | SKIP | FAIL | BORK | Known non-green |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| zend | 5305 | 388 | 1 | 4908 | 7 | 4916 |
| unknown | 1419 | 133 | 0 | 1268 | 17 | 1286 |
| standard | 1140 | 51 | 4 | 1062 | 23 | 1085 |
| filesystem | 947 | 23 | 1 | 867 | 56 | 923 |
| dom | 879 | 0 | 0 | 872 | 7 | 879 |
| standard.arrays | 871 | 85 | 0 | 785 | 1 | 786 |
| spl | 784 | 32 | 1 | 750 | 0 | 751 |
| date | 689 | 11 | 1 | 677 | 0 | 677 |
| standard.strings | 741 | 82 | 0 | 634 | 25 | 659 |
| soap | 589 | 0 | 12 | 571 | 6 | 577 |
| phar | 553 | 1 | 0 | 411 | 141 | 552 |
| reflection | 494 | 21 | 0 | 473 | 0 | 473 |
| intl | 477 | 0 | 10 | 466 | 0 | 467 |
| opcache | 593 | 144 | 0 | 448 | 0 | 449 |
| mysqli | 442 | 0 | 0 | 435 | 4 | 442 |
| mbstring | 420 | 2 | 4 | 393 | 21 | 414 |
| sapi | 347 | 1 | 0 | 272 | 73 | 346 |
| gd | 312 | 0 | 2 | 309 | 0 | 310 |
| session | 260 | 0 | 0 | 257 | 2 | 260 |
| streams | 252 | 5 | 3 | 238 | 6 | 244 |
| openssl | 208 | 0 | 5 | 203 | 0 | 203 |
| uri | 191 | 0 | 0 | 191 | 0 | 191 |
| curl | 170 | 0 | 0 | 168 | 0 | 170 |
| bcmath | 166 | 0 | 0 | 166 | 0 | 166 |
| pdo_mysql | 159 | 0 | 0 | 159 | 0 | 159 |
| simplexml | 157 | 0 | 0 | 157 | 0 | 157 |
| zend_test | 148 | 1 | 0 | 147 | 0 | 147 |
| ldap | 140 | 0 | 0 | 139 | 1 | 140 |
| zlib | 143 | 2 | 1 | 128 | 12 | 140 |
| pdo | 137 | 0 | 0 | 134 | 2 | 137 |
| pcre | 165 | 36 | 1 | 126 | 2 | 128 |
| filter | 120 | 0 | 0 | 117 | 0 | 120 |
| sockets | 106 | 0 | 0 | 106 | 0 | 106 |
| ffi | 106 | 1 | 0 | 105 | 0 | 105 |
| zip | 103 | 1 | 0 | 101 | 1 | 102 |
| pgsql | 100 | 0 | 0 | 100 | 0 | 100 |
| gmp | 99 | 0 | 0 | 99 | 0 | 99 |
| sqlite3 | 96 | 0 | 0 | 96 | 0 | 96 |
| exif | 93 | 0 | 0 | 92 | 1 | 93 |
| pdo_sqlite | 80 | 0 | 0 | 79 | 1 | 80 |
