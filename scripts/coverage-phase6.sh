#!/usr/bin/env bash
set -euo pipefail

scripts/phase6_preflight.py --out target/phase6/preflight.json >/dev/null
cargo build -p php_std --bin dump_phase6_registry >/dev/null
scripts/phase6/function_coverage.py

test -f docs/extension-coverage-phase6.md
test -f docs/function-coverage-phase6.md
test -s target/phase6/function-coverage/coverage.json
grep -q '| JSON |' docs/extension-coverage-phase6.md
grep -q '| PCRE |' docs/extension-coverage-phase6.md
grep -q '| Date/Time |' docs/extension-coverage-phase6.md
grep -q '| SPL |' docs/extension-coverage-phase6.md
grep -q '| Reflection |' docs/extension-coverage-phase6.md
grep -q '| PHAR |' docs/extension-coverage-phase6.md
grep -q 'Phase 6 Function Coverage' docs/function-coverage-phase6.md
grep -q 'implemented' target/phase6/function-coverage/coverage.json
grep -q 'known_gap' target/phase6/function-coverage/coverage.json

printf '%s\n' '[pass] phase6 coverage gate complete'
