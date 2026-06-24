# Phase 9: PHPT-driven Runtime Completion

Phase 9 uses the pinned php-src source tree and its PHPT corpus to drive
runtime completion module by module. php-src is the Reference PHP source and
behavior oracle; the Rust engine remains the Target PHP implementation.

Start here:

- [Phase 9 overview](docs/phase9/README.md)
- [Source integrity](docs/phase9/source-integrity.md)
- [Full PHPT gate](docs/phase9/full-phpt-gate.md)

## Module Workflow

Each module batch follows the same loop:

1. Index Original PHPT files and source references from the pinned php-src tree.
2. Select a module batch and generate any Derived PHPT or Minimized PHPT cases.
3. Run the module batch against Reference PHP and Target PHP.
4. Implement missing Rust runtime behavior only for the selected module scope.
5. Re-run the module batch until selected runnable tests are green.
6. Run the Full PHPT gate to reject new regressions outside the module.
7. Verify source integrity so the pinned php-src checkout remains unchanged.

Module green is not the same as final strict green. A module is green when its
selected runnable tests pass and the full-corpus no-regression gate does not
introduce new failures. Final strict green is reserved for the end of Phase 9,
when the complete PHPT corpus passes under the documented strict policy and any
remaining skips or xfails are explicitly justified.
