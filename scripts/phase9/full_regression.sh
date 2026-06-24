#!/usr/bin/env bash
set -euo pipefail

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  else
    php_src="third_party/php-src"
  fi
fi

corpus="${PHPT_CORPUS_MANIFEST:-tests/phpt/manifests/phpt-corpus.jsonl}"
known_failures="${PHPT_KNOWN_FAILURES:-tests/phpt/manifests/full-known-failures.jsonl}"
report="${PHPT_BASELINE_REPORT:-docs/phase9/reports/full-baseline.md}"
work_root="${PHPT_WORK_DIR:-target/phpt-work}"
timestamp="${PHPT_BASELINE_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
run_dir="$work_root/full-runs/$timestamp"

target_php="${TARGET_PHP:-target/debug/php-vm}"
target_mode="${PHPT_TARGET_MODE:-}"
if [[ -z "$target_mode" ]]; then
  if [[ "$(basename "$target_php")" == "php-vm" ]]; then
    target_mode="php-vm"
  else
    target_mode="php-cli"
  fi
fi

if [[ ! -s "$corpus" ]]; then
  printf '%s\n' '[info] PHPT corpus manifest is missing; generating it first.'
  cargo run -q -p php_phpt_tools --bin php-phpt-tools -- phpt-index --php-src "$php_src"
fi

if [[ ! -x "$target_php" ]]; then
  if [[ -z "${TARGET_PHP:-}" && "$target_php" == "target/debug/php-vm" ]]; then
    cargo build -p php_vm_cli --bin php-vm
  else
    printf 'Target PHP executable is not built: %s\n' "$target_php" >&2
    exit 1
  fi
fi

mkdir -p "$run_dir"

previous_args=()
if [[ -s "$known_failures" && "${PHPT_ACCEPT_BASELINE:-0}" != "1" ]]; then
  cp "$known_failures" "$run_dir/previous-known-failures.jsonl"
  previous_args=(--previous-known-failures "$run_dir/previous-known-failures.jsonl")
  previous_results="$(
    find "$work_root/full-runs" -mindepth 2 -maxdepth 2 -name results.jsonl -type f \
      ! -path "$run_dir/results.jsonl" \
      | sort \
      | tail -n 1
  )"
  if [[ -n "$previous_results" ]]; then
    previous_args+=(--previous-results "$previous_results")
    printf 'PHPT_PREVIOUS_RESULTS=%s\n' "$previous_results"
  fi
fi

printf 'TARGET_PHP=%s\n' "$target_php"
printf 'PHPT_TARGET_MODE=%s\n' "$target_mode"
printf 'PHPT_CORPUS_MANIFEST=%s\n' "$corpus"
printf 'PHPT_RUN_DIR=%s\n' "$run_dir"

set +e
cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target "$target_php" \
  --target-mode "$target_mode" \
  --manifest "$corpus" \
  --out "$run_dir/results.jsonl" \
  --summary "$run_dir/summary.md" \
  --php-src "$php_src" \
  --work-dir "$run_dir/work" \
  --timeout-seconds "${PHPT_TIMEOUT_SECONDS:-10}"
run_status=$?
set -e

if [[ "$run_status" -gt 1 ]]; then
  printf 'full PHPT runner failed before producing a comparable result: status %s\n' "$run_status" >&2
  exit "$run_status"
fi

cargo run -q -p php_phpt_tools --bin php-phpt-tools -- baseline \
  --results "$run_dir/results.jsonl" \
  --corpus "$corpus" \
  --known-failures "$known_failures" \
  --report "$report" \
  --timestamp "$timestamp" \
  "${previous_args[@]}"

scripts/phase9/verify_source_integrity.sh

printf '[ok] full PHPT regression baseline artifacts: %s\n' "$run_dir"
printf '[ok] known failures: %s\n' "$known_failures"
printf '[ok] baseline report: %s\n' "$report"
