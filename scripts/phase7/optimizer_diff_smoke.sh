#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
engine="${CARGO_TARGET_DIR:-target}/debug/php-vm"

cargo build -p php_vm_cli --bin php-vm
cargo test -p php_optimizer

"$root/scripts/phase7/optimizer_diff.py" --self-test
"$root/scripts/phase7/optimizer_diff.py" --engine "$engine"
