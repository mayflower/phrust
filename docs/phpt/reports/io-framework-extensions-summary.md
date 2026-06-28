# I/O and Framework Extensions Summary

Scope: I/O and framework extension modules.

Reference target:

- PHP series: 8.5
- PHP version: 8.5.7
- Reference binary:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`
- Reference source:
  `/Volumes/CrucialMusic/src/phrust/third_party/php-src`

## Scope

This report closes the selected, deterministic module harnesses for framework
and Composer-oriented local execution. It does not update the full PHPT
baseline, vendor `php-src`, or promote broad upstream extension corpora.

## Module Status

| Module | Selected status | Implemented surface |
| --- | --- | --- |
| `filesystem.streams` | 11 selected fixtures green | Local files, cwd/include_path state, php memory/temp streams, resources, include/require local semantics. |
| `json` | 10 selected fixtures green | Encode/decode basics, common flags, request-local last-error state, `JSON_THROW_ON_ERROR`. |
| `pcre` | 5 selected fixtures green | Match captures, last-error state, replace/split/grep/quote, selected callback dispatch. |
| `date` | 7 selected fixtures green | Request timezone state, selected Date/Time functions and classes, deterministic timezone/interval MVP. |
| `session` | Platform fixture green | Request-local in-memory session state, `$_SESSION`, selected session status/id/name/save-path/cache helpers, and in-process web `PHPSESSID` reuse. |
| `sqlite3` | Platform fixture green | `SQLite3` backed by `rusqlite`, local file and `:memory:` databases, query/result helpers, selected constants. |
| `pdo` | Platform fixture green | SQLite-only PDO core surface, `pdo_drivers`, constants, `PDOException`, statement/result basics. |
| `pdo_sqlite` | Platform fixture green | SQLite DSN support through PDO, `exec`, `query`, `prepare`, `execute`, fetch helpers. |
| `phar` | Platform fixture green | Read-only local uncompressed PHAR archive parsing, `phar://` `file_get_contents`, stream reads, include, and `new Phar($path)`. |

## Branch Changes

- Session support now lives in request execution state instead of global state,
  with deterministic request-local persistence for the current VM request and
  in-process web cookie reuse.
- SQLite support uses the approved `rusqlite` dependency in `php_runtime` and
  exposes runtime-owned connection/result handles through VM internal objects.
- PDO and PDO_SQLite reuse the same SQLite runtime layer and intentionally keep
  unsupported drivers unavailable.
- PHAR support adds a read-only archive parser and routes `phar://` reads
  through the existing resource and include pipelines.
- Extension discovery now enables `session`, `sqlite3`, `pdo`, `pdo_sqlite`,
  and `phar` in the standard-library introspection surface.
- Each new module has a generated PHPT platform fixture and a module document
  under `docs/phpt/modules/`.

## Remaining Gaps

- `session`: file-backed session handlers, custom handlers, serialization
  formats, INI policy, and full web SAPI behavior remain outside the
  request-local MVP.
- `sqlite3`: prepared statements, callbacks, aggregate/collation hooks, backup,
  blob I/O, and byte-perfect SQLite warning/exception parity remain future work.
- `pdo` and `pdo_sqlite`: non-SQLite drivers, attributes, transactions, bound
  parameters, cursor modes, class fetch modes, driver callbacks, and full
  diagnostic parity remain future work.
- `phar`: compressed entries, signatures, write/update APIs, aliases,
  `PharData`, `PharFileInfo`, tar/zip-backed archives, metadata/stat parity,
  and full Composer PHAR edge behavior remain future work.
- Broader upstream `ext/json`, `ext/pcre`, `ext/date`, `ext/session`,
  `ext/sqlite3`, `ext/pdo`, `ext/pdo_sqlite`, and `ext/phar` corpora remain
  separate promotion work after their documented gaps close.

## Closeout Gates

closeout validation:

- `nix develop -c just verify-runtime`: PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just verify-stdlib`:
  PASS.
- `nix develop -c just phpt-dev-module MODULE=filesystem.streams`: PASS,
  reference 11 green and target 11 green.
- `nix develop -c just phpt-dev-module MODULE=json`: PASS, reference 10
  green and target 10 green.
- `nix develop -c just phpt-dev-module MODULE=pcre`: PASS, reference 5 green
  and target 5 green.
- `nix develop -c just phpt-dev-module MODULE=date`: PASS, reference 7 green
  and target 7 green.
- `nix develop -c just phpt-dev-module MODULE=session`: PASS, reference 1
  green and target 1 green.
- `nix develop -c just phpt-dev-module MODULE=sqlite3`: PASS, reference 1
  green and target 1 green.
- `nix develop -c just phpt-dev-module MODULE=pdo`: PASS, reference 1 green
  and target 1 green.
- `nix develop -c just phpt-dev-module MODULE=pdo_sqlite`: PASS, reference 1
  green and target 1 green.
- `nix develop -c just phpt-dev-module MODULE=phar`: PASS, reference 1 green
  and target 1 green.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just composer-smoke`:
  PASS, total 5, pass 5, fail 0, skip 0, known_gap 0.

Run PHPT module gates with `PHPT_REUSE_LAST=0` and
`PHPT_DEV_REUSE_TARGET_PASS=0` for fresh target evidence.
