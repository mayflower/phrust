# wp.web-runtime Current Report

Reference target:

- PHP series: 8.5
- PHP version: 8.5.7
- Reference binary:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

## Scope

This report tracks the focused `wp.web-runtime` module harness and the server
transport tests that prove behavior unavailable through the PHP CLI oracle.

## Selected Fixtures

| Fixture | Purpose | Current status |
| --- | --- | --- |
| `tests/phpt/generated/wp.web-runtime/platform-surface.phpt` | CLI-comparable function and superglobal surface | PASS |
| `tests/phpt/generated/wp.web-runtime/transport-web-only.phpt` | Explicit web-transport skip with server-test routing | SKIP: web transport covered by server tests |

Latest module run:

- Command:
  `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.web-runtime`
- Reference outcomes: 1 PASS, 1 SKIP
- Target outcomes: 1 PASS, 1 SKIP
- Source-integrity note: skipped because this checkout has no pinned
  `php-src` checkout under `third_party/`.

## Required Gates

| Gate | Status |
| --- | --- |
| `nix develop -c cargo test -p php_runtime` | PASS |
| `nix develop -c cargo test -p php_vm` | PASS |
| `nix develop -c cargo test -p php_executor` | PASS |
| `nix develop -c cargo test -p php_server` | PASS |
| `nix develop -c cargo test -p php_phpt_tools` | PASS |
| `nix develop -c just server-smoke` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.web-runtime` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=filesystem.streams` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=session` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=phar` | PASS |
| `nix develop -c just verify-server` | PASS |
| `nix develop -c just verify-runtime` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-stdlib` | PASS |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-phpt` | PASS with source-integrity skip |
| `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just composer-smoke` | PASS |
| `nix develop -c just fmt` | PASS |
