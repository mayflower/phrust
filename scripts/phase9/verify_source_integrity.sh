#!/usr/bin/env bash
set -euo pipefail

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  elif [[ -d third_party/php-src ]]; then
    php_src="third_party/php-src"
  else
    printf '%s\n' 'No pinned php-src checkout found. Set PHP_SRC_DIR or bootstrap the reference checkout.' >&2
    exit 1
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

cargo run -q -p php_phpt_tools --bin php-phpt-tools -- verify-source --php-src "$php_src" --manifest "$manifest"
