# Engine Defaults

Phrust exposes one shared engine profile model through `php_executor`. The CLI
and HTTP server consume the same profiles so normal users do not need to choose
optimizer, execution-format, quickening, inline-cache, tiering, cache, or JIT
flags for ordinary execution.

## Profiles

| Profile | Use | Compile options | VM behavior |
| --- | --- | --- | --- |
| `default` | Normal CLI and server execution | optimizer level 2 | dense-bytecode `auto` with safe IR fallback, superinstructions, quickening, inline caches, tiering, and the guarded native tier where backend support exists |
| `baseline` | Compatibility and semantic debugging | optimizer level 0 | rich IR interpreter only, quickening off, inline caches off, tiering off, Cranelift/JIT off |
| `experimental-jit` | Native diagnostics and compiler experiments | optimizer level 2 | same managed fast paths and guarded native tier as `default`, with diagnostic-friendly thresholds |

`fast` is accepted as a compatibility alias for `default`.

## Command Surface

The CLI default is equivalent to:

```bash
php-vm run --engine-preset=default file.php
```

The server default is equivalent to:

```bash
phrust-server --engine-preset default --docroot public
```

Use `--engine-preset=baseline` to roll back to the compatibility profile. Low
level CLI flags remain available for focused tests and performance engineering,
but they are not the normal product surface.

The native tier is not a separate end-user mode. The managed runtime requests it
automatically only for narrow hot regions with exact live-state snapshots,
helper ABI/version hashes, runtime configuration hashes, invalidation epochs,
compile budgets, and platform/backend support. Unsupported platforms record
`native_platform_unavailable`; rejected regions and side exits record specific
reason counters and resume through the generic interpreter without disabling
dense bytecode, quickening, inline caches, or other local fast paths.

## Safety Gates

`nix develop -c just default-profile-smoke` compares `baseline` and `default`
over selected runtime, stdlib, performance, framework-like, and local PHPT smoke
fixtures. It checks stdout, stderr/runtime diagnostics, exit status, fallback
counters, managed fast-path counters, and native-tier availability/execution
counters for the default profile. The gate writes local-only JSON and Markdown under
`target/performance/default-profile/` and is included in `verify-performance`.

`nix develop -c just managed-fast-coverage` runs curated fixtures under the
default profile, asserts dense bytecode, superinstruction, quickening,
inline-cache, array-shape, builtin intrinsic, string/output, include/cache, and
native-tier policy counters, and checks bounded fallback reasons for reference,
COW, magic method, output, dynamic include, numeric-string, by-reference,
exception, and unsupported native-region cases. The gate writes local-only
reports under `target/performance/managed-fast/` and is included in
`verify-performance`.

New fast paths must reuse shared semantic helpers, keep local fallback to the
generic interpreter, expose exact fallback counters, and add focused fast-hit
assertions before they become part of the default managed profile.
