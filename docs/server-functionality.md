# Server Functionality

The integrated web server runs simple PHP applications in-process through the
phrust frontend, runtime, and VM. Hyper/Tokio accepts HTTP requests,
`php_server` routes them, `php_executor` compiles and executes PHP in-process,
and the response is emitted directly by the server.

The server must not use FPM, FastCGI, CGI, Apache module behavior, `mod_php`,
external PHP subprocesses, external PHP worker sockets, or a replacement web
framework stack in the hot path.

## Implemented Surface

The current server surface includes:

- A compatibility fixture app and `server-compat-smoke` harness for incremental
  app-surface checks.
- PHP-compatible URL-encoded input array construction for `$_GET`, `$_POST`,
  and `$_REQUEST`.
- Bounded multipart parsing and populated `$_FILES`.
- Upload builtins: `is_uploaded_file()` and `move_uploaded_file()`.
- Cookie emission through `setcookie()` and `setrawcookie()`.
- Persistent web sessions backed by integrated server storage.
- Output-buffering builtins wired to the existing VM output-buffer stack.
- PHP execution deadlines and `set_time_limit()` integration.
- Include/realpath and compiled-include caching for hot applications.
- Bounded script cache behavior, preload, anti-stampede protection, and safe
  cache invalidation.
- Static file streaming, conditional requests, ranges, and precompressed asset
  selection.
- Production-oriented config, access logs, metrics hardening, Rustls
  HTTP/1.1/HTTP/2 termination, and optional HTTP/3 over QUIC.
- A shared bounded response-transfer core for static files and PHP, including
  transport-visible `flush()`, backpressure, and client cancellation.

- `server-compat-smoke all` is strict for every compatibility section currently
  listed in the harness.
- Remaining gaps are tracked in `docs/server-known-gaps.md`; the current server
  improves standalone operability but does not turn phrust into full PHP
  SAPI compatibility.

## Out Of Scope

The integrated server does not provide FPM, FastCGI, CGI, Apache modules,
`mod_php`, external PHP process execution, Zend ABI emulation, a complete SAPI
compatibility layer, Opcache parity, a full standard library, or a production
process manager.

Known gaps should stay explicit until implemented and verified.

## Compatibility Harness

The compatibility app lives under `fixtures/server/apps/compat/`. The harness
can run named sections:

- `static`
- `input`
- `upload`
- `cookie`
- `session`
- `session-persistence`
- `output-buffer`
- `include`
- `headers`
- `php-input`
- `stream-output`
- `filesystem-cwd`
- `deadline`
- `cache-invalidation`
- `all`

All listed sections are strict. `all` runs the same fixture server once and
executes static serving, nested URL-encoded input, bounded multipart uploads,
upload movement, cookies, persistent sessions, output-buffer basics, include
execution, response headers/status, `php://input`, stream output,
request-local filesystem CWD behavior, a focused execution-deadline timeout
check, and loopback cache invalidation.

`fixtures/integration/plugin_theme_synthetic/` provides a small fixture for
plugin/theme activation smoke checks. It includes a hook-like callback registry,
plugin and theme files, docroot-adjacent filesystem reads/writes,
output-buffered template rendering, headers/cookies, redirects, and optional
multipart package upload handling.

## Persistent Web Sessions

The integrated server owns web session persistence. By default sessions are
enabled with cookie name `PHPSESSID` and cookie path `/`. Session data is held
in a bounded in-memory store owned by the server process, keyed by validated
session id, and serialized with the phrust-owned PHP-serialize-compatible
encoding of the whole `$_SESSION` array. It is intentionally not PHP's
historical `name|serialized-value` session module format.

`--session-save-path` and the `session_save_path` config key are retained as
compatibility knobs for existing launch scripts, but the development server no
longer creates `sess_<id>` files there. Operators can override cookie behavior
with `--session-cookie-name` and `--session-cookie-path`, or disable the feature
with `--disable-sessions`.

Session state is loaded and finalized per request without a global execution
mutex. Requests can run concurrently up to the server in-flight request limit;
concurrent writes for the same session id use last-completing-request-wins
semantics. The store is not cross-process, so sessions do not persist across
server restarts and are not shared between multiple server processes.

## PHP Execution Deadlines

The integrated server configures a cooperative PHP execution deadline with
`--max-execution-ms`, defaulting to `30000`. The deadline is separate from
`--request-timeout-ms`, which only bounds request body reads. When PHP execution
exceeds its request-local deadline, the VM returns the stable diagnostic
`E_PHP_VM_EXECUTION_TIMEOUT` and the server maps it to `504 Gateway Timeout`
with body `php execution timeout`.

