# wp.core-builtins

- Strategy: WordPress-focused generated PHPT harness for core builtins,
  configuration helpers, HTTP response APIs, output buffering, and common
  helper functions.
- Selected manifest:
  `tests/phpt/manifests/modules/wp.core-builtins.selected.jsonl`
- Selected gate: generated fixtures covering symbol and extension
  introspection, INI/environment/config access, CLI-comparable HTTP response
  state, output buffering, URL/filter/hash/password helpers, and
  serialization roundtrips.

## Runtime Contract

- Symbol, extension, and class-like introspection covers `define`, `defined`,
  `constant`, `function_exists`, `method_exists`, `class_exists`,
  `interface_exists`, `trait_exists`, `enum_exists`, `is_subclass_of`,
  `extension_loaded`, `get_loaded_extensions`, and `phpversion`.
- INI/config/environment helpers cover `ini_get`, `ini_set`, `ini_get_all`,
  `get_cfg_var`, `getenv`, `putenv`, `php_sapi_name`, `memory_get_usage`, and
  `memory_get_peak_usage`.
- HTTP response functions cover the callable surface for `header`,
  `header_remove`, `headers_list`, `headers_sent`, `http_response_code`,
  `setcookie`, and `setrawcookie`. CLI PHPT coverage is intentionally limited
  to initial state and function availability; transport behavior belongs to
  server tests.
- Output buffering covers stack capture, length/level inspection, clean, and
  flush behavior used by template rendering.
- Helper coverage includes `parse_str`, `parse_url`, `urlencode`,
  `urldecode`, `rawurlencode`, `rawurldecode`, `http_build_query`,
  `filter_var`, `filter_input`, `hash`, `hash_hmac`, `hash_equals`,
  `random_bytes`, `random_int`, `password_hash`, `password_verify`,
  `password_needs_rehash`, `serialize`, and `unserialize`.

## Selected Fixtures

- `tests/phpt/generated/wp.core-builtins/symbol-extension-introspection.phpt`
- `tests/phpt/generated/wp.core-builtins/ini-env-config.phpt`
- `tests/phpt/generated/wp.core-builtins/http-response.phpt`
- `tests/phpt/generated/wp.core-builtins/output-buffering.phpt`
- `tests/phpt/generated/wp.core-builtins/url-filter-hash-password-serialization.phpt`
- `tests/phpt/generated/wp.core-builtins/filter-helpers.phpt`

## Known Gaps

| Stable ID | Current behavior | Next owner layer |
| --- | --- | --- |
| `PHPT-WP-CORE-HTTP-CLI-GAP` | CLI PHPTs cover callable surface and initial state only; emitted response headers are proved through server/runtime gates. | `php_server` transport tests |
| `PHPT-WP-CORE-MEMORY-GAP` | Memory helpers return deterministic runtime-owned counters, not Zend heap accounting. | future runtime memory accounting |
| `PHPT-WP-CORE-PASSWORD-ARGON-GAP` | Password helpers implement bcrypt-compatible behavior only. | future password algorithm expansion |
| `PHPT-WP-CORE-STDLIB-MATRIX-GAP` | The helper fixture is a selected WordPress smoke slice, not exhaustive stdlib parity. | owning stdlib modules |

## Target Gates

- `nix develop -c cargo test -p php_runtime`
- `nix develop -c cargo test -p php_vm`
- `nix develop -c cargo test -p php_std`
- `nix develop -c cargo test -p php_server`
- `nix develop -c just server-smoke`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=wp.core-builtins`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.variables`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.strings`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=standard.serialization`
- `PHPT_REUSE_LAST=0 PHPT_DEV_REUSE_TARGET_PASS=0 nix develop -c just phpt-dev-module MODULE=diagnostics.output`
- `nix develop -c just verify-runtime`
- `nix develop -c just verify-stdlib`
- `nix develop -c just verify-server`
- `nix develop -c just verify-phpt`
