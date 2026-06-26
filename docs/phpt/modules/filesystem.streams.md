# filesystem.streams

- Priority: 11
- Selected manifest: `tests/phpt/manifests/modules/filesystem.streams.selected.jsonl`
- Current counts: 7 PASS, 0 SKIP, 0 FAIL, 0 BORK from 7 selected module fixtures

## Scope

- local filesystem
- php://memory streams
- resources
- include_path
- include/require

## Non-Scope

- network streams
- PHAR streams
- extension-backed wrappers
- user stream wrappers

## Selected PHPT Fixtures

- `tests/phpt/generated/filesystem.streams/local-file-roundtrip.phpt`
- `tests/phpt/generated/filesystem.streams/php-memory-stream.phpt`
- `tests/phpt/generated/filesystem.streams/include-path-scope.phpt`
- `tests/phpt/generated/filesystem.streams/directory-cwd-roundtrip.phpt`
- `tests/phpt/generated/filesystem.streams/local-file-resource.phpt`
- `tests/phpt/generated/filesystem.streams/missing-file-warnings.phpt`
- `tests/phpt/generated/filesystem.streams/php-temp-stream.phpt`

## Relevant php-src Source Areas

- `ext/standard/tests/file/`
- `ext/standard/tests/streams/`
- `crates/php_runtime/`

## Target Gates

- `nix develop -c just phpt-module MODULE=filesystem.streams`

## Known Gaps

- Network streams, PHAR streams, extension-backed wrappers, and user stream
  wrappers are outside this module contract.
- Additional local file, directory, cwd, and warning/error PHPTs should be
  added as deterministic selected fixtures before expanding this module count.

## Next Step

Extend only with deterministic local filesystem and builtin stream fixtures.
