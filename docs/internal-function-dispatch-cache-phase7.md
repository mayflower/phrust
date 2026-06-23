# Phase 7 Internal Function Dispatch Cache

Prompt 07.41 adds a request-local VM cache for generic internal builtin dispatch
metadata. The cache stores the runtime builtin registry entry for hot standard
library names after the first lookup:

- `count`
- `strlen`
- `is_*`
- selected array and string helpers when they exist in `BuiltinRegistry`

The cache only avoids repeated registry lookup for dispatch metadata. It does
not change `function_exists`, reflection metadata, named-argument conversion,
arity checks, type checks, or builtin `ValueError` diagnostics.

The VM option `VmOptions::internal_function_dispatch_cache` is enabled by
default and can be disabled for A/B tests. Counters are emitted when
`collect_counters` is enabled:

- `internal_function_dispatches`
- `internal_function_dispatch_cache_hits`
- `internal_function_dispatch_cache_misses`
- `internal_count_array_direct_fast_path_hits`

The only semantic fast path is the conservative `count(array)` direct case. It
runs after builtin arguments have been normalized to positional values, and only
for exactly one argument whose effective value is an array. Wrong arity,
non-array values, recursive mode, objects, and references that do not resolve to
arrays fall back to the existing builtin handler.

The Phase 7 smoke benchmark fixture `stdlib_dispatch.php` exercises repeated
`count`, `strlen`, `is_int`, `array_values`, and `strtolower` calls so
`docs/hotpaths-phase7.md` can report visible dispatch-cache hits from the
standard-library-heavy path.
