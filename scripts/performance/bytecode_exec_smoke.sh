#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

ENGINE="${CARGO_TARGET_DIR:-target}/debug/php-vm"
OUT_DIR="target/performance/bytecode-exec-smoke"
mkdir -p "${OUT_DIR}"

if [[ ! -x "${ENGINE}" ]]; then
  printf '[error] missing VM engine: %s\n' "${ENGINE}" >&2
  printf '[hint] run: cargo build -p php_vm_cli\n' >&2
  exit 1
fi

supported_fixtures=(
  "fixtures/runtime/valid/hello.php"
  "fixtures/runtime/valid/scalars/echo.php"
  "fixtures/runtime/valid/scalars/expressions.php"
  "fixtures/runtime/valid/scalars/comparisons.php"
  "fixtures/runtime/valid/variables/assignment.php"
  "fixtures/runtime/valid/functions/simple.php"
  "fixtures/runtime/valid/functions/two-args.php"
  "fixtures/bytecode/lower/valid/echo.php"
  "fixtures/bytecode/literals/valid/echo-int.php"
  "fixtures/bytecode/literals/valid/echo-multiple.php"
)

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"
}

check_counter() {
  local path="$1"
  local key="$2"
  local expected="$3"
  python3 - "$path" "$key" "$expected" <<'PY'
import json
import sys

path, key, expected = sys.argv[1], sys.argv[2], int(sys.argv[3])
data = json.loads(open(path, encoding="utf-8").read())
actual = data.get(key)
if actual != expected:
    raise SystemExit(f"[error] {path}: expected {key}={expected}, got {actual}")
PY
}

summary_rows=()
for fixture in "${supported_fixtures[@]}"; do
  stem="${fixture%.php}"
  stem="${stem//\//_}"
  ir_stdout="${OUT_DIR}/${stem}.ir.stdout"
  ir_stderr="${OUT_DIR}/${stem}.ir.stderr"
  bc_stdout="${OUT_DIR}/${stem}.bytecode.stdout"
  bc_stderr="${OUT_DIR}/${stem}.bytecode.stderr"
  bc_counters="${OUT_DIR}/${stem}.bytecode.counters.json"

  "${ENGINE}" run --exec-format=ir "${fixture}" >"${ir_stdout}" 2>"${ir_stderr}"
  "${ENGINE}" run --exec-format=bytecode --counters-json="${bc_counters}" "${fixture}" >"${bc_stdout}" 2>"${bc_stderr}"
  cmp "${ir_stdout}" "${bc_stdout}"
  cmp "${ir_stderr}" "${bc_stderr}"
  check_counter "${bc_counters}" bytecode_lower_attempts 1
  check_counter "${bc_counters}" bytecode_lower_successes 1
  check_counter "${bc_counters}" bytecode_unsupported_fallbacks 0
  python3 - "$bc_counters" <<'PY'
import json
import sys

data = json.loads(open(sys.argv[1], encoding="utf-8").read())
if data.get("bytecode_instructions_executed", 0) <= 0:
    raise SystemExit(f"[error] {sys.argv[1]}: expected bytecode instructions to execute")
PY
  summary_rows+=("$(json_escape "${fixture}")")
done

fallback_fixture="fixtures/runtime/valid/scalars/casts.php"
fallback_ir_stdout="${OUT_DIR}/fallback.ir.stdout"
fallback_ir_stderr="${OUT_DIR}/fallback.ir.stderr"
fallback_auto_stdout="${OUT_DIR}/fallback.auto.stdout"
fallback_auto_stderr="${OUT_DIR}/fallback.auto.stderr"
fallback_counters="${OUT_DIR}/fallback.auto.counters.json"
strict_stdout="${OUT_DIR}/fallback.strict.stdout"
strict_stderr="${OUT_DIR}/fallback.strict.stderr"

"${ENGINE}" run --exec-format=ir "${fallback_fixture}" >"${fallback_ir_stdout}" 2>"${fallback_ir_stderr}"
"${ENGINE}" run --exec-format=auto --counters-json="${fallback_counters}" "${fallback_fixture}" >"${fallback_auto_stdout}" 2>"${fallback_auto_stderr}"
cmp "${fallback_ir_stdout}" "${fallback_auto_stdout}"
cmp "${fallback_ir_stderr}" "${fallback_auto_stderr}"
check_counter "${fallback_counters}" bytecode_lower_attempts 1
check_counter "${fallback_counters}" bytecode_lower_successes 0
check_counter "${fallback_counters}" bytecode_unsupported_fallbacks 1
check_counter "${fallback_counters}" bytecode_instructions_executed 0

set +e
"${ENGINE}" run --exec-format=bytecode "${fallback_fixture}" >"${strict_stdout}" 2>"${strict_stderr}"
strict_status=$?
set -e
if [[ "${strict_status}" -ne 4 ]]; then
  printf '[error] strict bytecode unsupported fixture returned %s, expected 4\n' "${strict_status}" >&2
  exit 1
fi
grep -q 'E_PHP_VM_DENSE_BYTECODE_UNSUPPORTED' "${strict_stderr}"

summary="${OUT_DIR}/summary.json"
{
  printf '{\n'
  printf '  "status": "pass",\n'
  printf '  "engine": %s,\n' "$(json_escape "${ENGINE}")"
  printf '  "default_exec_format": "ir",\n'
  printf '  "supported_fixture_count": %s,\n' "${#supported_fixtures[@]}"
  printf '  "supported_fixtures": ['
  for index in "${!summary_rows[@]}"; do
    if [[ "${index}" -gt 0 ]]; then
      printf ', '
    fi
    printf '%s' "${summary_rows[$index]}"
  done
  printf '],\n'
  printf '  "fallback_fixture": %s,\n' "$(json_escape "${fallback_fixture}")"
  printf '  "strict_unsupported_status": %s\n' "${strict_status}"
  printf '}\n'
} >"${summary}"

printf '[pass] bytecode exec smoke compared %s supported fixture(s), verified auto fallback, and wrote %s\n' "${#supported_fixtures[@]}" "${summary}"
