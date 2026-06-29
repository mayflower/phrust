# wp.request-filesystem

- Priority: WordPress request/filesystem update primitives
- Selected manifest: `tests/phpt/manifests/modules/wp.request-filesystem.selected.jsonl`
- Current target snapshot: 6 PASS, 1 SKIP, 0 FAIL, 0 BORK from 7 selected
  generated fixtures

## Scope

- CLI-comparable request, upload constant, extension, and `$_FILES` surface
- Local permission/stat/temp/write helpers used by plugin and theme updates
- Directory iteration and request-local stream context defaults/options
- Read-only `phar://` package reads/includes and local archive/media probes
- zlib gzip file-handle helpers for package payloads
- Multipart upload metadata and move success through `php_server` transport
  tests and `server-smoke`

## Non-Scope

- FPM, FastCGI, CGI, Apache SAPI, mod_php, and phpdbg
- Remote filesystem clients and database clients
- Full libmagic parity, stream filters, and writable PHAR archives
- Complete PHP `Directory` object method/property parity

## Selected PHPT Fixtures

- `tests/phpt/generated/wp.request-filesystem/platform-surface.phpt`
- `tests/phpt/generated/wp.request-filesystem/local-permission-stat.phpt`
- `tests/phpt/generated/wp.request-filesystem/temp-directory-stream-context.phpt`
- `tests/phpt/generated/wp.request-filesystem/gzip-file-handles.phpt`
- `tests/phpt/generated/wp.request-filesystem/package-archive-media.phpt`
- `tests/phpt/generated/wp.request-filesystem/upload-cli-surface.phpt`
- `tests/phpt/generated/wp.request-filesystem/multipart-transport-only.phpt`

## Relevant Source Areas

- `crates/php_runtime/src/builtins/context.rs`
- `crates/php_runtime/src/builtins/modules/filesystem.rs`
- `crates/php_runtime/src/builtins/modules/streams.rs`
- `crates/php_runtime/src/builtins/modules/zlib.rs`
- `crates/php_runtime/src/resource.rs`
- `crates/php_vm/src/vm/mod.rs`
- `crates/php_std/src/lib.rs`

## Target Gates

- `nix develop -c cargo test -p php_server`
- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c just server-smoke`
- `nix develop -c just phpt-dev-module MODULE=wp.request-filesystem`
- `nix develop -c just phpt-dev-module MODULE=filesystem.streams`
- `nix develop -c just phpt-dev-module MODULE=phar`
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-server`

## Known Gaps

- `disk_free_space()` and `disk_total_space()` return deterministic positive
  capability-backed values for allowed existing paths instead of host capacity.
- `dir()` routes through the directory resource model; full PHP `Directory`
  object parity is not part of this selected gate.
- `stream_set_timeout()` returns false for the current local stream resources,
  matching the selected local-file and `php://memory` reference behavior.
- Multipart upload success is verified through server transport tests because
  CLI PHPT cannot populate request-local upload metadata.
