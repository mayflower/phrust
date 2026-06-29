#!/usr/bin/env bash
set -euo pipefail

cargo build -p php_vm_cli --bin php-vm

engine="${CARGO_TARGET_DIR:-target}/debug/php-vm"
out_dir="target/performance/dependency-units"
fixture="tests/fixtures/performance/framework_smoke/composer_autoload_lookup.php"
json_out="${out_dir}/dependency-units.json"
markdown_out="${out_dir}/dependency-units.md"

mkdir -p "${out_dir}"

"${engine}" dump-dependency-units "${fixture}" --json >"${json_out}"
"${engine}" dump-dependency-units "${fixture}" >"${markdown_out}"

python3 - "${json_out}" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
report = json.loads(path.read_text())
counters = report.get("counters", {})
if counters.get("dependency_units", 0) <= 0:
    raise SystemExit("[fail] dependency-units JSON missing dependency units")
if counters.get("dependency_edges", 0) <= 0:
    raise SystemExit("[fail] dependency-units JSON missing dependency edges")
if not any(unit.get("kind") == "autoload_resolver" for unit in report.get("units", [])):
    raise SystemExit("[fail] dependency-units JSON missing autoload resolver unit")
PY

if ! grep -q '^# Dependency Units$' "${markdown_out}"; then
    printf '%s\n' '[fail] dependency-units markdown missing heading' >&2
    exit 1
fi

printf '[pass] dependency-unit planner reports wrote %s and %s\n' "${json_out}" "${markdown_out}"
