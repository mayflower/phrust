#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT/target/phase6/composer-source-smoke"
mkdir -p "$OUT_DIR"

SOURCE_DIR="${PHASE6_COMPOSER_SOURCE_DIR:-}"
REPORT="$OUT_DIR/report.json"

if [[ -z "$SOURCE_DIR" || ! -d "$SOURCE_DIR" ]]; then
  python3 - "$REPORT" "$SOURCE_DIR" <<'PY'
import json
import sys
from pathlib import Path

report = Path(sys.argv[1])
source = sys.argv[2]
report.write_text(
    json.dumps(
        {
            "status": "skip",
            "source_dir": source,
            "reason": "Set PHASE6_COMPOSER_SOURCE_DIR to a local Composer source checkout",
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY
  echo "[skip] Composer source smoke: set PHASE6_COMPOSER_SOURCE_DIR to a local Composer source checkout"
  exit 0
fi

ENTRY="${PHASE6_COMPOSER_SOURCE_ENTRY:-"$SOURCE_DIR/bin/composer"}"
if [[ ! -f "$ENTRY" ]]; then
  echo "[fail] Composer source entry not found: $ENTRY" >&2
  exit 1
fi
if [[ "$(basename "$ENTRY")" == "composer.phar" ]]; then
  echo "[fail] composer.phar is not a Phase 6 source-mode input" >&2
  exit 1
fi

cargo build -q -p php_vm_cli --bin php-vm
VM="${PHASE6_VM_CLI:-"$ROOT/target/debug/php-vm"}"

STDOUT="$OUT_DIR/stdout.txt"
STDERR="$OUT_DIR/stderr.txt"
set +e
"$VM" run \
  --env "COMPOSER_HOME=$OUT_DIR/home" \
  --env "COMPOSER_CACHE_DIR=$OUT_DIR/cache" \
  "$ENTRY" \
  -- --no-plugins --version >"$STDOUT" 2>"$STDERR"
status=$?
set -e

python3 - "$REPORT" "$SOURCE_DIR" "$ENTRY" "$status" "$STDOUT" "$STDERR" <<'PY'
import json
import re
import sys
from collections import Counter
from pathlib import Path

report, source, entry, status, stdout, stderr = sys.argv[1:]
stderr_text = Path(stderr).read_text(encoding="utf-8", errors="replace")

patterns = [
    ("function", re.compile(r"undefined function ([A-Za-z_][A-Za-z0-9_]*)", re.I)),
    ("class", re.compile(r"class ([A-Za-z_\\\\][A-Za-z0-9_\\\\]*) is not defined", re.I)),
    ("class", re.compile(r"undefined class ([A-Za-z_\\\\][A-Za-z0-9_\\\\]*)", re.I)),
]
counts: Counter[tuple[str, str]] = Counter()
for kind, pattern in patterns:
    for match in pattern.finditer(stderr_text):
        counts[(kind, match.group(1))] += 1

missing = [
    {"kind": kind, "name": name, "count": count}
    for (kind, name), count in sorted(
        counts.items(), key=lambda item: (-item[1], item[0][0], item[0][1].lower())
    )
]
Path(report).write_text(
    json.dumps(
        {
            "status": "pass" if int(status) == 0 else "fail",
            "source_dir": source,
            "entry": entry,
            "exit": int(status),
            "stdout": stdout,
            "stderr": stderr,
            "missing_symbols": missing,
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)

missing_path = Path(stderr).with_name("missing-symbols.txt")
missing_path.write_text(
    "".join(f"{item['count']}\t{item['kind']}\t{item['name']}\n" for item in missing),
    encoding="utf-8",
)
PY

if [[ "$status" -ne 0 ]]; then
  echo "[fail] Composer source smoke failed with exit $status" >&2
  echo "[info] prioritized missing symbols:" >&2
  if [[ -s "$OUT_DIR/missing-symbols.txt" ]]; then
    cat "$OUT_DIR/missing-symbols.txt" >&2
  else
    echo "none extracted; inspect $STDERR" >&2
  fi
  exit "$status"
fi

echo "[ok] Composer source smoke passed"
