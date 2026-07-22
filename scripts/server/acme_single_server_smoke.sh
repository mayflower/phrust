#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo test -p php_server config::tests::tls_mode_is_derived_without_an_extra_mode_flag --lib -- --exact
cargo test -p php_server config::tests::partial_and_mixed_tls_modes_are_rejected --lib -- --exact
cargo test -p php_server acme::tests::low_level_state_does_not_take_listener_ownership --lib -- --exact

feature_tree="$(cargo tree -p php_server -e features)"
if ! grep -q 'rustls-acme feature "ring"' <<<"$feature_tree" ||
   ! grep -q 'rustls-acme feature "webpki-roots"' <<<"$feature_tree"; then
  printf '%s\n' '[fail] rustls-acme does not have exactly the required base features'
  exit 1
fi
if grep -Eq 'rustls-acme feature "(tokio|tower|axum|aws-lc-rs)"' <<<"$feature_tree"; then
  printf '%s\n' '[fail] rustls-acme has a forbidden integration/provider feature'
  exit 1
fi
if cargo tree -p php_server -i aws-lc-rs >/dev/null 2>&1; then
  printf '%s\n' '[fail] aws-lc-rs is present in the php_server dependency graph'
  exit 1
fi

python3 - <<'PY'
from pathlib import Path

root = Path("crates/php_server/src")
production = "\n".join(path.read_text() for path in root.glob("*.rs"))
serve = (root / "serve.rs").read_text()
server = (root / "server.rs").read_text()
acme = (root / "acme.rs").read_text()

assert server.count("TcpListener::bind(") == 1
assert serve.count("listener.accept()") == 1
assert "LazyConfigAcceptor::new" in serve
assert serve.count("manager.run(metrics, shutdown)") == 1
assert acme.count(".state();") == 1
assert acme.count("state.resolver()") == 1
assert acme.count("challenge_rustls_config_with_provider") == 1
for forbidden in (
    ".tokio_incoming(",
    ".incoming(",
    "Arc<Mutex<AcmeState",
    "http01",
    ".well-known/acme-challenge",
    "certbot",
    "Command::new(\"certbot\"",
):
    assert forbidden not in production, forbidden
PY

printf '%s\n' '[ok] ACME single-server source, task, and dependency invariants passed'
