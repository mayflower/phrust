#!/usr/bin/env bash
set -euo pipefail

module="${MODULE:-}"
args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    MODULE=*)
      module="${1#MODULE=}"
      args+=("$1")
      shift
      ;;
    --module)
      module="${2:-}"
      args+=("$1" "$2")
      shift 2
      ;;
    --module=*)
      module="${1#--module=}"
      args+=("$1")
      shift
      ;;
    *)
      args+=("$1")
      shift
      ;;
  esac
done

if [[ -z "$module" ]]; then
  printf '%s\n' 'MODULE is required, for example: just phpt-generate-module MODULE=zend.basic' >&2
  exit 2
fi

safe_module="$(printf '%s' "$module" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//')"

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

timestamp="${PHPT_GENERATED_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
manifest="tests/phpt/manifests/${safe_module}-generated.jsonl"
run_dir="target/phpt-work/module-runs/${safe_module}/reference-generated"

cargo run -q -p php_phpt_tools --bin php-phpt-tools -- generate \
  --module "$module" \
  --php-src "$php_src" \
  --reference "$reference_php" \
  --timestamp "$timestamp" \
  "${args[@]}"

cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target "$reference_php" \
  --target-mode php-cli \
  --manifest "$manifest" \
  --out "$run_dir/results.jsonl" \
  --summary "$run_dir/summary.md" \
  --php-src "$php_src" \
  --work-dir "$run_dir/work" \
  --timeout-seconds "${PHPT_TIMEOUT_SECONDS:-10}"

scripts/phpt/verify_source_integrity.sh

printf '[ok] generated module PHPTs for %s: %s\n' "$module" "$manifest"
