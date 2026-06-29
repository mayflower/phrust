#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

ENGINE="${CARGO_TARGET_DIR:-target}/debug/php-vm"
OUT_DIR="target/performance/bytecode-layout"
FIXTURE="tests/fixtures/performance/perf_smoke/arrays_packed.php"
mkdir -p "${OUT_DIR}"

if [[ ! -x "${ENGINE}" ]]; then
  printf '[error] missing VM engine: %s\n' "${ENGINE}" >&2
  printf '[hint] run: cargo build -p php_vm_cli --bin php-vm\n' >&2
  exit 1
fi

source_stdout="${OUT_DIR}/source.stdout"
source_stderr="${OUT_DIR}/source.stderr"
source_counters="${OUT_DIR}/source.counters.json"
profiled_stdout="${OUT_DIR}/profiled.stdout"
profiled_stderr="${OUT_DIR}/profiled.stderr"
profiled_counters="${OUT_DIR}/profiled.counters.json"
block_frequency="${OUT_DIR}/block-frequency.json"
summary="${OUT_DIR}/summary.json"

"${ENGINE}" run \
  --exec-format=bytecode \
  --bytecode-layout=source \
  --counters-json="${source_counters}" \
  "${FIXTURE}" >"${source_stdout}" 2>"${source_stderr}"

python3 - "${source_counters}" "${block_frequency}" <<'PY'
import json
import sys

source_path, output_path = sys.argv[1], sys.argv[2]
data = json.loads(open(source_path, encoding="utf-8").read())
block_entries = data.get("dense_block_entry_counts", {})
if not block_entries:
    raise SystemExit(f"[error] {source_path}: expected dense block entry counts")
if data.get("dense_branch_executions", 0) <= 0:
    raise SystemExit(f"[error] {source_path}: expected dense branch executions")
report = {
    "schema_version": 1,
    "fixture": "tests/fixtures/performance/perf_smoke/arrays_packed.php",
    "source": "request-local counters",
    "block_entries": block_entries,
    "branch_edges": data.get("dense_branch_edge_counts", {}),
}
with open(output_path, "w", encoding="utf-8") as handle:
    json.dump(report, handle, sort_keys=True, indent=2)
    handle.write("\n")
PY

"${ENGINE}" run \
  --exec-format=bytecode \
  --bytecode-layout=profiled \
  --bytecode-layout-profile="${block_frequency}" \
  --counters-json="${profiled_counters}" \
  "${FIXTURE}" >"${profiled_stdout}" 2>"${profiled_stderr}"

cmp "${source_stdout}" "${profiled_stdout}"
cmp "${source_stderr}" "${profiled_stderr}"

python3 - "${source_counters}" "${profiled_counters}" "${summary}" "${block_frequency}" <<'PY'
import json
import sys

source_path, profiled_path, summary_path, profile_path = sys.argv[1:5]
source = json.loads(open(source_path, encoding="utf-8").read())
profiled = json.loads(open(profiled_path, encoding="utf-8").read())
if profiled.get("dense_block_entries", 0) <= 0:
    raise SystemExit(f"[error] {profiled_path}: expected dense block entries")
if profiled.get("dense_branch_executions", 0) <= 0:
    raise SystemExit(f"[error] {profiled_path}: expected dense branch executions")
summary = {
    "status": "pass",
    "fixture": "tests/fixtures/performance/perf_smoke/arrays_packed.php",
    "source_layout": "source",
    "profiled_layout": "profiled",
    "profile": profile_path,
    "source_dense_block_entries": source.get("dense_block_entries", 0),
    "profiled_dense_block_entries": profiled.get("dense_block_entries", 0),
    "source_dense_branch_executions": source.get("dense_branch_executions", 0),
    "profiled_dense_branch_executions": profiled.get("dense_branch_executions", 0),
    "source_fallthrough_chosen": source.get("dense_branch_fallthrough_chosen", 0),
    "profiled_fallthrough_chosen": profiled.get("dense_branch_fallthrough_chosen", 0),
}
with open(summary_path, "w", encoding="utf-8") as handle:
    json.dump(summary, handle, sort_keys=True, indent=2)
    handle.write("\n")
PY

printf '[pass] bytecode layout smoke compared source/profiled dense bytecode and wrote %s plus %s\n' "${block_frequency}" "${summary}"
