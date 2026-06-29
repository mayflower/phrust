# wp.core-builtins Current Report

Reference target:

- PHP series: 8.5
- PHP version: 8.5.7
- Reference binary:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

## Scope

This report tracks the focused `wp.core-builtins` module harness and the
runtime/server gates that cover behavior unavailable through the PHP CLI oracle.

## Selected Fixtures

| Fixture | Purpose | Current status |
| --- | --- | --- |
| `tests/phpt/generated/wp.core-builtins/symbol-extension-introspection.phpt` | Symbol, extension, class-like, and PHP version introspection | pass |
| `tests/phpt/generated/wp.core-builtins/ini-env-config.phpt` | INI, config, environment, SAPI, and memory helpers | pass |
| `tests/phpt/generated/wp.core-builtins/http-response.phpt` | HTTP response callable surface and CLI-comparable initial state | pass |
| `tests/phpt/generated/wp.core-builtins/output-buffering.phpt` | Output buffering capture, clean, and flush helpers | pass |
| `tests/phpt/generated/wp.core-builtins/url-filter-hash-password-serialization.phpt` | URL, random, filter, hash, password, and serialization helpers | pass |
| `tests/phpt/generated/wp.core-builtins/filter-helpers.phpt` | Optional filter helper coverage when reference ext/filter is available | reference skip, target pass |

## Implementation Order

1. Added the `wp.core-builtins` selected manifest and generated fixtures.
2. Registered missing standard metadata for HTTP response, memory, password,
   `parse_str`, `filter_input`, class-like introspection, and `phpversion`
   helpers.
3. Added runtime implementations for `header_remove`, memory helpers, bcrypt
   password helpers, `filter_input`, and `parse_str`.
4. Routed `phpversion` through VM-owned symbol introspection.
5. Added stdlib registry tests for the new functions and constants.

## Required Gates

| Gate | Status |
| --- | --- |
| `nix develop -c cargo test -p php_runtime` | pass: 257 tests, doctests 0 |
| `nix develop -c cargo test -p php_vm` | pass: 450 tests, doctests 0 |
| `nix develop -c cargo test -p php_std` | pass: 60 lib tests, 1 bin test, doctests 0 |
| `nix develop -c cargo test -p php_server` | pass: 38 lib tests, 39 health integration tests, doctests 0 |
| `nix develop -c just server-smoke` | pass: `[ok] phrust-server smoke passed` |
| `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.core-builtins` | pass: reference 5 pass/1 skip, target 6 pass, 0 non-green; source integrity verified 24,475 entries, skipped 0 |
| `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.variables` | pass: reference 0 non-green, target 0 non-green; source integrity verified 24,475 entries, skipped 0 |
| `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings` | pass: reference 0 non-green, target 0 non-green; source integrity verified 24,475 entries, skipped 0 |
| `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.serialization` | pass: 5 PHPTs, reference 0 non-green, target 0 non-green; source integrity verified 24,475 entries, skipped 0 |
| `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=diagnostics.output` | pass: reference 0 non-green, target 0 non-green; source integrity verified 24,475 entries, skipped 0 |
| `nix develop -c just verify-runtime` | pass |
| `nix develop -c just verify-stdlib` | pass |
| `nix develop -c just verify-server` | pass: executor 7 tests, server 38 tests, health integration 39 tests, server smoke passed |
| `nix develop -c just verify-phpt` | pass |

The module PHPT gates used:

- `PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src`
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

## Remaining Gaps

- CLI PHPTs do not assert emitted response headers because PHP CLI behavior
  differs from transport SAPIs; server/runtime gates own response behavior.
- Memory helpers are deterministic runtime counters, not Zend heap accounting.
- Password helpers cover bcrypt only; Argon algorithms remain out of scope.
