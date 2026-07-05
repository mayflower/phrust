# zip

- Priority: archive read/extract MVP
- Selected manifest: `tests/phpt/manifests/modules/zip.selected.jsonl`
- Current focused snapshot: 3 PASS, 0 SKIP, 0 FAIL, 0 BORK from 3 selected
  fixtures

## Scope

- `ZipArchive` construction and `open` failure handling
- `close`, `count`, `numFiles`, `getNameIndex`, `getFromName`,
  `locateName`, `statName`, and `extractTo`
- Deprecated procedural resource helpers: `zip_open`, `zip_read`,
  `zip_close`, `zip_entry_open`, `zip_entry_read`, `zip_entry_close`,
  `zip_entry_name`, `zip_entry_filesize`, `zip_entry_compressedsize`, and
  `zip_entry_compressionmethod`
- Local-file ZIP reading for selected plugin/theme archive workflows
- Selected create/write methods: `addEmptyDir`, `addFromString`, `addFile`,
  `CREATE`, `OVERWRITE`, `FL_OVERWRITE`, and `LENGTH_TO_END`

## Non-Scope

- ZIP writing and mutation beyond selected local add/write methods
- Encryption, comments, external attributes, stream wrappers, and password
  support
- Exhaustive `ZipArchive` constants and error-code parity

## Selected PHPT Fixtures

- `tests/phpt/generated/zip/archive-basic.phpt`
- `tests/phpt/generated/zip/legacy-resource-api.phpt`
- `ext/zip/tests/oo_extract.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/zip.rs`
- `crates/php_vm/src/vm/mod.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime zip`
- `nix develop -c cargo test -p php_vm`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=zip`
- `nix develop -c just verify-phpt`

## Known Gaps

- Keep archive mutation and complete metadata parity beyond the selected
  WordPress package path as future work with focused fixtures.
- `oo_open.phpt` remains outside the selected gate until the complete
  `ZipArchive::CREATE` mutation surface, comments, and constants are
  implemented.

## Request Filesystem Overlay

The `wp.request-filesystem` overlay reuses the local read/extract `ZipArchive`
MVP for package archive checks. ZIP writing, encryption, comments, passwords,
and complete metadata parity remain outside the selected gate.
