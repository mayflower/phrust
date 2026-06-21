#!/usr/bin/env bash
set -euo pipefail

require_file() {
  local path="$1"
  if [[ ! -e "$path" ]]; then
    printf '[error] missing required file: %s\n' "$path" >&2
    exit 1
  fi
  printf '[ok] file exists: %s\n' "$path"
}

require_file crates/php_ast/Cargo.toml
require_file crates/php_ast/src/lib.rs
require_file crates/php_ast/src/ast_node.rs
require_file crates/php_ast/src/ast_token.rs
require_file crates/php_ast/src/support.rs
require_file crates/php_ast/src/names.rs
require_file crates/php_ast/src/types.rs
require_file crates/php_ast/src/expressions.rs
require_file crates/php_ast/src/statements.rs
require_file crates/php_ast/src/declarations.rs
require_file crates/php_ast/src/classes.rs
require_file crates/php_ast/src/attributes.rs
require_file crates/php_ast/src/validation.rs
require_file crates/php_semantics/Cargo.toml
require_file crates/php_semantics/src/lib.rs
require_file crates/php_semantics/src/db.rs
require_file crates/php_semantics/src/hir/mod.rs
require_file crates/php_semantics/src/hir/ids.rs
require_file crates/php_semantics/src/diagnostics/mod.rs
require_file crates/php_semantics/src/diagnostics/ids.rs
require_file crates/php_semantics/src/lower/mod.rs
require_file crates/php_semantics/src/symbols/mod.rs
require_file crates/php_semantics/src/scopes/mod.rs
require_file crates/php_semantics/src/checks/mod.rs
require_file crates/php_frontend_cli/Cargo.toml
require_file crates/php_frontend_cli/src/main.rs
require_file crates/php_testkit/src/semantic_reference.rs
require_file scripts/reference_php_frontend_json.py
require_file scripts/run_semantic_fixtures.py
require_file scripts/compare_semantic_acceptance.py
require_file fixtures/semantic/README.md
require_file fixtures/semantic/valid/minimal.php
require_file fixtures/semantic/valid/hello.php
require_file fixtures/semantic/invalid/README.md
require_file fixtures/semantic/invalid/missing-semicolon.php
require_file docs/phase-3/phase-3-definition-of-done.md
require_file docs/phase-3/semantic-frontend-architecture.md
require_file docs/phase-3/semantic-reference-oracle.md
require_file docs/phase-3/semantic-fixtures.md
require_file docs/adr/0011-typed-ast-views.md
require_file docs/adr/0012-hir-symbol-ids-and-interning.md
require_file docs/adr/0013-php-name-resolution.md
require_file docs/adr/0014-compile-time-diagnostics.md
require_file docs/adr/0015-constant-expression-lowering.md
require_file docs/adr/0016-phase3-no-runtime-boundary.md

if command -v shellcheck >/dev/null 2>&1; then
  shellcheck scripts/verify-phase3.sh scripts/verify_phase3.sh
else
  printf '%s\n' '[skip] shellcheck unavailable in this environment'
fi

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p php_frontend_cli -- --help >/dev/null
cargo run -p php_frontend_cli -- analyze fixtures/semantic/valid/hello.php --format json >/dev/null
just semantic-reference-smoke
just semantic-fixtures
just semantic-diff
just frontend-snapshots

printf '%s\n' '[info] optional soft gates are not part of verify-phase3: semantic-corpus-smoke, fuzz-frontend-smoke, bench-frontend'

printf '%s\n' '[pass] phase3 verification complete'
