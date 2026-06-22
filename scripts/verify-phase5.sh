#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' '[info] Phase 5 verification starts from the Phase 4 runtime baseline.'

just fmt
just lint
just runtime-hardening-lints
just phase5-toolchain-audit
just test
just verify-phase4
just phase5-fixtures
just phase5-diff
test -f docs/phase5-final-audit.md
test -f docs/phase5-coverage-matrix.md
test -f docs/phase6-handoff.md
grep -q 'Phase 5 Coverage Matrix' docs/phase5-coverage-matrix.md
grep -q 'Unsupported ID Cleanup' docs/phase5-coverage-matrix.md
grep -q 'Phase 6 Handoff' docs/phase6-handoff.md
grep -q 'Standard library' docs/phase6-handoff.md
grep -q 'SPL and Reflection expansion' docs/phase6-handoff.md
grep -q 'Bytecode cache' docs/phase6-handoff.md
grep -q 'Extension API' docs/phase6-handoff.md
grep -q 'Phase 5 Final Audit' docs/phase5-final-audit.md
test -f fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "references_cow"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "foreach"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "traits"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "enums"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "generators"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "fibers"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "property_hooks"' fixtures/phase5/phpt_allowlist.toml
grep -q 'category = "reflection"' fixtures/phase5/phpt_allowlist.toml
test -f fixtures/phase5/real_world/framework-style-direct-service.php
test -f fixtures/phase5/real_world/composer-style-autoload-service.php
test -f fixtures/phase5/real_world/framework-container-reflection-known-gap.php
test -x scripts/minimize_phase5_failure.py
test -f fixtures/phase5/regressions/pass/array-element-reference-cow.php
test -f fixtures/phase5/regressions/pass/fiber-suspend-stdout.php
test -f fixtures/phase5/regressions/known_gaps/object-property-reference.php
grep -q 'regression_category=' fixtures/phase5/regressions/pass/array-element-reference-cow.php
grep -q 'reference_behavior=' fixtures/phase5/regressions/pass/array-element-reference-cow.php
grep -q 'fix_prompt=' fixtures/phase5/regressions/pass/array-element-reference-cow.php
just phase5-phpt-smoke

printf '%s\n' '[pass] phase5 verification complete'
