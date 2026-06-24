#!/usr/bin/env bash
set -euo pipefail

module="${MODULE:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    MODULE=*)
      module="${1#MODULE=}"
      shift
      ;;
    --module)
      module="${2:-}"
      shift 2
      ;;
    --module=*)
      module="${1#--module=}"
      shift
      ;;
    *)
      printf 'unknown phpt-module-phase9 argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$module" ]]; then
  printf '%s\n' 'MODULE is required, for example: just phpt-module-phase9 MODULE=zend.basic' >&2
  exit 2
fi

safe_module="$(printf '%s' "$module" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//')"
selected_manifest="tests/phpt/manifests/modules/${safe_module}.selected.jsonl"
generated_manifest="tests/phpt/manifests/${safe_module}-generated.jsonl"
manifest="$generated_manifest"

if [[ -s "$selected_manifest" ]]; then
  manifest="$selected_manifest"
fi

if [[ ! -s "$manifest" ]]; then
  scripts/phase9/generate_module.sh "MODULE=$module"
fi

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  else
    php_src="third_party/php-src"
  fi
fi

reference_php="${REFERENCE_PHP:-$php_src/sapi/cli/php}"
if [[ ! -x "$reference_php" ]]; then
  printf 'Reference PHP CLI is not built: %s\n' "$reference_php" >&2
  printf '%s\n' 'Run: nix develop -c just build-ref-php' >&2
  exit 1
fi

target_php="${TARGET_PHP:-target/debug/php-vm}"
target_mode="${PHPT_TARGET_MODE:-}"
if [[ -z "$target_mode" ]]; then
  if [[ "$(basename "$target_php")" == "php-vm" ]]; then
    target_mode="php-vm"
  else
    target_mode="php-cli"
  fi
fi

if [[ ! -x "$target_php" ]]; then
  if [[ -z "${TARGET_PHP:-}" && "$target_php" == "target/debug/php-vm" ]]; then
    cargo build -p php_vm_cli --bin php-vm
  else
    printf 'Target PHP executable is not built: %s\n' "$target_php" >&2
    exit 1
  fi
fi

reference_dir="target/phpt-work/module-runs/${safe_module}/reference"
target_dir="target/phpt-work/module-runs/${safe_module}/target"

cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target "$reference_php" \
  --target-mode php-cli \
  --manifest "$manifest" \
  --out "$reference_dir/results.jsonl" \
  --summary "$reference_dir/summary.md" \
  --php-src "$php_src" \
  --work-dir "$reference_dir/work" \
  --timeout-seconds "${PHPT_TIMEOUT_SECONDS:-10}"

set +e
cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target "$target_php" \
  --target-mode "$target_mode" \
  --manifest "$manifest" \
  --out "$target_dir/results.jsonl" \
  --summary "$target_dir/summary.md" \
  --php-src "$php_src" \
  --work-dir "$target_dir/work" \
  --timeout-seconds "${PHPT_TIMEOUT_SECONDS:-10}"
target_status=$?
set -e

if [[ "$target_status" -gt 1 ]]; then
  printf 'target module run failed before producing a report: status %s\n' "$target_status" >&2
  exit "$target_status"
fi

scripts/phase9/verify_source_integrity.sh

printf '[ok] module PHPT reports for %s\n' "$module"
printf '[ok] reference: %s\n' "$reference_dir"
printf '[ok] target: %s\n' "$target_dir"
