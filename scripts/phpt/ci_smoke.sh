#!/usr/bin/env bash
set -euo pipefail

out_dir="target/phpt-work/ci/runner-smoke"
results="$out_dir/results.jsonl"
summary="$out_dir/summary.md"
known="tests/phpt/manifests/runner-smoke-known-non-green.jsonl"
known_keys="$out_dir/known-non-green.keys"
actual_keys="$out_dir/actual-non-green.keys"
unexpected="$out_dir/unexpected-non-green.keys"

mkdir -p "$out_dir"

cargo test -p php_phpt_tools
cargo build -p php_vm_cli --bin php-vm

set +e
cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target target/debug/php-vm \
  --target-mode php-vm \
  --manifest tests/phpt/manifests/runner-smoke.jsonl \
  --out "$results" \
  --summary "$summary" \
  --php-src . \
  --work-dir "$out_dir" \
  --timeout-seconds 10
run_status=$?
set -e

jq -r 'select(.outcome == "FAIL" or .outcome == "BORK") | [.path, .outcome] | @tsv' \
  "$known" | sort > "$known_keys"
jq -r 'select(.outcome == "FAIL" or .outcome == "BORK") | [.path, .outcome] | @tsv' \
  "$results" | sort > "$actual_keys"
comm -23 "$actual_keys" "$known_keys" > "$unexpected"

if [[ -s "$unexpected" ]]; then
  printf '%s\n' '[fail] PHPT CI smoke found unexpected non-green outcomes:' >&2
  cat "$unexpected" >&2
  printf 'summary: %s\n' "$summary" >&2
  exit 1
fi

if [[ "$run_status" -ne 0 ]]; then
  known_count="$(wc -l < "$actual_keys" | tr -d ' ')"
  printf '[ok] accepted %s known PHPT CI smoke non-green outcome(s); reports: %s %s\n' \
    "$known_count" "$results" "$summary"
else
  printf '[ok] PHPT CI smoke passed green; reports: %s %s\n' "$results" "$summary"
fi
