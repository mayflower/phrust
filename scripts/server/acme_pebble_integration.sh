#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  printf '%s\n' '[skip] Pebble ACME integration is Linux-only.'
  exit 0
fi
for command in cargo curl docker openssl python3; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf '[skip] %s is required for Pebble ACME integration.\n' "$command"
    exit 0
  fi
done
if ! docker info >/dev/null 2>&1; then
  printf '%s\n' '[skip] a running Docker daemon is required for Pebble ACME integration.'
  exit 0
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

pebble_image="${PHRUST_PEBBLE_IMAGE:-ghcr.io/letsencrypt/pebble@sha256:ddf230642b1a584f519f32e347de1b05a6e4c1f6c35c1863b33effeab5f78199}"
work="$(mktemp -d "${TMPDIR:-/tmp}/phrust-acme-pebble.XXXXXX")"
container="phrust-pebble-${RANDOM}-$$"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" >/dev/null 2>&1; then
    kill -TERM "$server_pid" >/dev/null 2>&1 || true
    wait "$server_pid" >/dev/null 2>&1 || true
  fi
  if [[ "${PHRUST_KEEP_ACME_TEST:-0}" == "1" ]]; then
    printf '[keep] ACME test state: %s (container %s)\n' "$work" "$container"
    return
  fi
  docker rm -f "$container" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

cargo build -p php_server --bin phrust-server

read -r tls_port acme_port < <(python3 - <<'PY'
import socket

ports = []
for _ in range(2):
    sock = socket.socket()
    sock.bind(("0.0.0.0", 0))
    ports.append(sock.getsockname()[1])
    sock.close()
print(*ports)
PY
)

extract_id="$(docker create "$pebble_image")"
docker cp "$extract_id:/test/config/pebble-config.json" "$work/pebble-base.json"
docker cp "$extract_id:/test/certs/pebble.minica.pem" "$work/pebble.minica.pem"
docker rm "$extract_id" >/dev/null

python3 - "$work/pebble-base.json" "$work/pebble.json" "$tls_port" <<'PY'
import json
import sys

source, target, tls_port = sys.argv[1:]
with open(source, encoding="utf-8") as handle:
    config = json.load(handle)
pebble = config["pebble"]
pebble["tlsPort"] = int(tls_port)
pebble["retryAfter"] = {"authz": 1, "order": 1}
pebble["profiles"]["default"]["validityPeriod"] = 18
with open(target, "w", encoding="utf-8") as handle:
    json.dump(config, handle)
PY

mkdir -p "$work/docroot" "$work/cache" "$work/body" "$work/uploads" "$work/sessions"
chmod 700 "$work/cache"
printf '%s\n' 'pebble static' > "$work/docroot/static.txt"
printf '%s\n' '<?php echo "pebble php\n";' > "$work/docroot/index.php"
printf '%s\n' '<?php echo "start\n"; flush(); usleep(15000000); echo "done\n";' > "$work/docroot/renewal.php"

directory="https://localhost:${acme_port}/dir"
start_server() {
  local log_file="$1"
  "${CARGO_TARGET_DIR:-target}/debug/phrust-server" \
    --listen "0.0.0.0:${tls_port}" \
    --docroot "$work/docroot" \
    --request-body-temp-dir "$work/body" \
    --upload-temp-dir "$work/uploads" \
    --session-save-path "$work/sessions" \
    --acme-domains phrust.test \
    --acme-contact mailto:acme-test@example.invalid \
    --acme-cache-dir "$work/cache" \
    --acme-directory "$directory" \
    --acme-directory-ca-cert "$work/pebble.minica.pem" \
    >"$log_file" 2>&1 &
  server_pid="$!"
  for _ in {1..200}; do
    if grep -q '^listening https://' "$log_file"; then
      return 0
    fi
    if ! kill -0 "$server_pid" >/dev/null 2>&1; then
      printf '%s\n' '[fail] phrust-server exited before listening'
      cat "$log_file"
      return 1
    fi
    sleep 0.05
  done
  printf '%s\n' '[fail] phrust-server did not become listenable'
  cat "$log_file"
  return 1
}

stop_server() {
  kill -TERM "$server_pid"
  wait "$server_pid"
  server_pid=""
}

pre_pebble_log="$work/server-pre-pebble.log"
start_server "$pre_pebble_log"

pre_certificate="$(timeout 3 openssl s_client -connect "127.0.0.1:${tls_port}" -servername phrust.test </dev/null 2>&1 || true)"
if grep -q -- '-----BEGIN CERTIFICATE-----' <<<"$pre_certificate"; then
  printf '%s\n' '[fail] empty ACME cache unexpectedly served a normal certificate'
  exit 1
fi
for _ in {1..200}; do
  if grep -q 'transient ACME order error' "$pre_pebble_log"; then
    break
  fi
  sleep 0.05
