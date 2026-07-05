# pdo_pgsql

- Strategy: bounded PDO PostgreSQL execution surface over a PostgreSQL client backend
- Classification: partial live driver implementation
- Selected manifest: `tests/phpt/manifests/modules/pdo_pgsql.selected.jsonl`
- Fixtures:
  - `tests/phpt/generated/pdo_pgsql/platform-checks.phpt`
  - `tests/phpt/generated/pdo_pgsql/live-basic-dsn.phpt`

## Implemented Scope

This slice exposes the PDO PostgreSQL extension surface and the first live
connection/query path without claiming full libpq or server-version parity.

Implemented behavior:

- `extension_loaded("pdo_pgsql")`.
- PDO driver discovery includes `pgsql` through `pdo_drivers()` and
  `PDO::getAvailableDrivers()`.
- Generated `Pdo\Pgsql` and `PDO_PGSql_Ext` class metadata is visible through
  `class_exists`.
- `new PDO("pgsql:host=...;port=...;dbname=...", $user, $pass)` opens a live
  PostgreSQL connection through the `postgres` crate runtime backend.
- `PDO::query`, `PDO::exec`, `PDO::prepare`, `PDOStatement::execute`, `fetch`,
  `fetchAll`, `fetchColumn`, `rowCount`, `columnCount`, `closeCursor`,
  `errorCode`, and `errorInfo` route to PostgreSQL state for PostgreSQL-backed
  PDO handles.
- Basic transaction SQL, sequence-name `lastInsertId`, `PDO::ATTR_DRIVER_NAME`,
  PostgreSQL SQL literal quoting, scalar result conversion, and `$n`
  placeholder rewriting are implemented for PostgreSQL-backed PDO handles.

## Remaining Gaps

- Stable ID: `PHPT-DATA-PDO-PGSQL-MVP-GAPS`
- Unix-socket DSNs, persistent connections, timeout attributes, advanced
  server attributes, notifications, large objects, copy helpers, and full libpq
  parity remain outside this selected slice.
- PostgreSQL array/json/bytea handling, cursor metadata, full column metadata,
  and server-version-specific diagnostics remain bounded gaps.
- Live PHPT promotion remains opt-in and must stay disabled unless
  `PHRUST_POSTGRES_TEST_DSN` is set.

## Source References

- `ext/pdo_pgsql/pdo_pgsql.stub.php`
- `ext/pdo_pgsql/pgsql_driver.stub.php`
- `ext/pdo_pgsql/tests/`

## Target Gates

- `nix develop -c cargo test -p php_std introspection --no-fail-fast`
- `nix develop -c cargo test -p php_runtime pdo --no-fail-fast`
- `nix develop -c cargo test -p php_runtime postgres --no-fail-fast`
- `nix develop -c cargo test -p php_vm pdo_pgsql --no-fail-fast`
- `nix develop -c just phpt-dev-module MODULE=pdo_pgsql`
