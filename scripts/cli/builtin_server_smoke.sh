#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

cargo build -p php_vm_cli -p php_server --bins >/dev/null

php="$repo_root/target/debug/phrust-php"
docroot="$(mktemp -d "${TMPDIR:-/tmp}/phrust-cli-server.XXXXXX")"
log="$(mktemp "${TMPDIR:-/tmp}/phrust-cli-server-log.XXXXXX")"
cleanup() {
  if [[ -n "${server_pid:-}" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$docroot"
  rm -f "$log"
}
trap cleanup EXIT

cat >"$docroot/index.php" <<'PHP'
<?php echo "index:", $_SERVER["REQUEST_URI"];
PHP
cat >"$docroot/sapi.php" <<'PHP'
<?php echo PHP_SAPI, "|", php_sapi_name();
PHP
cat >"$docroot/router.php" <<'PHP'
<?php
if ($_SERVER["REQUEST_URI"] === "/router-hit") {
    echo "router-output:", $_SERVER["REQUEST_URI"];
    return true;
}
if ($_SERVER["REQUEST_URI"] === "/router-large") {
    echo str_repeat("r", 1048576);
    return true;
}
if ($_SERVER["REQUEST_URI"] === "/static.txt") {
    echo str_repeat("discarded", 16384);
    return false;
}
return false;
PHP
printf 'static-ok\n' >"$docroot/static.txt"

"$php" -S 127.0.0.1:0 -t "$docroot" >"$log" 2>&1 &
server_pid=$!
url=""
for _ in {1..100}; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$log" >&2 || true
    exit 1
  fi
  url="$(sed -n 's/^listening //p' "$log" | tail -n 1)"
  [[ -n "$url" ]] && break
  sleep 0.05
done
[[ -n "$url" ]]

curl -fsS "$url/" | grep -q '^index:/$'
sapi="$(curl -fsS "$url/sapi.php")"
if [[ "$sapi" != "cli-server|cli-server" ]]; then
  printf '[fail] unexpected built-in SAPI identity: %q\n' "$sapi" >&2
  exit 1
fi
curl -fsS "$url/static.txt" | grep -q '^static-ok$'
status="$(curl -sS -o /dev/null -w '%{http_code}' "$url/missing.php")"
[[ "$status" == "404" ]]

kill "$server_pid" 2>/dev/null || true
wait "$server_pid" 2>/dev/null || true
server_pid=""
: >"$log"

"$php" -S 127.0.0.1:0 -t "$docroot" "$docroot/router.php" >"$log" 2>&1 &
server_pid=$!
url=""
for _ in {1..100}; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    cat "$log" >&2 || true
    exit 1
  fi
  url="$(sed -n 's/^listening //p' "$log" | tail -n 1)"
  [[ -n "$url" ]] && break
  sleep 0.05
done
[[ -n "$url" ]]

curl -fsS "$url/router-hit" | grep -q '^router-output:/router-hit$'
large_bytes="$(curl -fsS "$url/router-large" | wc -c | tr -d ' ')"
[[ "$large_bytes" == "1048576" ]]
curl -fsS "$url/static.txt" | grep -q '^static-ok$'
sapi="$(curl -fsS "$url/sapi.php")"
if [[ "$sapi" != "cli-server|cli-server" ]]; then
  printf '[fail] unexpected routed SAPI identity: %q\n' "$sapi" >&2
  exit 1
fi
curl -fsS "$url/" | grep -q '^index:/$'
if find "${TMPDIR:-/tmp}" -maxdepth 1 -name "phrust-router-${server_pid}-*.spool" -print -quit | grep -q .; then
  printf '%s\n' '[fail] built-in router left an output spool behind' >&2
  exit 1
fi

printf '%s\n' '[ok] phrust-php built-in server smoke passed'
