# Real WordPress Smoke

This gate answers whether a local WordPress checkout installs and serves through
phrust. It complements the reduced `wordpress-blockers`, `wp-language-vm`, and
`wp-autoload-stdlib` gates. Those reduced gates stay useful for focused
regressions; this smoke records the first real bootstrap failure without hiding
environment problems as runtime bugs.

## Prerequisites

Set the WordPress checkout path:

```bash
export PHRUST_WORDPRESS_DIR=/path/to/wordpress
```

Optional PHP oracle comparison uses:

```bash
export REFERENCE_PHP=/path/to/php-8.5.7/sapi/cli/php
```

DB-backed phases also need the local MariaDB DSN used by the runtime:

```bash
export PHRUST_MYSQL_TEST_DSN='mysql://wordpress:secret@127.0.0.1:3306/wordpress'
```

The scripts do not download WordPress, start Docker, or require public
internet access. Missing local prerequisites are reported as environment
blockers.

## Preflight

Run:

```bash
nix develop -c just wordpress-preflight
```

The preflight writes JSON to `target/wordpress-real/preflight.json` and prints
the same payload. It classifies blockers such as
`missing_wordpress_checkout`, `missing_reference_php`,
`missing_php_vm_binary_or_stale_binary`, `docker_unavailable`,
`mariadb_unavailable`, `mariadb_credentials_invalid`,
`document_root_invalid`, `phrust_server_unavailable`, and
`port_unavailable`.

## No-DB Frontpage Smoke

Run:

```bash
nix develop -c just wordpress-real-smoke
```

This builds the local phrust binaries and requests `/` through
`phrust-server`. If the local WordPress checkout needs a database before it can
render the front controller, the failure is recorded as a real product or
configuration failure in the generated report.

## DB-Backed Install Smoke

Run with Docker MariaDB already available and `PHRUST_MYSQL_TEST_DSN` set:

```bash
nix develop -c just wordpress-real-install-smoke
```

The DB-backed target runs the install submission, admin login page, and
post-install frontpage phases. If `PHRUST_MYSQL_TEST_DSN` is missing, the run
skips with `environment_blocker=missing_mysql_dsn` rather than reporting a
runtime regression.

## Reports

Each smoke run writes a timestamped directory under `target/wordpress-real/`:

```text
wordpress-smoke-report.json
first-failure.json
server.log
http-transcript.jsonl
preflight.json
```

`wordpress-smoke-report.json` contains the run summary, preflight payload, phase
results, and artifact paths. `first-failure.json` keeps the first request,
status, diagnostic IDs, source path and line when available, excerpts, runtime
stack, and candidate owner layer.

Generated `target/` reports are local artifacts and must not be committed.

## Extracting A Reduced Fixture

After a failing smoke, run:

```bash
nix develop -c just wordpress-real-extract-first-failure
```

By default the extractor uses the newest
`target/wordpress-real/*/first-failure.json` and creates:

```text
target/wordpress-real/<run-id>/reduction/<slug>.php
target/wordpress-real/<run-id>/reduction/README.md
```

The scaffold preserves the source path, diagnostic ID, request shape, document
root context, DB state, and suggested destination. It is only a scaffold: reduce
the code to generic PHP before promoting it into fixtures.

## Ownership

Use the first failure class to pick the owning layer:

- `runtime`: `fixtures/runtime_semantics/wp_language_vm` or
  `include_eval_autoload`
- `stdlib`: `tests/phpt/generated/wp.core-builtins` or the owning stdlib module
- `web`: `tests/phpt/generated/wp.web-runtime` or `php_server` tests
- `database`: `tests/phpt/generated/wp.db-network` or mysqli/curl/openssl
  modules
- `diagnostics` or `environment`: `scripts/wordpress` or diagnostics tests/docs
- `timeout`: execution deadline handling or the owning phase's runtime layer

Do not special-case WordPress behavior in the runtime. Reduce the failure first,
then fix the generic PHP behavior in the owning layer.

## Tiers

- Tier 0: reduced gates pass
- Tier 1: real WordPress front controller executes a frontpage request without
  DB install
- Tier 2: install page loads against MariaDB
- Tier 3: installation completes
- Tier 4: post-install frontpage and admin login page render
- Tier 5: plugin/theme/update flows execute selected operations

For this project, "WordPress works" means Tier 4 is green on a local configured
WordPress checkout and MariaDB without unclassified fatal diagnostics. Tier 5 is
an expansion target for selected ecosystem flows, not a prerequisite for the
basic install-and-serve claim.
