# pdo_sqlite

- Strategy: SQLite-backed PDO MVP
- Classification: real implementation, still incomplete
- Selected manifest: `tests/phpt/manifests/modules/pdo_sqlite.selected.jsonl`
- Fixture: `tests/phpt/generated/pdo_sqlite/platform-checks.phpt`

## Implemented Scope

Prompt 4G enables `pdo_sqlite` by reusing the Prompt 4F `rusqlite` connection
and result layer.

Implemented behavior:

- `extension_loaded("pdo_sqlite")` and PDO driver discovery with `sqlite`.
- SQLite `:memory:` and root-constrained local file DSNs.
- `PDO::exec`, `query`, and `prepare` with `PDOStatement::execute`.
- `PDOStatement::fetch`, `fetchAll`, `fetchColumn`, `columnCount`, and
  `closeCursor` for associative, numeric, both, and column fetch modes.
- Basic `errorCode` and `errorInfo` plumbing through the SQLite connection.

## Remaining Gaps

- Stable ID: `PHPT-DATA-PDO-SQLITE-MVP-GAPS`
- Bound parameters are not implemented; prepared statements currently execute
  stored SQL without non-empty parameter arrays.
- SQLite-specific PDO callbacks (`sqliteCreateFunction`,
  `sqliteCreateAggregate`, and `sqliteCreateCollation`) are not implemented.
- Transactions, persistent connections, full attribute behavior, exact warning
  text, exception-mode behavior, and advanced fetch modes remain incomplete.
- Non-SQLite PDO drivers remain unavailable.

## Source References

- `ext/pdo_sqlite/pdo_sqlite.stub.php`
- `ext/pdo_sqlite/sqlite_driver.stub.php`
- `ext/pdo_sqlite/tests/`

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=pdo_sqlite`