`set_time_limit($seconds)` resets the request-local deadline when a mutable
execution deadline is configured. Passing `0` disables the deadline for that
request, matching the supported web-mode behavior. The optional
`--disable-execution-deadline` flag disables server-created deadlines for
development and deterministic tests; metrics expose both timeout totals and
disabled-deadline request counts.

Deadline enforcement is cooperative at native loop headers. It does not use
Tokio task cancellation as the primary safety mechanism, so blocking builtins
are checked when control returns to generated native code.

## Include Cache

The server owns one process-local include cache and passes it into each request
VM through `php_executor`. The cache has two independent shard sets: one for
include path resolution and one for compiled include units. Resolution entries
are keyed by the including directory, requested path, include path entries, cwd,
and allowed-root fingerprint. Compiled include entries are keyed by canonical
path plus opened-source identity, optimization level, compiler/runtime
fingerprint, and local dependency identities discovered at compile time.
Mutable-mode hits validate current primary and dependency bytes before returning
the cached unit. Explicitly immutable deployments use a metadata-only fast path
only while deployment, directory, and file-generation guards remain valid. File
generation or content changes remove stale entries before reuse.

`include_once` and `require_once` tracking stays request-local in VM state; the
shared cache only reuses resolved paths and compiled units. The server exposes
include resolution hits/misses, include compile hits/misses, source reads and
bytes hashed, content validations, identity-only hits, content mismatches,
conservative misses, dependency metadata validations, stale invalidations,
stale dependency invalidations, and include compile errors under
`/__phrust/metrics`.

Web requests allow includes under the public docroot and its parent app root so
compatibility fixtures can keep non-public helpers outside `public/`. Compiled
include artifacts remain in memory only and are never serialized to disk.

## Script Cache Controls

The server owns a bounded process-local compiled script cache for request entry
scripts. It is configured with `--script-cache-shards` and
`--script-cache-max-entries`; entries are distributed across shards and each
shard evicts approximately least-recently-used entries when it exceeds its
share of the configured limit. The cache key includes the canonical path,
source fingerprint, source hash on compile paths, source path, optimization
level, and compiler fingerprint. Cached scripts keep a reusable VM-facing
compiled-unit handle, so request execution does not clone the lowered IR unit.

By default the cache checks file metadata on every lookup so local development
sees edits immediately. A metadata-fresh hit does not reread source; source is
read only for misses, stale metadata, or exact compile-path key construction.
Operators can set
`--script-cache-check-interval-ms <n>` to skip repeated stat checks for hot
entries during that interval. Concurrent requests for the same missing script
share a per-path compile guard so only one request compiles the miss while the
others wait for the populated entry.

`--script-cache-preload <file>` reads a newline-delimited list of absolute
paths or docroot-relative paths at startup and compiles those scripts through
the same cache path as requests. Each listed file is also compiled into the
shared include cache, which allows a trace-generated manifest of include targets
to warm application graphs without executing application code. Blank lines and
`#` comments are ignored. Preload failures warn and continue by default;
`--strict-preload` turns preload read or compile failures into startup failures.

Local cache invalidation is disabled by default. When explicitly enabled with
`--enable-cache-clear-endpoint`, `POST /__phrust/cache/clear` clears process
local entry-script and include caches. In `deployment_mode = "immutable"` it
first builds and atomically publishes a replacement static-asset index. A
failed rebuild leaves the previous index active. The handler still rejects
non-loopback peers. There is no remote or cross-process invalidation protocol.

Metrics expose script cache lookups, hits, misses, source reads, metadata stats,
compiles avoided, entries, entries by shard, stale invalidations, compile
errors, evictions, in-progress compiles, and preload success/failure totals
under `/__phrust/metrics`.

## Static File Responses

The document root is opened once as a filesystem capability. Request paths are
decoded segment by segment, and static resolution opens files only relative to
that capability. The regular-file metadata check and streaming use the same
opened handle; the response path does not canonicalize, stat, and reopen an
ambient pathname. Tokio and Hyper stream that handle without whole-file body
collection. `HEAD` preserves the GET metadata and length without a body.

The public-file policy returns 404 for dotfiles except first-segment
`.well-known`, VCS/secret metadata, backup/editor files, non-configured
PHP-source suffixes, special files, and directly addressed `.br`, `.zst`, or
`.gz` sidecars. `php_extensions` (default `php`) controls executable suffixes.
Directories redirect to a trailing slash with 308 and then try the ordered
`index` list, which defaults to `index.php,index.html`; there is no autoindex.

MIME types come from `mime_guess` with web-specific text, JavaScript, JSON, and
WebAssembly overrides. Static responses include `X-Content-Type-Options:
nosniff`. `Accept-Encoding` evaluates all header instances, q-values, wildcard,
and identity, with equal-quality preference `br > zstd > gzip > identity`.
Sidecars are representations of the identity URL only, and negotiated resources
consistently send `Vary: Accept-Encoding`. No dynamic compression is performed.

