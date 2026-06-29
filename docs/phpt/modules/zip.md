# zip

- Priority: archive read/extract MVP
- Selected manifest: `tests/phpt/manifests/modules/zip.selected.jsonl`
- Current focused snapshot: 1 PASS, 0 SKIP, 0 FAIL, 0 BORK from 1 selected
  generated fixture

## Scope

- `ZipArchive` construction and `open` failure handling
- `close`, `count`, `numFiles`, `getNameIndex`, `getFromName`,
  `locateName`, `statName`, and `extractTo`
- Local-file ZIP reading for selected plugin/theme archive workflows

## Non-Scope

- ZIP writing and mutation
- Encryption, comments, external attributes, stream wrappers, and password
  support
- Exhaustive `ZipArchive` constants and error-code parity

## Selected PHPT Fixtures

- `tests/phpt/generated/zip/archive-basic.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/zip.rs`
- `crates/php_vm/src/vm/mod.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime zip`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just phpt-dev-module MODULE=zip`
- `nix develop -c just verify-phpt`

## Known Gaps

- Keep archive mutation and complete metadata parity as future work with
  focused fixtures.

## Request Filesystem Overlay

The `wp.request-filesystem` overlay reuses the local read/extract `ZipArchive`
MVP for package archive checks. ZIP writing, encryption, comments, passwords,
and complete metadata parity remain outside the selected gate.
