# phpt.foundation

- Priority: 1
- Selected manifest: `tests/phpt/manifests/modules/phpt.foundation.selected.jsonl`
- Current counts: 0 PASS, 0 SKIP, 0 FAIL, 0 BORK from 0 corpus candidates

## Scope

- baseline metadata
- source integrity
- full-regression bookkeeping

## Non-Scope

- runtime semantics
- standard library behavior

## Relevant PHPT Paths

- none identified yet

## Relevant php-src Source Areas

- `scripts/phpt/`
- `crates/php_phpt_tools/`

## Target Gates

- `nix develop -c just verify-phpt`

## Known Gaps

- no known non-green fingerprints assigned in the current baseline

## Next Step

Keep committed baseline, corpus, and source-integrity manifests consistent.
