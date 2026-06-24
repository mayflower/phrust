#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

require_file() {
  local path="$1"
  if [[ ! -e "${path}" ]]; then
    printf '[fail] missing required file: %s\n' "${path}" >&2
    exit 1
  fi
  printf '[ok] file exists: %s\n' "${path}"
}

find_reference_php() {
  if [[ -n "${REFERENCE_PHP:-}" ]]; then
    printf '%s\n' "${REFERENCE_PHP}"
    return 0
  fi
  if [[ -x third_party/php-src/sapi/cli/php ]]; then
    printf '%s\n' "third_party/php-src/sapi/cli/php"
    return 0
  fi
  if command -v php >/dev/null 2>&1; then
    command -v php
    return 0
  fi
  return 1
}

if [[ -x scripts/verify/foundation.sh ]]; then
  scripts/verify/foundation.sh
fi

required_files=(
  crates/php_source/src/span.rs
  crates/php_source/src/line_index.rs
  crates/php_lexer/Cargo.toml
  crates/php_lexer/src/lib.rs
  crates/php_lexer/src/token.rs
  crates/php_lexer/src/lexer.rs
  crates/php_lexer/src/cursor.rs
  crates/php_lexer/src/modes.rs
  crates/php_lexer/src/diagnostics.rs
  crates/php_lexer_cli/Cargo.toml
  crates/php_lexer_cli/src/main.rs
  crates/php_testkit/src/lexer_reference.rs
  scripts/dump-reference-tokens.php
  scripts/tokenize-reference.php
  scripts/compare-lexer-fixtures.py
  scripts/lexer-corpus-smoke.py
  tests/fixtures/lexer/000-inline-html.php
  tests/fixtures/lexer/010-tags.php
  tests/fixtures/lexer/020-comments-whitespace.php
  docs/lexer/definition-of-done.md
  docs/lexer/lexer-architecture.md
  docs/lexer/token-model.md
  docs/lexer/token-coverage.md
  docs/lexer/fixture-catalog.md
  docs/lexer/diagnostics-policy.md
  docs/lexer/known-lexer-differences.md
)

for path in "${required_files[@]}"; do
  require_file "${path}"
done

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p php_lexer lexer_invariants

if php_bin="$(find_reference_php)"; then
  printf '[info] running lexer fixture reference harness with %s\n' "${php_bin}"
  REFERENCE_PHP="${php_bin}" scripts/compare-lexer-fixtures.py
else
  printf '[skip] no PHP binary found; lexer fixture reference harness skipped\n'
fi

printf '[pass] lexer verification complete\n'
