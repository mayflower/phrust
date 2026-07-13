# Performance Cranelift Safety Audit

that stage audits the default-off Cranelift native execution experiment.
The audited surface is compiled only with the `jit-cranelift` feature and still
requires a runtime opt-in before native code is entered.

## Scope

Audited files:

- `crates/php_jit/src/lib.rs`
- `crates/php_jit/src/abi.rs`
- `crates/php_jit/src/code_manager.rs`
- `crates/php_jit/src/cranelift_lowering.rs`
- `crates/php_jit/src/helpers.rs`
- `crates/php_jit/src/code_memory.rs`
- `crates/php_vm/src/vm/mod.rs`
- `crates/php_runtime/src/jit_array.rs`
- `crates/php_jit/Cargo.toml`
- `justfile`

Out of scope for Performance:

- persistent native-code cache,
- persistent native-code serialization or cross-process cache ownership,
- JIT calls into arbitrary PHP frames,
- inlined object, array, string, destructor, or standard-library semantics,
- Zend ABI or extension ABI compatibility.

## Status Summary

| Risk | Status | Evidence | Notes |
| --- | --- | --- | --- |
| Executable memory lifecycle | mitigated | `CraneliftCodeManager`, `SharedJitCodeHandle`, code-manager lifetime/eviction tests | One module backs many functions in a bounded generation. Every published handle owns the generation through `Arc`; final drop invokes `JITModule::free_memory` only when no handle can execute it. |
| W^X / Cranelift memory provider | accepted | `JITModule::new(JITBuilder::with_isa(...))` | The feature-gated Cranelift path delegates executable-memory mapping and protection transitions to Cranelift's JIT memory provider. Default builds keep `jit-cranelift` disabled. |
| W^X / `code_memory` allocator | mitigated | `code_memory::CodeMemory` + `executes_native_return_constant_*`, `rejects_empty_code` tests | The VM-owned `code_memory` abstraction (ADR 0019 prereq #1) is the single custom executable-memory allocator. It never leaves a page simultaneously writable and executable in a usable way: on Apple Silicon it maps `MAP_JIT` and toggles the per-thread `pthread_jit_write_protect_np` (write → flip to execute → `sys_icache_invalidate`); on other Unix hosts it maps read/write, copies, then `mprotect`s the range to read/execute (`__clear_cache` on aarch64); unsupported hosts fail closed. It is NOT wired into VM execution — no interpreter path constructs it — so it does not enable a `native_execution` mode. |
| Symbol registry safety | mitigated | `JIT_HELPER_SYMBOLS`, `helper_registry_is_stable`, `helper_registry_layout_summary` tests | Helper names, ids, argument kinds, return kinds, and side-effect flags are centralized. The two exported arithmetic helpers have local `SAFETY:` notes for their unsafe `no_mangle` attributes. |
| ABI layout assumptions | mitigated | `JIT_RUNTIME_ABI_HASH`, `JIT_HELPER_REGISTRY_ABI_HASH`, handle invoke checks | Native handles check the runtime ABI hash before transmuting a raw address into an `extern "C"` function pointer. Stable C-facing metadata uses fixed integer/pointer shapes rather than Rust references. |
| Lifetime of compiled functions | mitigated | generation-bound handle token; invalidation and old-handle tests | Cache retirement removes publication first. Executable memory remains live until all request and worker handles release the generation. |
| Frame/value pointer validity | mitigated | VM helper shims in `crates/php_vm/src/vm/mod.rs` | Native entries receive opaque `usize` pointers only for synchronous calls. VM shims reject null pointers, point at stack-owned prepared values, and never store the pointers after return. |
| Panic behavior | accepted | helper shims avoid explicit panics and return status/fallback codes | Performance native helpers are small Rust functions with explicit null, overflow, guard, and allocation-failure branches. A process-aborting Rust panic or OOM remains outside the Performance recovery model, so fast paths must keep helper logic minimal and deterministic. |
| Side-exit live-state | mitigated | `JitInvokeError::side_exit`, guard report, diff fixtures | Native paths either write a scalar/result pointer and return success or return a status mapped to interpreter fallback counters. They do not resume at an arbitrary native PC or expose partial VM frames. |
| Drop/destructor interactions | mitigated | string concat and property load helpers return `Box<Value>` consumed by VM immediately | Helpers that allocate PHP values transfer ownership with `Box::into_raw` only on success. The VM reconstructs the `Box<Value>` in the same synchronous call path, so Rust drops happen on the VM side after fallback/success accounting. |
| Unsupported fast paths | disabled | eligibility rejects unsupported IR; fixtures exercise fallback | Calls inside loops, mutable arrays, by-reference shapes, magic conversions, broad object paths, and unsupported dynamic behavior stay in the interpreter or direct VM helper path. They are not silently lowered to native code. |
| Platform skips | mitigated | native-target setup returns `JIT_CRANELIFT_REJECT_NATIVE_TARGET`; `verify-cranelift` runs through default-off feature gates | Host ISA setup failures are typed compile rejections rather than panics. The Cranelift addendum gate must fail or skip clearly if the feature cannot be built on the active platform. |

## Unsafe Inventory

All Rust unsafe boundaries in the audited Cranelift surface have local
`SAFETY:` comments:

- `crates/php_jit/src/lib.rs`: native entry invocation uses
  `mem::transmute` after ABI-hash and signature-kind checks.
- `crates/php_jit/src/abi.rs`: the versioned helper table stores an unsafe C
  function pointer because callers supply raw argument/result buffers. The
  default dispatcher rejects null pointers before writing exactly one
  synchronous caller-owned result record and never retains either pointer.
- `crates/php_jit/src/helpers.rs`: `write_checked_result` writes to a
  non-null out pointer; exported arithmetic helper symbols document their
  unsafe `no_mangle` boundary.
- `crates/php_vm/src/vm/mod.rs`: VM helper shims dereference synchronous
  stack-owned value pointers and reconstruct VM-owned boxed result pointers.
- `crates/php_jit/src/cranelift_lowering.rs`: test helper out-pointer writes
  are limited to stack-owned slots passed by JIT trampoline tests.
- `crates/php_jit/src/code_memory.rs`: the custom executable-memory allocator.
  Each `unsafe` block has a local `SAFETY:` comment — `mmap`/`munmap`/`mprotect`
  with checked `MAP_FAILED`/errno handling, the Apple-Silicon
  `pthread_jit_write_protect_np` W^X toggle bracketing the `copy_nonoverlapping`
  into a freshly mapped `mapped_len >= code.len()` region, and
  `sys_icache_invalidate`/`__clear_cache` for i-cache coherence. The tests
  `transmute` the finalized read-execute pointer to an `extern "C" fn() -> i32`
  only over hand-assembled leaf stubs.
- `crates/php_jit/src/code_manager.rs`: generation drop calls
  `JITModule::free_memory`. The final `Arc` can drop only after the manager,
  process cache, and all cloned native handles have released the generation.

Cranelift modules are no longer leaked. The module itself remains confined
behind a generation mutex because Cranelift marks its memory provider and
compiled blobs `Send`, while the module's internal `RefCell` state prevents
direct shared mutation. Immutable finalized entry points are used concurrently.

## Default-Off Check

`jit-cranelift` remains default-off:

```toml
[features]
default = []
jit-cranelift = [...]
```

The runtime also requires explicit native execution opt-in. The no-exec backend
test `cranelift_no_exec_backend_refuses_native_entry_by_default` validates that
the backend refuses native entries when the caller does not pass
`allow_native_execution: true`.

## Validation

that stage validation commands:

```bash
nix develop -c cargo test --workspace --features jit-cranelift
nix develop -c just verify-cranelift
```

Focused lifecycle evidence:

```bash
nix develop -c cargo test -p php_jit --features jit-cranelift cranelift_native_handle_copy_survives_original_handle_drop
nix develop -c cargo test -p php_jit --features jit-cranelift code_manager::tests
```
