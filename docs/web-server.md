# Run The Web Server

Phrust includes a PHP-compatible built-in server front door:
`phrust-php -S <addr> [-t <docroot>] [router]`. It executes PHP through the
workspace frontend, runtime, and VM. It does not use FPM, FastCGI, CGI, Apache,
`mod_php`, or an external PHP fallback.

## Start The Basic Fixture App

```bash
nix develop -c cargo run -p php_vm_cli --bin phrust-php -- -S 127.0.0.1:8080 -t fixtures/server/apps/basic/public
```

In another shell:

```bash
curl -i http://127.0.0.1:8080/
```

## Use A Config File

The advanced `phrust-server` binary supports server-specific CLI flags and a
simple TOML-style config file:

```bash
nix develop -c cargo run -p php_server --bin phrust-server -- --config path/to/server.toml
```

See [server functionality](server-functionality.md) for config keys,
timeouts, access-log settings, metrics-token handling, cache options, and TLS
options.

HTTP admission and PHP CPU execution are bounded independently.
`max_in_flight` limits accepted requests, while `cpu_execution_limit` (or
`--cpu-execution-limit`) defaults to the host's available parallelism and
limits CPU-bound PHP work. A saturated CPU gate queues for at most
`cpu_queue_timeout_ms`; request admission uses its own
`request_admission_timeout_ms` budget. Both are separate from the total and
idle request-body limits and from the cooperative `max_execution_ms` deadline,
which starts when execution begins. The removed `request_timeout_ms` key is
rejected with a migration error rather than retaining two meanings. Queue and
execution state is request-local and permits are released on cancellation.
The metrics endpoint exposes admitted, queued, current, saturated, rejected,
cancelled, and queue-timeout totals plus the cumulative `cpu_queue` phase.

Response delivery uses one bounded transfer path for HTTP/1.1, HTTP/2, and
optional HTTP/3. Static files are read incrementally, and regular PHP root
output crosses a four-chunk bounded queue in 32 KiB chunks. `flush()` commits
the PHP response head and makes pending root output visible before script
completion. The `max_in_flight` permit remains owned until both PHP cleanup and
response-body completion or abort; access logs and transfer counters therefore
report emitted body frames, not planned `Content-Length` bytes.

Static delivery resolves decoded path segments beneath a document-root
capability and streams the already inspected regular-file handle. The default
public policy hides source, secret, VCS, backup, dotfile, special-file, and
direct sidecar paths. Directory indexes default to `index.php,index.html`;
`--index` accepts an ordered CSV list and `--php-extensions` configures
executable suffixes. Precompressed representations use full q-value/wildcard
negotiation, and validators plus single ranges follow HTTP precondition order.
`--deployment-mode dev` observes live files with `no-cache`; `immutable` builds
a startup asset index and enables strong validators and cacheable release
assets. Dynamic compression, multipart ranges, autoindex, and sendfile remain
out of scope.

Pinned PHP workers reserve a 16 MiB OS-thread stack by default. Set
`PHRUST_SERVER_PHP_WORKER_STACK_BYTES` to a positive byte count when a measured
deployment needs a different bound. Tokio transport workers use Tokio's
default stack; the former `PHRUST_SERVER_TOKIO_WORKER_STACK_BYTES` override no
longer exists.

## Transport Limits And Shutdown

TCP, TLS, and QUIC share `max_connections` admission before expensive
handshake or HTTP work. TLS and QUIC handshakes, HTTP/1 header reads, total and
idle request-body reads, stalled response writes, and inactive keep-alive
connections each have separate deadlines. HTTP/1 uses a strict 64 KiB parser
buffer and keeps Hyper's stack-backed 100-header fast path. HTTP/2 and HTTP/3
share the configured concurrent-stream and decoded-header limits; their
flow-control windows and send buffers are bounded internally. Request targets
default to 16 KiB and decoded header sections to 64 KiB.

`GET /readyz` returns `ready\n` with status 200 while the process accepts work;
`HEAD /readyz` has the same status without a body. SIGINT or SIGTERM changes
readiness to 503 `draining\n`, stops new TCP/QUIC admission, sends HTTP/2 and
HTTP/3 GOAWAY, and lets admitted requests finish until
`graceful_shutdown_timeout_ms`. A second signal or the drain deadline forces
the owned connection/request tasks through their normal cancellation and
tempfile/session cleanup paths. `/healthz` remains the liveness endpoint until
process exit.

The integrated listener deliberately does not interpret `Forwarded`,
`X-Forwarded-*`, or PROXY protocol, and it does not add virtual hosts,
WebSocket/Upgrade, WebTransport, Extended CONNECT, or a separate admin
listener.

## Automatic HTTPS With ACME

`phrust-server` can obtain and renew certificates natively with TLS-ALPN-01.
It remains one server process with one public TCP HTTPS listener: the existing
listener classifies the TLS ClientHello and completes an active ACME challenge
there, without sending that connection to Hyper, PHP, static routing, or the
access log. It does not open port 80, implement HTTP-01, invoke Certbot, or run
a challenge sidecar. Optional HTTP/3 uses the same dynamically updated
certificate over UDP, but ACME validation itself remains TCP.

Configure `acme_domains`, a `mailto:` `acme_contact`, and a persistent
`acme_cache_dir`; do not combine them with `tls_cert`/`tls_key`. Staging is the
default and `acme_directory = "production"` is always explicit. Custom private
or test directories must be HTTPS URLs and may supply
`acme_directory_ca_cert`. The cache directory must be private, writable,
non-symlinked, and separate from request-body, upload, and session directories.
Wildcards and IP literals are rejected.

