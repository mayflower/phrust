# Cranelift Code Manager

The `jit-cranelift` build owns native code in one process-level
`CraneliftCodeManager`. It replaces the former per-function `JITModule` leak.

## Ownership model

Cranelift 0.133's `JITModule` is `Send` but is not `Sync`: it contains mutable
state including `RefCell` symbol storage. Each active code generation therefore
owns one module behind a mutex. Compilation and finalization are serialized;
published entry points are immutable and can execute on pinned PHP worker
threads without taking the compiler lock.

Every `JitFunctionHandle` holds a `SharedJitCodeHandle`. That token owns an
`Arc` to its `CodeGeneration`, so executable memory cannot be freed while a
request or worker cache can still call it. Dropping the final generation owner
calls Cranelift's unsafe `JITModule::free_memory` only after all published
handles are gone.

Cranelift's system memory provider remains the W^X owner. Functions are
published only after `finalize_definitions`; the code manager never exposes a
writable code pointer or modifies a finalized entry.

## Identity and compile-once behavior

Process-cache keys and generated symbol names include:

- compiled-unit/IR fingerprint;
- function and region identity;
- runtime ABI hash;
- effective VM/host-ISA configuration hash;
- runtime invalidation generation;
- specialization version.

Runtime helper imports are additionally namespaced by address. Cranelift caches
resolved imports inside a long-lived module, so this prevents a later runtime
helper table from accidentally reusing an earlier relocation.

The manager mutex is also the compile-once synchronization point. A concurrent
request for the same key waits, observes the publication, and receives the same
generation-bound handle. Different-key contention may fall back at the VM's
existing compile-budget boundary; it cannot publish duplicate code.

## Bounds and retirement

Production defaults are 64 MiB total attributed native code and 1 MiB per
generation. Reaching the generation target seals it for future compilation.
When the total limit is reached, the oldest sealed generation is removed from
the process cache. Its bytes are retired immediately for accounting, while the
allocation remains callable until the last active handle drops. If active old
handles prevent reclamation, new compilation fails closed and Dense execution
continues.

The manager exposes exact per-operation events and process gauges:

```text
jit_process_cache_hits
jit_process_cache_misses
jit_compile_waits
jit_duplicate_compiles_avoided
jit_code_bytes_live
jit_code_bytes_retired
jit_code_generations
jit_evictions
```

Focused tests cover many functions in one generation, concurrent compile-once,
execution from multiple worker threads, execution while the compiler extends
the active generation, invalidation, old-handle lifetime, and limit eviction.

```bash
nix develop -c cargo test -p php_jit --features jit-cranelift code_manager::tests
```
