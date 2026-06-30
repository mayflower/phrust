# PHPT Green 6C Core Runtime, Objects, and Diagnostics Current Report

Date: 2026-06-30

Branch: `phpt-green/core-runtime-objects-diagnostics`

Oracle: `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

PHP source: `/Volumes/CrucialMusic/src/phrust/third_party/php-src`

Note: this checkout does not contain `third_party/php-src/sapi/cli/php`.
The inventory used the existing read-only PHP 8.5.7 oracle checkout above.

## Fresh Inventory Before Edits

Commands used `PHPT_REUSE_LAST=0` and `PHPT_DEV_REUSE_TARGET_PASS=0`.

| Module | Reference PASS | Reference SKIP | Reference FAIL | Target PASS | Target SKIP | Target FAIL | Status |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `objects.classes` | 246 | 0 | 0 | 246 | 0 | 0 | Green |
| `zend.basic` | 10 | 0 | 0 | 10 | 0 | 0 | Green |
| `zend.functions` | 29 | 0 | 0 | 29 | 0 | 0 | Green |
| `standard.variables` | 32 | 1 | 0 | 32 | 1 | 0 | Green |
| `standard.serialization` | 23 | 0 | 0 | 23 | 0 | 0 | Green |
| `diagnostics.output` | 6 | 0 | 0 | 6 | 0 | 0 | Green |
| `closure.core-runtime` | 0 | 0 | 0 | 0 | 0 | 0 | Invalid selector |
| `wp.core-language` | 26 | 0 | 0 | 26 | 0 | 0 | Green |

The prompt's selected object/core/diagnostic modules are already green in the
post-Wave-5C checkout. The only failing inventory command is
`closure.core-runtime`, which is not a module selector in the current manifest
set. The existing dashboard is named `closure.core` and is documented as the
closure core runtime gate.

## Failure Clusters

There are no target failures in the valid selected modules, so there are no top
five failure clusters to rank for the inventory set.

| Cluster | Count | Representative PHPTs | Owner area |
| --- | ---: | --- | --- |
| Invalid module selector | 1 command | `MODULE=closure.core-runtime` | PHPT module selector compatibility |

## Adjacent Green Candidates

Because the selected modules are already green, this campaign should not remove
or skip selected rows. The bounded actionable fix is to make the prompt's
`closure.core-runtime` selector resolve to the existing `closure.core` selected
dashboard, preserving all selected rows and counts.

## After Fix

The PHPT shell runners now normalize `MODULE=closure.core-runtime` to
`MODULE=closure.core` before selected-manifest lookup and work-directory
selection.

| Module command | Reference PASS | Reference SKIP | Reference FAIL | Target PASS | Target SKIP | Target FAIL | Status |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `closure.core-runtime` | 33 | 0 | 0 | 33 | 0 | 0 | Green via `closure.core` alias |

No PHPT rows were promoted in this pass because the selected object/core,
diagnostic, closure, and WordPress dashboards were already green. The change is
a gate-compatibility fix for the campaign prompt's selector spelling.

## Remaining Non-Scope Blockers

No selected failures require broad SPL aggregate internals, full serialization
reference/cycle support, full Zend stack trace byte parity, or broad extension
expansion in the current inventory.

## Validation Snapshot

- `nix develop -c cargo fmt --check`: PASS
- `nix develop -c cargo test -p php_ir`: PASS
- `nix develop -c cargo test -p php_runtime`: PASS
- `nix develop -c cargo test -p php_vm`: PASS
- `nix develop -c cargo test -p php_std`: PASS
- Selected module loop with `PHPT_REUSE_LAST=0` and
  `PHPT_DEV_REUSE_TARGET_PASS=0`: PASS for `objects.classes`, `zend.basic`,
  `zend.functions`, `standard.variables`, `standard.serialization`,
  `diagnostics.output`, `closure.core-runtime`, and `wp.core-language`.
- `REFERENCE_PHP=... PHP_SRC_DIR=... nix develop -c just verify-runtime`:
  PASS.
- `REFERENCE_PHP=... PHP_SRC_DIR=... nix develop -c just verify-stdlib`:
  PASS.
- `REFERENCE_PHP=... PHP_SRC_DIR=... nix develop -c just verify-phpt`: PASS.
- `REFERENCE_PHP=... PHP_SRC_DIR=... PHPT_REUSE_LAST=0 nix develop -c just phpt-module-target MODULE=closure.core-runtime`:
  PASS.
- `REFERENCE_PHP=... PHP_SRC_DIR=... nix develop -c just phpt-rerun-failures MODULE=closure.core-runtime`:
  PASS, no non-green paths to rerun.
- `bash -n scripts/phpt/common.sh scripts/phpt/generate_module.sh scripts/phpt/module_run.sh scripts/phpt/module_target.sh scripts/phpt/rerun_failures.sh`:
  PASS.