done
if ! grep -q 'transient ACME order error' "$pre_pebble_log"; then
  printf '%s\n' '[fail] an unreachable ACME directory was not surfaced as a transient error'
  exit 1
fi
stop_server

server_log="$work/server-initial.log"
start_server "$server_log"

docker run -d --name "$container" \
  --add-host phrust.test:host-gateway \
  -p "127.0.0.1:${acme_port}:14000" \
  -e PEBBLE_VA_NOSLEEP=1 \
  -e PEBBLE_WFE_NONCEREJECT=0 \
  -e PEBBLE_AUTHZREUSE=100 \
  -v "$work/pebble.json:/test/config/phrust-pebble.json:ro" \
  "$pebble_image" -config /test/config/phrust-pebble.json -strict=false >/dev/null

for _ in {1..200}; do
  if curl --noproxy '*' --cacert "$work/pebble.minica.pem" -fsS "$directory" >/dev/null 2>&1; then
    break
  fi
  sleep 0.05
done
if ! curl --noproxy '*' --cacert "$work/pebble.minica.pem" -fsS "$directory" >/dev/null; then
  printf '%s\n' '[fail] Pebble directory did not start'
  docker logs "$container"
  exit 1
fi

base_url="https://phrust.test:${tls_port}"
curl_args=(--noproxy '*' --resolve "phrust.test:${tls_port}:127.0.0.1" -k -fsS)
for _ in {1..600}; do
  if curl "${curl_args[@]}" "$base_url/readyz" 2>/dev/null | grep -qx ready; then
    break
  fi
  if ! kill -0 "$server_pid" >/dev/null 2>&1; then
    printf '%s\n' '[fail] phrust-server exited during ACME issuance'
    cat "$server_log"
    docker logs "$container"
    exit 1
  fi
  sleep 0.1
done
if ! curl "${curl_args[@]}" "$base_url/readyz" | grep -qx ready; then
  printf '%s\n' '[fail] ACME certificate was not issued'
  cat "$server_log"
  docker logs "$container"
  exit 1
fi

[[ "$(curl "${curl_args[@]}" --http1.1 "$base_url/static.txt")" == 'pebble static' ]]
[[ "$(curl "${curl_args[@]}" --http1.1 "$base_url/index.php")" == 'pebble php' ]]
if curl -V | grep -q 'Features:.*HTTP2'; then
  [[ "$(curl "${curl_args[@]}" --http2 "$base_url/static.txt")" == 'pebble static' ]]
fi

certificate_text="$(openssl s_client -connect "127.0.0.1:${tls_port}" -servername phrust.test </dev/null 2>/dev/null | openssl x509 -noout -text)"
if ! grep -q 'DNS:phrust.test' <<<"$certificate_text"; then
  printf '%s\n' '[fail] issued certificate does not contain phrust.test SAN'
  exit 1
fi

metrics="$(curl "${curl_args[@]}" "$base_url/__phrust/metrics")"
assert_metric() {
  local name="$1" expected="$2"
  if ! grep -q "^${name} ${expected}$" <<<"$metrics"; then
    printf '[fail] metric %s did not equal %s\n' "$name" "$expected"
    exit 1
  fi
}
assert_metric phrust_server_acme_enabled 1
assert_metric phrust_server_acme_certificate_available 1
assert_metric phrust_server_acme_manager_running 1
assert_metric phrust_server_acme_readiness 1
assert_metric phrust_server_tcp_listener_binds_total 1
assert_metric phrust_server_http_listener_count 1
assert_metric phrust_server_acme_listener_binds_total 0
assert_metric phrust_server_port_80_binds_total 0
assert_metric phrust_server_acme_state_tasks_started_total 1
assert_metric phrust_server_acme_state_tasks_active 1
assert_metric phrust_server_acme_challenge_http_requests_total 0
assert_metric phrust_server_acme_challenge_hyper_requests_total 0
assert_metric phrust_server_quic_endpoint_count 0
if ! grep -Eq '^phrust_server_acme_challenge_handshakes_completed_total [1-9][0-9]*$' <<<"$metrics"; then
  printf '%s\n' '[fail] Pebble did not complete TLS-ALPN-01 on the normal listener'
  exit 1
fi

if [[ "$(find "$work/cache" -maxdepth 1 -type f | wc -l)" -lt 2 ]]; then
  printf '%s\n' '[fail] ACME account and certificate were not both cached'
  exit 1
fi
if find "$work/cache" -maxdepth 1 -type f ! -perm 0600 | grep -q .; then
  printf '%s\n' '[fail] ACME cache files are not mode 0600'
  find "$work/cache" -maxdepth 1 -type f -printf '%m %p\n'
  exit 1
fi

