#!/usr/bin/env bash
set -euo pipefail

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  elif [[ -d third_party/php-src ]]; then
    php_src="third_party/php-src"
  else
    printf '%s\n' '[skip] no pinned php-src checkout found; set PHP_SRC_DIR or run `just bootstrap-ref` for source-integrity verification.'
    exit 0
  fi
fi

if [[ ! -d "$php_src" ]]; then
  printf 'PHP_SRC_DIR is not a directory: %s\n' "$php_src" >&2
  exit 1
fi

if [[ -d "$php_src/.git" ]]; then
  status="$(git -C "$php_src" status --short)"
  if [[ -n "$status" ]]; then
    printf 'Pinned php-src checkout has local modifications:\n%s\n' "$status" >&2
    exit 1
  fi
fi

manifest="tests/phpt/manifests/php-src-hashes.jsonl"
if [[ ! -f "$manifest" ]]; then
  printf '[ok] php-src checkout clean at %s; hash manifest pending 09.01.\n' "$php_src"
  exit 0
fi

default_phpt_tool="${CARGO_TARGET_DIR:-target}/debug/php-phpt-tools"
phpt_tool="${PHPT_TOOLS_BIN:-$default_phpt_tool}"
if [[ -z "${PHPT_TOOLS_BIN:-}" && "$phpt_tool" == "$default_phpt_tool" ]]; then
  cargo build -q -p php_phpt_tools --bin php-phpt-tools
elif [[ ! -x "$phpt_tool" ]]; then
  printf 'PHPT tools executable is not built: %s\n' "$phpt_tool" >&2
  exit 1
fi

"$phpt_tool" verify-source --php-src "$php_src" --manifest "$manifest"
