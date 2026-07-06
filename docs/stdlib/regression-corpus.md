# Standard library Regression Corpus

`just compat-corpus-smoke` runs the curated Composer/framework-adjacent corpus
under `tests/fixtures/stdlib/corpus` through the existing standard-library differential
harness. The corpus is intentionally local and self-contained: it vendors no
external framework code, does not use the network, and does not require a
database.

Each PHP snippet contains:

- a `standard-library-diff` metadata line with a stable test ID and `area=corpus`;
- a `purpose` comment explaining the realistic behavior being covered;
- a `reference-output` comment documenting the expected normalized output.

Coverage areas:

| Fixture | Purpose |
| --- | --- |
| `container_autoload.php` | Composer-like autoload and container service construction |
| `env_parsing.php` | Request-local environment parsing |
| `json_config.php` | Local JSON config decode and stable encode |
| `regex_routing.php` | Regex route match and dispatch-table lookup |
| `datetime_version_parsing.php` | PHP version checks and deterministic date parsing |
| `array_manipulation.php` | Request option normalization through array helpers |
| `reflection_attributes.php` | Reflection-based attribute metadata scanning |

The runner writes `target/stdlib/corpus/stdlib-diff-report.json` and per-fixture
detail snapshots. If no reference PHP is configured and the pinned reference
binary is not built, the existing harness reports each pass fixture as skipped
with the same explicit reference setup message used by other standard-library gates.
