# Runtime Builtin Modules

The runtime builtin layer is organized by runtime responsibility. Public
consumers continue to use `php_runtime::builtins`;
the internal layout separates request context, errors, signatures, registry
assembly, and PHP module ownership.

## Layout

- `crates/php_runtime/src/builtins/mod.rs` exports the stable builtin API.
- `context.rs` owns request-local runtime services passed to builtins, including
  output, cwd, include path, filesystem policy, resources, PCRE state, JSON
  state, and emitted diagnostics.
- `error.rs` owns `BuiltinError` and stable diagnostic IDs.
- `signatures.rs` owns the internal function pointer and result aliases.
- `registry.rs` owns `BuiltinEntry`, compatibility classification, and
  deterministic registry lookup.
- `modules/*.rs` own module-level builtin registration slices.

`BuiltinRegistry` flattens the module slices through a `OnceLock`, sorts the
entries by builtin name, and exposes the same stable `entries`, `get`, and
`contains` behavior as before. Sorting at the registry boundary keeps lookup and
test behavior deterministic while allowing module files to group entries by
functional ownership.

## Module Ownership

| Builtin area | Module file |
| --- | --- |
| Registry glue, scalar/type helpers, output/config/env/process placeholders, tokenizer, serialization, var dumping | `builtins/modules/core.rs` |
| Array functions, array callback placeholders, array sorting placeholders | `builtins/modules/arrays.rs` |
| String, formatting, encoding, hashing, URL/HTML, version comparison | `builtins/modules/strings.rs` |
| Numeric and math functions | `builtins/modules/math.rs` |
| Path and filesystem functions | `builtins/modules/filesystem.rs` |
| Resource streams, directories, stream metadata/context/include-path helpers | `builtins/modules/streams.rs` |
| JSON encode/decode/validate and JSON last-error functions | `builtins/modules/json.rs` |
| PCRE functions and PCRE last-error functions | `builtins/modules/pcre.rs` |
| Date/time/timezone functions | `builtins/modules/date.rs` |
| SPL object helpers and SPL autoload placeholders | `builtins/modules/spl.rs` |
| Symbol introspection, callable dispatch placeholders, class/function/method existence helpers | `builtins/modules/reflection.rs` |

## Adding Builtins

New standard-library functions should be added to the file matching their PHP
module ownership, and their `BuiltinEntry` should be added to that file's
`ENTRIES` slice. Shared helpers belong in `core.rs` only when they are reused
across module boundaries; otherwise keep helpers private to the module that owns
the builtin.

Do not add files or registries whose ownership is based on implementation
history. Unsupported behavior should remain explicit through stable runtime
diagnostics or VM-level placeholders rather than silently returning plausible
values.
