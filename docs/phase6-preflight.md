# Phase 6 Preflight

Phase 6 starts from the accepted Phase 0 through Phase 5 baseline for PHP
8.5.7 (`php-8.5.7`). This preflight records the current repository shape before
standard-library, streams, SPL, Reflection, and Composer compatibility work
begins.

## Report

Generate the machine-readable report with:

```bash
nix develop -c scripts/phase6_preflight.py --out target/phase6/preflight.json
```

The report includes:

- workspace crates from `Cargo.toml`
- available `just` targets
- Phase 0 through Phase 5 verification target presence
- reference PHP discovery status
- existing Phase 4 and Phase 5 known-gap document paths
- Phase 6 documentation and gate presence

`target/phase6/preflight.json` is intentionally not committed. The script emits
an actionable reference-PHP message when neither `REFERENCE_PHP` nor
`third_party/php-src/sapi/cli/php` is available.

## Current Baseline

The committed Phase 5 baseline provides lexer, parser, typed AST, semantic HIR,
IR, VM, runtime values, references/COW, objects, traits, enums, magic methods,
generators, fibers, Reflection MVP, include/require/eval/autoload basics, and a
Phase 5 differential harness.

Phase 6 must preserve the existing `verify-phase0`, `verify-phase1`,
`verify-phase2`, `verify-phase3`, `verify-phase4`, and `verify-phase5` gates.
The Phase 6 gate starts as a documentation and harness gate, then becomes
stricter as required prompts add functionality.

## Regression Smoke

`nix develop -c just verify-phase6` runs `scripts/phase6_regression_smoke.sh`.
This early Phase 6 smoke does not replace the Phase 0 through Phase 5 gates. It
checks that those gates remain discoverable in `just`, that their verification
scripts remain present, and that the Phase 5 handoff and final audit artifacts
are still available. Run the full historical gates directly when changing their
owned layers:

```bash
nix develop -c just verify-phase0
nix develop -c just verify-phase1
nix develop -c just verify-phase2
nix develop -c just verify-phase3
nix develop -c just verify-phase4
nix develop -c just verify-phase5
```

## Reference PHP

Reference behavior is PHP 8.5.7 from the `php-8.5.7` tag. Use one of:

```bash
nix develop -c just build-ref-php
nix develop -c env REFERENCE_PHP=third_party/php-src/sapi/cli/php just verify-phase6
```

Phase 6 differential commands must not silently use a global system PHP. They
must use `REFERENCE_PHP` or the pinned in-repository reference binary.
