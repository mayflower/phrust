# Runtime Embedding and WASI Spike

This spike checks whether Runtime crate boundaries leave room for later
embedding or `wasm32-wasi` experiments. It is not a WASI support commitment.

## Current Dependency Boundary

| Crate | Role | Host/CLI-specific dependencies |
| --- | --- | --- |
| `php_source` | source text, spans, line maps | none beyond standard library data structures |
| `php_lexer` | tokenization | none beyond standard library data structures |
| `php_syntax` | lossless CST parser | none beyond parser-owned memory structures |
| `php_ast` | typed CST views | none beyond frontend data structures |
| `php_semantics` | HIR, declarations, scopes, diagnostics | no process or filesystem dependency in the core API |
| `php_ir` | source-mapped IR and HIR lowering | no process or filesystem dependency in the core API |
| `php_runtime` | values, diagnostics, context, arrays, objects | uses owned runtime data; host state is injected through `RuntimeContext` |
| `php_vm` | interpreter and include loader | core execution is in-process; include behavior is injected through `IncludeLoader` |
| `php_vm_cli` | command-line compile/dump/run/report | owns filesystem reads, stdout/stderr, process exit codes, and CLI argument parsing |
| `php_testkit` | local test and reference tools | intentionally host/process/filesystem oriented |

The useful separation is already present: source loading, output streams,
process exit codes, and reference execution are outside the VM core. The VM
accepts a compiled IR unit and returns a structured result.

## CLI-Specific Logic

The following behavior belongs in `php_vm_cli` or test tools, not in the core
VM/runtime crates:

- Reading PHP source files from the host filesystem.
- Resolving command-line arguments after `--`.
- Writing stdout/stderr streams.
- Translating `ExitStatus` into process exit codes.
- Rendering human-readable reports.
- Running reference PHP binaries.

The current `include` MVP still uses host filesystem paths through
`IncludeLoader`. That is acceptable for Runtime, but an embedded target should
provide an alternate loader backed by host callbacks, an in-memory file map, or
a virtual filesystem.

## WASI/Embedding Constraints

- `php_vm` and `php_runtime` should keep accepting injected context rather than
  reading process environment directly.
- Include roots and current working directory need an abstract policy before a
  WASI target can be treated as portable.
- Runtime output should remain byte-buffer based so embedders can collect or
  stream it without a process stdout dependency.
- Reference-diff and PHPT tools are not embedding targets.
- Generated reports under `target/` are host-only developer artifacts.

## Small Boundary Check

No large API split is needed in Runtime. The existing boundaries are adequate
for a later spike that compiles core crates for a restricted target, provided
the CLI and testkit crates are excluded.

## Runtime semantics Follow-Ups

1. Add an explicit in-memory include loader fixture to prove host-independent
   include behavior.
2. Keep `$argv`, `$_SERVER`, `$_ENV`, cwd, and include roots explicit in
   `RuntimeContext`.
3. Avoid adding direct `std::env`, process spawning, or stdout/stderr writes to
   `php_runtime` or `php_vm`.
4. If a WASI target is attempted, start with `php_source` through `php_vm`
   library tests, not the CLI or testkit tools.
