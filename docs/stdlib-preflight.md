# Standard library Preflight

Standard library starts from the accepted Foundation through Runtime semantics baseline for PHP
8.5.7 (`php-8.5.7`). This preflight records the current repository shape before
standard-library, streams, SPL, Reflection, and Composer compatibility work
begins.

## Report

Generate the machine-readable report with:

```bash
nix develop -c scripts/stdlib_preflight.py --out target/stdlib/preflight.json
```

The report includes:

- workspace crates from `Cargo.toml`
- available `just` targets
- required verification target presence
- reference PHP discovery status
- existing runtime known-gap document paths
- standard-library documentation and gate presence

`target/stdlib/preflight.json` is intentionally not committed. The script emits
an actionable reference-PHP message when neither `REFERENCE_PHP` nor
`third_party/php-src/sapi/cli/php` is available.

## Current Baseline

The committed Runtime semantics baseline provides lexer, parser, typed AST, semantic HIR,
IR, VM, runtime values, references/COW, objects, traits, enums, magic methods,
generators, fibers, Reflection MVP, include/require/eval/autoload basics, and a
Runtime semantics differential harness.

The standard-library gate must preserve the foundation, lexer, frontend, and
runtime verification layers. It starts as a documentation and harness gate, then
becomes stricter as required functionality is promoted.

## Regression Smoke

`nix develop -c just verify-performance` runs
`scripts/performance_regression_smoke.sh`. This smoke does not replace the
foundation, lexer, frontend, runtime, or standard-library gates. It checks that
those gates remain discoverable in `just`, that their verification scripts
remain present, and that the handoff and final audit artifacts are still
available. Run the owned gates directly when changing those layers:

```bash
nix develop -c just verify-foundation
nix develop -c just verify-lexer
nix develop -c just verify-frontend
nix develop -c just verify-runtime
nix develop -c just verify-stdlib
```

## Reference PHP

Reference behavior is PHP 8.5.7 from the `php-8.5.7` tag. Use one of:

```bash
nix develop -c just build-ref-php
nix develop -c env REFERENCE_PHP=third_party/php-src/sapi/cli/php just verify-stdlib
```

Standard library differential commands must not silently use a global system PHP. They
must use `REFERENCE_PHP` or the pinned in-repository reference binary.
