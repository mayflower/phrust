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

reference_php="${REFERENCE_PHP:-}"
if [[ -z "$reference_php" ]]; then
  reference_php="$php_src/sapi/cli/php"
fi

if [[ ! -x "$reference_php" ]]; then
  printf '%s\n' 'Reference PHP CLI is not built; cannot run PHPT runner smoke.' >&2
  printf '%s\n' 'Run: nix develop -c just build-ref-php' >&2
  exit 1
fi

cargo test -p php_phpt_tools
cargo run -q -p php_phpt_tools --bin php-phpt-tools -- run \
  --target "$reference_php" \
  --manifest tests/phpt/manifests/runner-smoke.jsonl \
  --out target/phpt-work/module-runs/runner-smoke/results.jsonl \
  --summary target/phpt-work/module-runs/runner-smoke/summary.md \
  --php-src "$php_src"
