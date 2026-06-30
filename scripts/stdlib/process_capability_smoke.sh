#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VM="${PHRUST_STDLIB_VM:-"$ROOT/target/debug/php-vm"}"
FIXTURE="$ROOT/tests/fixtures/stdlib/_harness/composer/process_capability_disabled.php"

cargo build -q -p php_vm_cli --bin php-vm

OUT_DIR="$ROOT/target/stdlib/process-capability"
mkdir -p "$OUT_DIR"

stdout="$OUT_DIR/stdout.txt"
stderr="$OUT_DIR/stderr.txt"
"$VM" run --engine-preset baseline "$FIXTURE" >"$stdout" 2>"$stderr"

expected="$OUT_DIR/expected.txt"
cat >"$expected" <<'EOF'
proc-open-symbol
shell-symbol
shell-disabled
exec-disabled
system-disabled
passthru-disabled
proc-disabled
popen-disabled
EOF

if ! diff -u "$expected" "$stdout"; then
  echo "[fail] process capability smoke output differed" >&2
  exit 1
fi

warning_count="$(grep -c 'E_PHP_VM_PROCESS_CAPABILITY_DISABLED' "$stderr" || true)"
if [[ "$warning_count" -ne 6 ]]; then
  echo "[fail] expected 6 process capability warnings, got $warning_count" >&2
  cat "$stderr" >&2
  exit 1
fi

echo "[ok] process capability smoke passed"