For public issuance, every configured DNS name must resolve to the active
Phrust instance and TCP 443 must reach that listener unchanged. A
TLS-terminating proxy in front of Phrust prevents TLS-ALPN-01. Operate only one
active instance for a domain set unless external routing guarantees that every
validation reaches the instance holding the current ACME state. Manual PEM TLS
remains the alternative. Local verification uses the test-only Pebble CA:

```bash
nix develop -c just server-acme-single-server-smoke
nix develop -c just server-acme-pebble-integration
```

Prefix request rewrites are a webserver-only routing feature. Configure them
with `--rewrite-prefix-query /api=route` or
`rewrite_prefix_query = "/api=route"` for `phrust-server`, or set
`PHRUST_SERVER_REWRITE_PREFIX_QUERY=/api=route` for the PHP-compatible
`phrust-php -S` entrypoint. Matching requests execute through `/` while
prepending the matched suffix as a query parameter. The PHP engine only sees the
resulting ordinary request URI and query string; it does not know which rewrite
rule, if any, was applied.

## Run Server Checks

```bash
nix develop -c just server-smoke
nix develop -c just cli-server-smoke
nix develop -c just verify-user-interfaces
nix develop -c just server-compat-smoke all
nix develop -c just server-tls-smoke
nix develop -c just server-transport-hardening-smoke
nix develop -c just server-graceful-shutdown-smoke
nix develop -c just server-benchmark-smoke
nix develop -c just verify-server
```

## Inspect Request Performance

Per-request performance tracing is disabled by default. Enable it with
`--perf-trace <path>` or `PHRUST_PERF_TRACE=<path>`. Setting
`PHRUST_PERF_TRACE=1` writes JSONL to
`target/performance/server/perf-trace.jsonl`.

Each JSONL event records route resolution, body read, CPU queue wait, request-context
construction, entry-script cache lookup, VM execution, session seed/finalize,
response build, response bytes, diagnostics count, and cache/source-read deltas.
Failed PHP requests include the last failure phase that was reached.

`/__phrust/metrics` exposes aggregate phase counts/timing plus source-read and
cache-effectiveness counters for the entry script and include cache. It also
reports session seed/lazy-load/ID-generation/finalize/store counters: requests
that never activate a PHP session should increment seed/finalize counters
without incrementing ID-generation or session-store load/write counters. Header materialization counters
show how many incoming headers were seen, carried into the runtime context, or
skipped because an equivalent direct PHP server value already exists. The server
snapshots process environment variables at startup for normal request contexts;
restart the server to expose changed process environment values to PHP requests.
Persistent-engine metrics distinguish immutable metadata reuse from request
state. Script/include cache hits and worker-owned guarded adaptive state may persist
across requests; PHP globals, request context, output buffers, sessions, and
runtime values are reset per request. A request-local reset is therefore counted
as a reset, not as rejected persistence.

For deterministic front-controller request overhead checks, run:

```bash
nix develop -c just front-controller-hotpath-smoke
```

The smoke starts `phrust-server`, warms a local front-controller fixture, asserts
structural cache/phase counters instead of wall-clock thresholds, and writes a
local report under `target/performance/front-controller-hotpath/`.

For an optional local real-WordPress diagnostic report, set
`PHRUST_WORDPRESS_DIR` and optionally `PHRUST_MYSQL_TEST_DSN`, then run:

```bash
nix develop -c just wordpress-real-perf-report
```

Missing WordPress or database prerequisites are reported as skips. Reports land
under `target/wordpress-real/` and treat latency numbers as advisory local
measurements.

For a real WordPress root request-profile JSON plus markdown summary, set
`PHRUST_WORDPRESS_DIR` and run:

```bash
nix develop -c just wordpress-root-profile
```

For the clean root-page benchmark, first build the pinned PHP-FPM 8.5.7 image,
then point the tool at a WordPress tree:

```bash
PHRUST_WORDPRESS_DIR=/path/to/wordpress \
  nix develop -c just wordpress-reference-image
PHRUST_WORDPRESS_DIR=/path/to/wordpress \
  nix develop -c just wordpress-root-benchmark
```

The helper starts telemetry-free `release-lean` Phrust with an immutable
deployment root and stock PHP-FPM with OPcache behind nginx. Reports land under
`target/performance/wordpress-root/` and include p50/p95, throughput, CPU/RSS
where supported, response equivalence, identities, and Phrust/PHP ratios. Use
`just wordpress-root-benchmark-feedback-ab` for persistent-feedback A/B with a
joint off/on ratio report. The main WordPress benchmark uses the mandatory
Cranelift engine. Use `just wordpress-root-diagnostics` for a separate
instrumented Phrust
pass; diagnostic samples are never mixed into clean timing.

See [WordPress smoke workflow](contributor/wordpress-smoke.md) for request-profile
schema 3 and its phase timings plus native compilation, execution, side-exit,
runtime-helper, and version-publication counters.

## Related Docs

- [Server functionality](server-functionality.md)
- [WordPress smoke workflow](contributor/wordpress-smoke.md)
- [PHP user interface matrix](user/php-user-interface-matrix.md)
- [Switching from PHP](user/switching-from-php.md)
- [Server architecture](server-architecture.md)
- [Server known gaps](server-known-gaps.md)
- [Cache architecture](runtime/cache-architecture.md)
- [API facades](api-facades.md)
