#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

ENGINE="${CARGO_TARGET_DIR:-target}/debug/php-vm"
OUT_DIR="target/performance/superinstruction-smoke"
mkdir -p "${OUT_DIR}"

if [[ ! -x "${ENGINE}" ]]; then
  printf '[error] missing VM engine: %s\n' "${ENGINE}" >&2
  printf '[hint] run: cargo build -p php_vm_cli\n' >&2
  exit 1
fi

fixtures=(
  "fixtures/runtime/valid/hello.php"
  "fixtures/runtime/valid/scalars/echo.php"
  "fixtures/runtime/valid/scalars/expressions.php"
  "fixtures/runtime/valid/variables/assignment.php"
  "fixtures/runtime/valid/functions/simple.php"
  "fixtures/runtime/valid/functions/two-args.php"
  "fixtures/bytecode/literals/valid/echo-int.php"
  "fixtures/bytecode/literals/valid/echo-multiple.php"
)

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

summary_rows=()
for fixture in "${fixtures[@]}"; do
  stem="${fixture%.php}"
  stem="${stem//\//_}"
  off_stdout="${OUT_DIR}/${stem}.off.stdout"
  off_stderr="${OUT_DIR}/${stem}.off.stderr"
  on_stdout="${OUT_DIR}/${stem}.on.stdout"
  on_stderr="${OUT_DIR}/${stem}.on.stderr"
  on_counters="${OUT_DIR}/${stem}.on.counters.json"

  "${ENGINE}" run --exec-format=bytecode --superinstructions=off "${fixture}" >"${off_stdout}" 2>"${off_stderr}"
  "${ENGINE}" run --exec-format=bytecode --superinstructions=on --counters-json="${on_counters}" "${fixture}" >"${on_stdout}" 2>"${on_stderr}"
  cmp "${off_stdout}" "${on_stdout}"
  cmp "${off_stderr}" "${on_stderr}"
  python3 - "$on_counters" <<'PY'
import json
import sys

path = sys.argv[1]
data = json.loads(open(path, encoding="utf-8").read())
if data.get("bytecode_lower_attempts") != 1:
    raise SystemExit(f"[error] {path}: expected one dense bytecode lowering attempt")
if data.get("bytecode_lower_successes") != 1:
    raise SystemExit(f"[error] {path}: expected one dense bytecode lowering success")
if data.get("bytecode_unsupported_fallbacks") != 0:
    raise SystemExit(f"[error] {path}: expected no dense bytecode fallback")
if data.get("superinstruction_deopt_or_fallbacks") != 0:
    raise SystemExit(f"[error] {path}: expected no superinstruction deopt/fallback")
PY
  summary_rows+=("$(json_escape "${fixture}")")
done

python3 - "${OUT_DIR}"/*.on.counters.json <<'PY'
import json
import sys

total_candidates = 0
total_emitted = 0
total_executed = 0
kinds = set()
for path in sys.argv[1:]:
    data = json.loads(open(path, encoding="utf-8").read())
    total_candidates += data.get("superinstruction_candidates", 0)
    total_emitted += data.get("superinstructions_emitted", 0)
    executed = data.get("superinstructions_executed", {})
    total_executed += sum(executed.values())
    kinds.update(executed)
if total_candidates <= 0:
    raise SystemExit("[error] expected at least one superinstruction candidate")
if total_emitted <= 0:
    raise SystemExit("[error] expected at least one emitted superinstruction")
if total_executed <= 0:
    raise SystemExit("[error] expected at least one executed superinstruction")
required = {"load_const_echo", "load_local_echo", "binary_concat_echo"}
missing = sorted(required - kinds)
if missing:
    raise SystemExit(f"[error] missing executed superinstruction kinds: {', '.join(missing)}")
print(f"{total_candidates} {total_emitted} {total_executed}")
PY

summary="${OUT_DIR}/summary.json"
{
  printf '{\n'
  printf '  "status": "pass",\n'
  printf '  "engine": %s,\n' "$(json_escape "${ENGINE}")"
  printf '  "fixture_count": %s,\n' "${#fixtures[@]}"
  printf '  "fixtures": ['
  for index in "${!summary_rows[@]}"; do
    if [[ "${index}" -gt 0 ]]; then
      printf ', '
    fi
    printf '%s' "${summary_rows[$index]}"
  done
  printf ']\n'
  printf '}\n'
} >"${summary}"

printf '[pass] superinstruction smoke compared %s fixture(s) and wrote %s\n' "${#fixtures[@]}" "${summary}"
