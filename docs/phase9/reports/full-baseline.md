# Phase 9 Full PHPT Baseline

Generated: `20260624T125543Z`

## Totals

| Outcome | Count |
| --- | ---: |
| BORK | 455 |
| FAIL | 19973 |
| PASS | 1056 |
| SKIP | 64 |

## Top Failure Clusters

| Cluster | Count |
| --- | ---: |
| runtime-error-or-diagnostic | 11400 |
| runtime-unsupported-feature | 6185 |
| runtime-output-mismatch | 2315 |
| needs-triage | 320 |
| frontend-parse-or-compile | 187 |
| runtime-timeout | 21 |

## Top Failing Modules

| Module | Count |
| --- | ---: |
| zend | 4916 |
| unknown | 1286 |
| standard | 1085 |
| filesystem | 923 |
| dom | 879 |
| standard.arrays | 786 |
| spl | 751 |
| date | 677 |
| standard.strings | 659 |
| soap | 577 |
| phar | 552 |
| reflection | 473 |
| intl | 467 |
| opcache | 449 |
| mysqli | 442 |
| mbstring | 414 |
| sapi | 346 |
| gd | 310 |
| session | 260 |
| streams | 244 |

## Policy

Module prompts may reduce known failures, but must not add new failures or mutate unrelated fingerprints without explanation.
