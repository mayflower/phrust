#!/usr/bin/env bash
set -euo pipefail

out_dir="target/performance/templates"
json_out="${out_dir}/templates.json"
markdown_out="${out_dir}/templates.md"

mkdir -p "${out_dir}"

cargo run -p php_jit --example runtime_templates_report -- --json >"${json_out}"
cargo run -p php_jit --example runtime_templates_report >"${markdown_out}"

python3 - "${json_out}" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
report = json.loads(path.read_text())
if report.get("templates_considered", 0) < 9:
    raise SystemExit("[fail] template report considered too few templates")
if report.get("templates_lowered", 0) < 4:
    raise SystemExit("[fail] template report lowered too few templates")
if report.get("guards_required", 0) <= 0:
    raise SystemExit("[fail] template report missing guards_required")
if report.get("snapshot_slots_required", 0) <= 0:
    raise SystemExit("[fail] template report missing snapshot_slots_required")
rejections = report.get("templates_rejected_by_reason", {})
for reason in [
    "object_or_string_conversion",
    "reference_or_cow_sensitive_array",
    "magic_property_or_hook",
]:
    if reason not in rejections:
        raise SystemExit(f"[fail] template report missing rejection reason {reason}")
PY

if ! grep -q '^# Runtime IR Templates$' "${markdown_out}"; then
    printf '%s\n' '[fail] runtime template markdown missing heading' >&2
    exit 1
fi

printf '[pass] runtime template reports wrote %s and %s\n' "${json_out}" "${markdown_out}"