certificate_fingerprint() {
  openssl s_client -connect "127.0.0.1:${tls_port}" -servername phrust.test </dev/null 2>/dev/null |
    openssl x509 -outform DER 2>/dev/null |
    sha256sum | cut -d' ' -f1
}
first_fingerprint="$(certificate_fingerprint)"
curl "${curl_args[@]}" --http1.1 "$base_url/renewal.php" > "$work/renewal-body.txt" &
renewal_request_pid="$!"
changed_fingerprint=""
for _ in {1..160}; do
  candidate_fingerprint="$(certificate_fingerprint || true)"
  if [[ -n "$candidate_fingerprint" && "$candidate_fingerprint" != "$first_fingerprint" ]]; then
    changed_fingerprint="$candidate_fingerprint"
    break
  fi
  sleep 0.25
done
if [[ -z "$changed_fingerprint" ]]; then
  printf '%s\n' '[fail] ACME renewal did not update the resolver for new connections'
  exit 1
fi
wait "$renewal_request_pid"
if [[ "$(cat "$work/renewal-body.txt")" != $'start\ndone' ]]; then
  printf '%s\n' '[fail] connection active across resolver renewal did not complete'
  exit 1
fi

metrics="$(curl "${curl_args[@]}" "$base_url/__phrust/metrics")"
if ! grep -Eq '^phrust_server_acme_certificates_issued_total ([2-9]|[1-9][0-9]+)$' <<<"$metrics"; then
  printf '%s\n' '[fail] short-lived Pebble certificate did not renew'
  exit 1
fi

timeout 3 openssl s_client -connect "127.0.0.1:${tls_port}" -servername unknown.phrust.test -alpn acme-tls/1 </dev/null >/dev/null 2>&1 || true
timeout 3 openssl s_client -connect "127.0.0.1:${tls_port}" -alpn acme-tls/1 </dev/null >/dev/null 2>&1 || true
timeout 3 openssl s_client -connect "127.0.0.1:${tls_port}" -servername phrust.test -alpn acme-tls/1 </dev/null >/dev/null 2>&1 || true
metrics="$(curl "${curl_args[@]}" "$base_url/__phrust/metrics")"
if ! grep -Eq '^phrust_server_acme_challenge_unknown_sni_total ([2-9]|[1-9][0-9]+)$' <<<"$metrics"; then
  printf '%s\n' '[fail] unknown or missing TLS-ALPN-01 SNI was not rejected and counted'
  exit 1
fi
if ! grep -Eq '^phrust_server_acme_challenge_handshake_failures_total ([3-9]|[1-9][0-9]+)$' <<<"$metrics"; then
  printf '%s\n' '[fail] TLS-ALPN-01 without an active order did not fail and get counted'
  exit 1
fi

account_creations_before_restart="$(docker logs "$container" 2>&1 | grep -c 'There are now .* accounts in memory' || true)"

stop_server
restart_log="$work/server-restart.log"
start_server "$restart_log"
for _ in {1..100}; do
  if curl "${curl_args[@]}" "$base_url/readyz" 2>/dev/null | grep -qx ready; then
    break
  fi
  sleep 0.05
done
metrics="$(curl "${curl_args[@]}" "$base_url/__phrust/metrics")"
if ! grep -q '^phrust_server_acme_cache_loads_total 1$' <<<"$metrics"; then
  printf '%s\n' '[fail] restart did not deploy the cached certificate'
  exit 1
fi
account_creations_after_restart="$(docker logs "$container" 2>&1 | grep -c 'There are now .* accounts in memory' || true)"
if [[ "$account_creations_before_restart" != "1" || "$account_creations_after_restart" != "$account_creations_before_restart" ]]; then
  printf '%s\n' '[fail] restart did not reuse the cached ACME account'
  exit 1
fi
if grep -q 'acme-test@example.invalid\|phrust.test' "$restart_log"; then
  printf '%s\n' '[fail] ACME startup logs exposed a contact or domain value'
  exit 1
fi

stop_server
cert_cache="$(find "$work/cache" -maxdepth 1 -type f -name 'cached_cert_*' -print -quit)"
printf '%s\n' 'corrupt-cache' > "$cert_cache"
corrupt_log="$work/server-corrupt.log"
start_server "$corrupt_log"
for _ in {1..200}; do
  if ! kill -0 "$server_pid" >/dev/null 2>&1; then
    wait "$server_pid" || true
    server_pid=""
    break
  fi
  sleep 0.05
done
if [[ -n "$server_pid" ]]; then
  printf '%s\n' '[fail] corrupt certificate cache did not trigger operational shutdown'
  exit 1
fi
if ! grep -q 'terminal ACME cache error' "$corrupt_log"; then
  printf '%s\n' '[fail] corrupt certificate cache was not diagnosed'
  exit 1
fi

printf '%s\n' '[ok] Pebble issuance, same-listener TLS-ALPN-01, renewal, cache restart, and failure integration passed'
