# fileinfo

- Priority: media/archive MIME MVP
- Selected manifest: `tests/phpt/manifests/modules/fileinfo.selected.jsonl`
- Current focused snapshot: 1 PASS, 0 SKIP, 0 FAIL, 0 BORK from 1 selected
  generated fixture

## Scope

- `finfo_open`, `finfo_file`, `finfo_buffer`, and `finfo_close`
- `mime_content_type`
- Deterministic MIME sniffing for selected upload/archive payloads:
  PNG, JPEG, GIF, PDF, ZIP, JSON, XML/SVG, and plain text

## Non-Scope

- Host libmagic database parity
- Complete MIME database coverage
- All `FILEINFO_*` modes and flag combinations
- Charset reporting beyond the selected `FILEINFO_MIME_TYPE` workflow

## Selected PHPT Fixtures

- `tests/phpt/generated/fileinfo/mime-basic.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/modules/fileinfo.rs`
- `crates/php_runtime/src/builtins/registry.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_runtime fileinfo`
- `nix develop -c just phpt-dev-module MODULE=fileinfo`
- `nix develop -c just verify-phpt`

## Known Gaps

- Full libmagic parity remains out of scope. Add MIME patterns only with a
  caller-backed fixture and deterministic expected output.
