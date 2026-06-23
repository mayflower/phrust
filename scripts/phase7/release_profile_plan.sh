#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

printf '%s\n' '# Phase 7 Experimental Release Build Profile'
printf '%s\n' ''
printf '%s\n' 'This recipe intentionally does not change the default devshell or Cargo profiles.'
printf '%s\n' 'Use it as a checklist for local before/after measurements only.'
printf '%s\n' ''
printf '%s\n' 'Baseline:'
printf '%s\n' '  nix develop -c just perf-baseline'
printf '%s\n' ''
printf '%s\n' 'LTO candidate:'
printf '%s\n' '  nix develop -c env RUSTFLAGS="-C codegen-units=1 -C lto=fat" cargo build --release -p php_vm_cli --bin php-vm'
printf '%s\n' '  nix develop -c scripts/phase7/bench_matrix.py --engine target/release/php-vm --out target/phase7/release-lto.json --repetitions 5 --warmups 1'
printf '%s\n' ''
printf '%s\n' 'PGO generate candidate:'
printf '%s\n' '  nix develop -c env RUSTFLAGS="-C profile-generate=target/phase7/pgo-data" cargo build --release -p php_vm_cli --bin php-vm'
printf '%s\n' '  nix develop -c scripts/phase7/bench_matrix.py --engine target/release/php-vm --out target/phase7/pgo-training.json --repetitions 3 --warmups 1'
printf '%s\n' ''
printf '%s\n' 'PGO use candidate:'
printf '%s\n' '  nix develop -c llvm-profdata merge -output=target/phase7/pgo.profdata target/phase7/pgo-data'
printf '%s\n' '  nix develop -c env RUSTFLAGS="-C profile-use=target/phase7/pgo.profdata -C llvm-args=-pgo-warn-missing-function" cargo build --release -p php_vm_cli --bin php-vm'
printf '%s\n' '  nix develop -c scripts/phase7/bench_matrix.py --engine target/release/php-vm --out target/phase7/release-pgo.json --repetitions 5 --warmups 1'
printf '%s\n' ''
printf '%s\n' 'Compare:'
printf '%s\n' '  nix develop -c scripts/phase7/compare_perf_json.py target/phase7/baseline.json target/phase7/release-lto.json --out target/phase7/release-lto-compare.md --json-out target/phase7/release-lto-compare.json'
printf '%s\n' ''
printf '%s\n' 'Do not commit target/phase7/pgo-data, *.profraw, *.profdata, or benchmark outputs.'
