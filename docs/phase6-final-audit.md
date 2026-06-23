# Phase 6 Final Audit

Reference target: PHP 8.5.7 (`php-8.5.7`).

This audit is the Prompt 06.56 handoff artifact. It records the required
Phase 6 gates, the optional coverage that was implemented in-tree, and the
remaining known gaps carried as Phase 7 input.

## Required Gates

Run these before closing Phase 6:

```bash
nix develop -c just verify-phase6
nix develop -c just test-phase6
nix develop -c just diff-stdlib
nix develop -c just diff-streams
nix develop -c just diff-json-pcre-date
nix develop -c just diff-spl-reflection
nix develop -c just composer-smoke
nix develop -c just coverage-phase6
```

The stream, JSON/PCRE/Date, and SPL/Reflection gates are real
`scripts/phase6_diff.py` differential runs over dedicated fixture areas. They
must not be replaced by placeholder skip scripts.

## Fixture Coverage

- `tests/fixtures/phase6/_harness/stdlib`: broad standard-library MVP
  differential fixtures, including optional `hash`, `hash_hmac`,
  `random_bytes`, and `random_int` shape/range coverage.
- `tests/fixtures/phase6/_harness/streams`: resource, `php://memory`, and
  local filesystem path smoke fixtures.
- `tests/fixtures/phase6/_harness/json-pcre-date`: JSON, PCRE, and Date/Time
  extension smoke fixtures.
- `tests/fixtures/phase6/_harness/spl-reflection`: SPL iterator/container and
  Reflection smoke fixtures.
- `tests/fixtures/phase6/corpus`: Composer/framework-style regression snippets
  for autoload, environment, JSON config, routing, DateTime/version parsing,
  arrays, and reflection attributes.

## Optional Coverage

- Prompt 06.A: reference metadata extraction exists through
  `scripts/phase6/list_reference_functions.php`,
  `scripts/phase6/list_reference_classes.php`,
  `scripts/phase6/list_reference_constants.php`, and
  `scripts/phase6/function_coverage.py`. Optional arginfo generation is
  available through `just phase6-generate-arginfo`; `test-phase6` runs the
  generator against a local php-src-style fixture with manual overrides.
- Prompt 06.B: PHAR remains governed by ADR-0066. Composer source mode is the
  required path; read-only PHAR support is not enabled in Phase 6.
- Prompt 06.C: tokenizer extension metadata and runtime smoke coverage are
  included in `test-phase6`.
- Prompt 06.D: online Composer and Packagist access are default-off; local
  source-mode Composer smoke is available through `composer-smoke-source`.
- Prompt 06.E: hash/random MVP is implemented and covered by
  `PHASE6_STDLIB_HASH_RANDOM`.
- Prompt 06.F: larger Composer source checkouts are opt-in through
  `PHASE6_COMPOSER_SOURCE_DIR` and skip explicitly when absent.

## Phase 7 Handoff

Phase 7 should consume `docs/known-gaps-phase6.md`,
`docs/function-coverage-phase6.md`, and `docs/extension-coverage-phase6.md` as
the authoritative gap map. Highest-priority carryovers are full arginfo
generation/import, byte-perfect extension diagnostics, full Date/Time timelib
parity, complete hash algorithm coverage, PHAR only if ADR-0066 is superseded,
and broader upstream PHPT promotion.
