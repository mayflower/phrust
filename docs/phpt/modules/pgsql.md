# pgsql

- Strategy: bounded procedural PostgreSQL execution surface over the shared PostgreSQL client backend
- Classification: partial live driver implementation
- Selected manifest: `tests/phpt/manifests/modules/pgsql.selected.jsonl`
- Fixtures:
  - `tests/phpt/generated/pgsql/platform-checks.phpt`
  - `tests/phpt/generated/pgsql/live-basic-dsn.phpt`

## Implemented Scope

This slice exposes the procedural `pgsql` extension surface needed for basic
database connectivity without claiming full libpq parity.

Implemented behavior:

- `extension_loaded("pgsql")`.
- Generated `PgSql\Connection`, `PgSql\Result`, and `PgSql\Lob` class metadata
  is visible through `class_exists`.
- `PGSQL_ASSOC`, `PGSQL_NUM`, `PGSQL_BOTH`, and `PGSQL_CONNECTION_OK` are
  registered with PHP-compatible values.
- `pg_connect` opens a live PostgreSQL connection through the shared
  `postgres` crate runtime backend.
- `pg_query`, `pg_prepare`, `pg_execute`, `pg_fetch_array`, `pg_fetch_assoc`,
  `pg_fetch_row`, `pg_fetch_object`, `pg_fetch_result`, `pg_num_rows`,
  `pg_num_fields`, `pg_affected_rows`, `pg_free_result`, `pg_close`, and
  `pg_last_error` are implemented for bounded live handles.
- `pg_escape_string`, `pg_escape_literal`, `pg_escape_identifier`, and
  `pg_escape_bytea` provide deterministic PostgreSQL-compatible escaping for
  common scalar values.
- The default connection follows the most recent successful `pg_connect` for
  optional-connection call forms.

## Remaining Gaps

- Stable ID: `PHPT-DATA-PGSQL-MVP-GAPS`
- Persistent connections, connection pooling, async/polling APIs, notifications,
  tracing, copy helpers, large objects, and full libpq option/status metadata
  remain outside this selected slice.
- Upstream object/resource identity quirks, default connection edge cases,
  server-version-specific diagnostics, field metadata, and advanced type
  conversion remain bounded gaps.
- Live PHPT promotion remains opt-in and must stay disabled unless
  `PHRUST_POSTGRES_TEST_DSN` is set.

## Source References

- `ext/pgsql/pgsql.stub.php`
- `ext/pgsql/tests/`

## Target Gates

- `nix develop -c cargo test -p php_runtime postgres --no-fail-fast`
- `nix develop -c cargo test -p php_runtime pgsql --no-fail-fast`
- `nix develop -c cargo test -p php_std introspection --no-fail-fast`
- `nix develop -c cargo test -p php_vm pdo --no-fail-fast`
- `nix develop -c just phpt-dev-module MODULE=pgsql`
