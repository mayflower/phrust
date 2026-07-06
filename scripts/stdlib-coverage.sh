#!/usr/bin/env bash
set -euo pipefail

scripts/stdlib_preflight.py --out target/stdlib/preflight.json >/dev/null
cargo build -p php_std --bin dump_stdlib_registry >/dev/null
scripts/stdlib/function_coverage.py

test -f docs/stdlib/extension-coverage.md
test -f docs/stdlib/function-coverage.md
test -s target/stdlib/function-coverage/coverage.json
grep -q '| JSON |' docs/stdlib/extension-coverage.md
grep -q '| PCRE |' docs/stdlib/extension-coverage.md
grep -q '| Date/Time |' docs/stdlib/extension-coverage.md
grep -q '| SPL |' docs/stdlib/extension-coverage.md
grep -q '| Reflection |' docs/stdlib/extension-coverage.md
grep -q '| PHAR |' docs/stdlib/extension-coverage.md
grep -q 'Standard Library Function Coverage' docs/stdlib/function-coverage.md
grep -q 'implemented' target/stdlib/function-coverage/coverage.json
grep -q 'known_gap' target/stdlib/function-coverage/coverage.json

printf '%s\n' '[pass] standard library coverage gate complete'
