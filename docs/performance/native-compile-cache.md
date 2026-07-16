# Native compile-record cache

The server shares one `VmWorkerState` across requests. Its compile-record cache
sits above Region IR construction and Cranelift emission, so a warm request can
reuse already-published native handles without rebuilding the same function
graph.

The cache has two independently bounded LRU segments:

- primary entries are keys that execution explicitly requested;
- aliases are the additional function entries published by the same compiled
  graph.

Keeping the segments separate prevents a large PHP application from filling
the cache with graph aliases and evicting every include-entry key needed by the
next request. An alias hit is promoted to the primary segment. Both segments
remain bounded by the configured entry capacity.

Validate the policy with:

```bash
nix develop -c cargo test -p php_vm native_compile_cache --lib
```

## Restart-persistent PNA2 unit bundles

When `--native-cache` permits reads or writes, every executable IR unit is a
cache candidate, including declaration-heavy include/eval units. The same
Cranelift lowering that publishes the process-local entry retains actual code
bytes and symbolic relocations before `JITModule` finalization. PNA2 stores one
deduplicated bundle per unit identity:

- all native function entries in the unit;
- code bytes and internal-symbol relocations;
- stable helper IDs and names, never helper process addresses;
- deterministic `PRM4` state metadata for exceptions, native continuations, OSR,
  generators, and fibers. Each shared function graph is serialized once and
  root entries refer to it by a compact index.

Writers emit only PNA2. The loader accepts PNA1 for one migration window using
the same strict checksum, section, identity, relocation, and W^X validation;
the next successful write replaces it with PNA2. Function graphs shared by
multiple published entries are emitted once, while helper imports and internal
relocations are deduplicated at bundle scope. PNA2 also stores the uniform
packed-call ABI once for the function-entry section rather than repeating it in
every function record. PNA1/PRM3 remains read-only compatibility data during
the migration window.

Diagnostic linkage and footprint collection is deliberately separate from
clean timings. Use `just native-linkage-report COUNTERS`,
`just native-footprint-report CACHE_DIR`, and then
`just native-linkage-tranche-report` to assemble the complete C13 report tree.
Pass a native-smoke linkage JSON with `--smoke-linkage`; the builder records it
as synthetic, non-acceptance diagnostic evidence rather than treating its
direct-call ratio as a WordPress result.
The tranche builder records unavailable WordPress/RSS inputs as unmeasured with
an exact reason; structural gates never substitute for performance results.

Same-unit calls with fully materialized positional arguments call the compiled
callee symbol directly, including callees with declared parameter types. A
versioned `phrust_native_argument_check` helper applies call-site strictness,
weak scalar coercion, callable checks, by-reference write-back, and catchable
`TypeError` publication before the direct branch. Named/unpacked arguments and
other runtime-bound shapes remain on the typed call dispatcher.

Published functions from include/eval units execute through a scoped active-unit
view on the existing request context. The view swaps immutable unit metadata,
native entries, continuations, and callsite tables while retaining the same
value store, frame arena, output, globals, extension state, and request-owned
resources. Constant handles are materialized at the boundary, and return values
are materialized before restoring the caller unit. This removes the former
nested `NativeExecutionContext` and state-move/merge path for successful
cross-unit calls.

On x86-64, an exact forwarding wrapper with the same parameter and return
contract as a side-effect-free leaf callee reuses its incoming packed argument
buffer and emits Cranelift `return_call`. The subset excludes methods,
closures, generators, references, variadics, handlers, and callees with
observable operations; broader tail calls require an explicit owned arena-frame
transfer protocol. Native code preserves frame pointers because Cranelift's
x86-64 tail-call lowering currently requires them. Other architectures retain
their platform ABI and do not advertise tail calls.

On AMD64, helper calls target an artifact-local `movabs`/`jmp` trampoline. The
loader resolves the trampoline immediate through the current versioned helper
registry after validating the artifact, then changes the mapping from writable
to executable. This keeps the original Cranelift `call rel32` in range without
persisting an absolute address.

Transition metadata follows the same 64-register publication bound as the
generated resume loaders. Do not publish entries after that bound: besides
advertising a nonexistent transition, cloning an ever-growing register prefix
turns metadata construction and serialization quadratic for generated
declaration units.

Focused restart checks:

```bash
nix develop -c cargo test -p php_vm vm_reloads_ --lib
nix develop -c cargo test -p php_jit native_helpers_publish_symbolic_restart_cache_relocations --lib
```

The default security/resource bounds remain 64 MiB per artifact, 32 MiB of
code per artifact, 65,536 relocations, and 512 MiB for the cache directory.
The unchanged WordPress 6.8.3 frontpage cache fitted inside those defaults
after bounding transition metadata; increasing the limits is not the remedy
for metadata growth.

For a real application, collect an instrumented request after at least one
warmup. The warm request must report zero `compile_attempts` before its latency
is treated as compilation-free performance evidence.
