#!/usr/bin/env bash
set -euo pipefail

scripts/phase6_preflight.py --out target/phase6/preflight.json
cargo test -p php_std
cargo test -p php_vm std_builtins
cargo build -q -p php_vm_cli --bin php-vm
scripts/phase6_diff.py --area stdlib --out target/phase6/diff-stdlib-test --vm-binary "${CARGO_TARGET_DIR:-target}/debug/php-vm"

test -s target/phase6/preflight.json
grep -q '"version": "8.5.7"' target/phase6/preflight.json
grep -q '"verify-phase5": true' target/phase6/preflight.json
grep -q 'crates/php_std' target/phase6/preflight.json
grep -q '"docs/known-gaps-phase6.md": true' target/phase6/preflight.json

test -f docs/phase6-preflight.md
test -f docs/phase6-standard-library.md
test -f docs/extension-coverage-phase6.md
test -f docs/function-coverage-phase6.md
test -f docs/composer-compatibility-phase6.md
test -f docs/security-capabilities-phase6.md
test -f docs/known-gaps-phase6.md
test -f docs/phase6-phpt-extension-smoke.md
test -f docs/phase6-regression-corpus.md
test -f docs/phase6-stabilization-06-54.md
test -f docs/phase6-arginfo-coercion.md
test -f docs/phase6-platform-constants.md
test -f docs/phase6-final-audit.md
test -f scripts/phase6/diff_builtin_function.php
test -x scripts/phase6/function_coverage.py
test -x scripts/phase6/generate_arginfo.py
test -f scripts/phase6/list_reference_functions.php
test -f scripts/phase6/list_reference_classes.php
test -f scripts/phase6/list_reference_constants.php
test -x scripts/phase6/normalize_php_output.py
test -x scripts/phase6/composer_source_smoke.sh
test -x scripts/phase6/phpt_extension_selector.py
test -x scripts/phase6_diff.py
test -f fixtures/phase6/phpt_extension_manifest.toml
test -f tests/fixtures/phase6/corpus/known_gaps.toml
test "$(find tests/fixtures/phase6/corpus -maxdepth 1 -name '*.php' | wc -l | tr -d ' ')" -ge 7
test -f tests/fixtures/phase6/_harness/known_gaps.toml
test -f fixtures/phase6/arginfo_overrides.txt
test "$(find tests/fixtures/phase6/_harness/stdlib -name '*.php' | wc -l | tr -d ' ')" -ge 5
test "$(find tests/fixtures/phase6/_harness/streams -name '*.php' | wc -l | tr -d ' ')" -ge 2
test "$(find tests/fixtures/phase6/_harness/json-pcre-date -name '*.php' | wc -l | tr -d ' ')" -ge 3
test "$(find tests/fixtures/phase6/_harness/spl-reflection -name '*.php' | wc -l | tr -d ' ')" -ge 2

grep -q 'PHP 8.5.7' docs/phase6-standard-library.md
grep -q 'php-8.5.7' docs/phase6-standard-library.md
grep -q 'PHAR' docs/phase6-standard-library.md
grep -q 'mbstring' docs/phase6-standard-library.md
grep -q 'intl' docs/phase6-standard-library.md
grep -q 'DOM/XML' docs/phase6-standard-library.md
grep -q 'PDO' docs/phase6-standard-library.md
grep -q 'curl' docs/phase6-standard-library.md
grep -q 'FPM' docs/phase6-standard-library.md
grep -q 'nix develop -c just verify-phase6' docs/phase6-standard-library.md
grep -q 'composer-smoke-source' docs/composer-compatibility-phase6.md
grep -q 'PHASE6_COMPOSER_SOURCE_DIR' docs/composer-compatibility-phase6.md
grep -q 'Phase 6 Function Coverage' docs/function-coverage-phase6.md
grep -q 'coverage-phase6' docs/function-coverage-phase6.md
grep -q 'PHASE6-GAP-STDLIB-FULL-PARITY' docs/known-gaps-phase6.md
grep -q 'phase6-phpt-smoke' docs/phase6-phpt-extension-smoke.md
grep -q 'normalized-report.json' docs/phase6-phpt-extension-smoke.md
grep -q 'phase6-phpt-smoke' docs/extension-coverage-phase6.md
grep -q 'PHASE6-GAP-EXTENSION-PHPT-PROMOTION' docs/known-gaps-phase6.md
grep -q 'phase6-corpus-smoke' docs/phase6-regression-corpus.md
grep -q 'reference-output' docs/phase6-regression-corpus.md
grep -q 'PHASE6_STDLIB_ARRAY_FLIP_WARNING' docs/phase6-stabilization-06-54.md
grep -q 'PHASE6-GAP-ARRAY-WALK-BY-REF-MUTATION' docs/phase6-stabilization-06-54.md
grep -q 'PHASE6_CORPUS_JSON_CONFIG' tests/fixtures/phase6/corpus/json_config.php
grep -q 'purpose:' tests/fixtures/phase6/corpus/reflection_attributes.php
grep -q 'category = "standard"' fixtures/phase6/phpt_extension_manifest.toml
grep -q 'category = "spl"' fixtures/phase6/phpt_extension_manifest.toml
grep -q 'category = "json"' fixtures/phase6/phpt_extension_manifest.toml
grep -q 'category = "pcre"' fixtures/phase6/phpt_extension_manifest.toml
grep -q 'category = "date"' fixtures/phase6/phpt_extension_manifest.toml

for adr in 0060 0061 0062 0063 0064 0065 0066; do
  test -f "docs/adr/${adr}-"*.md
done
grep -q 'ADR-0066' docs/phase6-standard-library.md
grep -q 'ADR-0066' docs/composer-compatibility-phase6.md
grep -q 'PHASE6-GAP-PHAR-REQUIRED' docs/known-gaps-phase6.md

grep -q 'phase6_regression_smoke.sh' docs/phase6-preflight.md
grep -q 'ArgumentValidator' docs/phase6-arginfo-coercion.md
grep -q 'phase6-generate-arginfo' docs/phase6-arginfo-coercion.md
grep -q 'Strict' docs/phase6-arginfo-coercion.md
grep -q 'Weak' docs/phase6-arginfo-coercion.md
grep -q 'PHP_VERSION_ID' docs/phase6-platform-constants.md
grep -q 'DIRECTORY_SEPARATOR' docs/phase6-platform-constants.md
grep -q 'diff-streams' docs/phase6-final-audit.md
grep -q 'diff-json-pcre-date' docs/phase6-final-audit.md
grep -q 'diff-spl-reflection' docs/phase6-final-audit.md
grep -q 'PHASE6-GAP-HASH-RANDOM-ALGORITHMS' docs/known-gaps-phase6.md

scripts/phase6/generate_arginfo.py \
  --php-src tests/fixtures/phase6/arginfo/php-src \
  --overrides fixtures/phase6/arginfo_overrides.txt \
  --out target/phase6/generated/arginfo-smoke.rs
grep -q '@generated by scripts/phase6/generate_arginfo.py' target/phase6/generated/arginfo-smoke.rs
grep -q 'name: "sort"' target/phase6/generated/arginfo-smoke.rs
grep -q 'by_ref: &\["array"\]' target/phase6/generated/arginfo-smoke.rs
grep -q 'variadic: true' target/phase6/generated/arginfo-smoke.rs

printf '%s\n' '[pass] phase6 test gate complete'
