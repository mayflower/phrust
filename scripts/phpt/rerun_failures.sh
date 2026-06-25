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
      printf 'unknown phpt-rerun-failures argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$module" ]]; then
  printf '%s\n' 'MODULE is required, for example: just phpt-rerun-failures MODULE=standard.strings' >&2
  exit 2
fi

safe_module="$(printf '%s' "$module" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//')"
work_root="${PHPT_WORK_DIR:-target/phpt-work}"
module_dir="$work_root/module-runs/${safe_module}"
source_results="${PHPT_RESULTS:-$module_dir/target/results.jsonl}"
rerun_dir="$module_dir/rerun-failures"
rerun_manifest="$rerun_dir/manifest.jsonl"

if [[ ! -s "$source_results" ]]; then
  printf 'No previous module results found for rerun: %s\n' "$source_results" >&2
  printf '%s\n' 'Run the module once first, for example: just phpt-fast MODULE=standard.strings' >&2
  exit 1
fi

default_phpt_tool="${CARGO_TARGET_DIR:-target}/debug/php-phpt-tools"
phpt_tool="${PHPT_TOOLS_BIN:-$default_phpt_tool}"
if [[ -n "${PHPT_SKIP_BUILD:-}" ]]; then
  if [[ ! -x "$phpt_tool" ]]; then
    printf 'PHPT tools executable is not built: %s\n' "$phpt_tool" >&2
    printf '%s\n' 'Run: just phpt-dev-build' >&2
    exit 1
  fi
elif [[ -z "${PHPT_TOOLS_BIN:-}" && "$phpt_tool" == "$default_phpt_tool" ]]; then
  cargo build -q -p php_phpt_tools --bin php-phpt-tools
elif [[ ! -x "$phpt_tool" ]]; then
  printf 'PHPT tools executable is not built: %s\n' "$phpt_tool" >&2
  exit 1
fi

"$phpt_tool" rerun-manifest \
  --results "$source_results" \
  --out "$rerun_manifest"

if [[ ! -s "$rerun_manifest" ]]; then
  printf '[ok] no non-green PHPT outcomes to rerun in %s\n' "$source_results"
  exit 0
fi

PHPT_MANIFEST="$rerun_manifest" \
PHPT_RUN_LABEL="${PHPT_RUN_LABEL:-rerun-failures}" \
PHPT_REUSE_LAST="${PHPT_REUSE_LAST:-0}" \
scripts/phpt/module_target.sh --module "$module"