Preconditions run in HTTP order: `If-Match`, `If-Unmodified-Since`,
`If-None-Match`, then `If-Modified-Since`. A matching strong `If-Range` enables
a single byte range on the selected representation. Valid ranges return 206;
only a syntactically valid but wholly unsatisfiable range returns 416. Malformed,
unknown-unit, overflowed, and multi-range requests are ignored and return the
full 200 representation. Range on HEAD is ignored.

In the default `dev` deployment mode every static request observes capability
handle metadata, uses a weak ETag, and sends `Cache-Control: no-cache`.
`immutable` builds a capability-relative asset index before readiness, uses one
capability open per indexed static response and strong ETags, sends `no-cache`
for HTML, one-year immutable caching for fingerprinted assets, and one-hour
caching for other assets. This mode declares release files unchanged until
restart or cache clear.

Metrics expose emitted static bytes, validator/range/encoding outcomes,
capability opens, policy denials, mutable resolutions, and immutable-index
build/hit/miss counters under `/__phrust/metrics`.

## Production Server Configuration

The server can read an optional simple TOML-style config file with
`--config <path>`. CLI flags keep their existing names and override values from
the file, so a shared config can define production defaults while deployment
scripts override listen addresses, docroots, or tokens.

Example:

```toml
listen = "127.0.0.1:8080"
docroot = "public"
index = "index.php,index.html"
php_extensions = "php"
deployment_mode = "immutable"
front_controller = "index.php"
max_body_bytes = 1048576
upload_temp_dir = "/var/tmp/phrust-uploads"
session_save_path = "/var/tmp/phrust-sessions"
max_in_flight = 200
cpu_execution_limit = 8
request_timeout_ms = 30000
max_execution_ms = 30000
metrics_endpoint_enabled = true
metrics_token = "replace-with-deployment-secret"
tls_cert = "/etc/phrust/tls/fullchain.pem"
tls_key = "/etc/phrust/tls/privkey.pem"
script_cache_enabled = true
script_cache_shards = 16
script_cache_max_entries = 4096
script_cache_check_interval_ms = 1000
access_log = "/var/log/phrust/access.log"
```

Access logging is disabled by default. `--access-log <path|->` enables one
compact line per request, appending to a file path or writing to stdout when the
target is `-`. Each line records epoch timestamp, method, path/query target,
status, body bytes emitted to the transport, transfer outcome, duration in milliseconds, route
kind (`static`, `php`, `front-controller`, `health`, `metrics`, or rejection
kind), and script-cache hit state when a PHP cache lookup happened.

`GET /__phrust/metrics` remains available by default for local development.
Operators can protect it with `--metrics-token <token>`, which requires
`Authorization: Bearer <token>` or `X-Phrust-Metrics-Token: <token>` on metrics
requests. `--disable-metrics-endpoint` still removes the route entirely.

At startup the first stdout line remains the stable machine-readable
`listening http://<addr>` or `listening https://<addr>` handshake. A separate
stderr summary reports the resolved docroot, front controller, script-cache
settings, upload/session temp directories, metrics exposure, access-log target,
and TLS/ALPN state.

## TLS Transport

`phrust-server` supports first-class Rustls termination with `--tls-cert <path>`
and `--tls-key <path>`, or the equivalent `tls_cert` and `tls_key` config-file
keys. Both files must be PEM encoded and both must be provided together. Invalid
or unreadable certificate/key configuration fails startup with a clear
diagnostic before the server accepts traffic.

TLS wraps the same Hyper service and request handler as plaintext HTTP, so
routing, request body limits, PHP execution, script/include caches, sessions,
access logging, and metrics stay on the same integrated path. Plain HTTP remains
the default for local development when no TLS files are configured.

The TLS transport advertises `h2` and `http/1.1` through ALPN. HTTP/3 is
available over QUIC with `--enable-http3` and `--http3-listen`; all three
protocols consume the same incremental response body. The local TLS smoke uses the committed self-signed
localhost fixture under `fixtures/server/tls/` and `curl -k`:

```bash
nix develop -c just server-tls-smoke
```

## Validation

```bash
nix develop -c cargo fmt --all --check
nix develop -c cargo clippy -p php_server -p php_executor -p php_runtime --all-targets -- -D warnings
nix develop -c cargo test -p php_server
nix develop -c just server-smoke
nix develop -c just server-compat-smoke all
nix develop -c just server-tls-smoke
nix develop -c just server-benchmark-smoke
nix develop -c rg "FastCGI|php-fpm|mod_php|CGI|std::process::Command|Command::new" crates/php_server crates/php_executor docs README.md
```
