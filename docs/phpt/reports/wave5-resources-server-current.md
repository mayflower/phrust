# Wave 5B Resources, Streams, SAPI, and Server Current Report

Reference target: PHP 8.5.7 (`php-8.5.7`).

Branch: `wave5b-resources-streams-sapi-server`

Reference binary used for this report:
`/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php`

## Baseline Gates

- `nix develop -c just server-compat-smoke all`: PASS.
  Baseline sections covered static files, nested URL-encoded input, bounded
  multipart uploads, cookies, persistent sessions, output buffering, and
  include execution.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-server`:
  PASS. The gate ran `php_executor`, `php_server`, health tests, and the server
  compatibility smoke.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`:
  PASS. `diff-streams` reported total=2 pass=2 fail=0 skip=0 known_gap=0.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=session`:
  reference PASS 7, target PASS 7.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=phar`:
  reference PASS 6, target PASS 6.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=curl`:
  reference PASS 7, target PASS 7.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=mysqli`:
  reference PASS 5, target PASS 5.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=pdo`:
  reference PASS 4, target PASS 4.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.db-network`:
  reference PASS 10, target PASS 10.
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.web-runtime`:
  reference PASS 2, target PASS 2.

## Implemented Updates

- `fseek` now models signed offsets with `SEEK_SET`, `SEEK_CUR`, and
  `SEEK_END`. Invalid negative targets and invalid `whence` values return `-1`
  without moving the stream cursor.
- `file_get_contents("php://input")` now receives the HTTP request raw body
  through VM builtin dispatch instead of opening an empty request input stream.
- `server-compat-smoke all` now includes strict sections for response
  headers/status, `php://input`, `php://stdout` plus memory stream output,
  request-local filesystem CWD behavior, cooperative execution deadlines, and
  loopback cache invalidation. `session-persistence` is also available as an
  alias for the persistent session section.
- Closed stale documentation gaps for `fseek` whence handling and VM-owned CWD
  persistence across builtin calls.

## Focused Validation

- `nix develop -c cargo test -p php_runtime stream_seek_supports_set_current_and_end_origins`:
  PASS.
- `nix develop -c cargo test -p php_runtime streams`: PASS.
- `nix develop -c cargo test -p php_runtime filesystem`: PASS, 5 tests.
- `nix develop -c cargo test -p php_runtime session`: PASS, 4 tests.
- `nix develop -c cargo test -p php_vm php_input_stream_reads_http_request_body_inside_vm_builtins`:
  PASS.
- `nix develop -c cargo test -p php_vm`: PASS, 560 tests.
- `nix develop -c cargo test -p php_server`: PASS, 46 library tests and 42
  health tests.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php nix develop -c just diff-streams`:
  PASS, total=2 pass=2 fail=0 skip=0 known_gap=0.
- `nix develop -c just phpt-dev-build`: PASS after the generated PHPT fixture
  was expanded.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=filesystem.streams`:
  PASS, reference PASS 25 and target PASS 25.
- `nix develop -c just server-compat-smoke all`: PASS with static, input,
  upload, cookie, session, output-buffer, include, headers, php-input,
  stream-output, filesystem-cwd, deadline, and cache-invalidation sections.

## Aggregate Validation

- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-server`:
  PASS.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-stdlib`:
  PASS. `diff-stdlib` reported total=43 pass=37 fail=0 skip=0 known_gap=6;
  `diff-streams` reported total=2 pass=2 fail=0 skip=0 known_gap=0.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-runtime`:
  PASS. Runtime semantics aggregate reported total=366 pass=304 fail=0 skip=0
  known_gap=62.
- `REFERENCE_PHP=/Volumes/CrucialMusic/src/phrust/third_party/php-src/sapi/cli/php PHP_SRC_DIR=/Volumes/CrucialMusic/src/phrust/third_party/php-src nix develop -c just verify-phpt`:
  PASS.
- Final selected PHPT modules:
  `session` reference/target PASS 7/7, `phar` PASS 6/6, `curl` PASS 7/7,
  `wp.web-runtime` PASS 2/2, and `wp.db-network` PASS 10/10.

## Remaining Explicit Gaps

- Network streams, TLS wrappers, DB access, and process execution remain
  default-off capabilities. Existing `curl`, `mysqli`, `pdo`, and
  `wp.db-network` PHPT modules prove deterministic disabled/local fixture
  behavior only.
- PHAR remains read-only/optional by policy and is still bounded by ADR-0066
  and the selected `phar` PHPT module.
- Byte-perfect stat arrays, filesystem warning text, advanced glob flags,
  stream filters, user stream wrappers, host TTY probing, and `tmpfile`
  unlink-on-close lifetime remain documented known gaps.
- The integrated server remains an application compatibility layer, not FPM,
  FastCGI, CGI, Apache module behavior, Zend extension ABI emulation, or
  external PHP worker execution.
