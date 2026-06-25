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

reference_php="${REFERENCE_PHP:-$php_src/sapi/cli/php}"
target_php="${TARGET_PHP:-$reference_php}"
target_mode="${PHPT_TARGET_MODE:-php-cli}"
default_phpt_tool="${CARGO_TARGET_DIR:-target}/debug/php-phpt-tools"
phpt_tool="${PHPT_TOOLS_BIN:-$default_phpt_tool}"

if [[ ! -x "$target_php" ]]; then
  if [[ -n "${TARGET_PHP:-}" ]]; then
    printf 'Target PHP CLI is not built: %s\n' "$target_php" >&2
    exit 1
  fi
  printf '%s\n' 'Reference PHP CLI is not built; cannot run PHPT runner smoke.' >&2
  printf '%s\n' 'Run: nix develop -c just build-ref-php' >&2
  exit 1
fi

cargo test -p php_phpt_tools
if [[ -z "${PHPT_TOOLS_BIN:-}" && "$phpt_tool" == "$default_phpt_tool" ]]; then
  cargo build -p php_phpt_tools --bin php-phpt-tools
elif [[ ! -x "$phpt_tool" ]]; then
  printf 'PHPT tools executable is not built: %s\n' "$phpt_tool" >&2
  exit 1
fi

"$phpt_tool" run \
  --target "$target_php" \
  --target-mode "$target_mode" \
  --manifest tests/phpt/manifests/runner-smoke.jsonl \
  --out target/phpt-work/module-runs/runner-smoke/results.jsonl \
  --summary target/phpt-work/module-runs/runner-smoke/summary.md \
  --php-src "$php_src"
