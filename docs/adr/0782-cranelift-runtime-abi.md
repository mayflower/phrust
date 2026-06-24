# ADR-0782: Cranelift Runtime ABI

## Status

Accepted for the Performance Cranelift addendum.

## Context

The safe Rust-facing JIT boundary already models VM handles, frame views,
opaque heap values, callouts, bailouts, and exception markers. Native code
cannot use those Rust enums and heap-backed `String`/`Vec` records directly.
Work item.05 therefore needs a separate C-compatible runtime ABI that can be
layout-tested and versioned.

## Decision

`php_jit::abi` defines a narrow `repr(C)` boundary:

- `JIT_RUNTIME_ABI_VERSION`
- `JIT_RUNTIME_ABI_HASH`
- `JitCValueTag`
- `JitCValue`
- `JitCFrameView`
- `JitCExitTag`
- `JitCExit`

Opaque VM handles use `repr(transparent)` wrappers over non-zero integer
handles. The C ABI records use integer tags, integer payloads, and reserved
fields. They do not expose Rust references, Rust enum layouts with data
payloads, `String`, `Vec`, refcount internals, GC cells, COW storage, frame
borrows, or executable pointers.

## Layout Policy

The ABI layout is covered by `php_jit` unit tests:

```bash
nix develop -c cargo test -p php_jit c_abi_layout
```

The current layout expectations are:

| Type | Size | Alignment |
| --- | --- | --- |
| `JitOpaqueHandle` | 8 | 8 |
| `JitCValueTag` | 4 | 4 |
| `JitCValue` | 24 | 8 |
| `JitCFrameView` | 32 | 8 |
| `JitCExitTag` | 4 | 4 |
| `JitCExit` | 48 | 8 |

Any layout or tag change must update `JIT_RUNTIME_ABI_HASH`, tests, and this
ADR in the same work item.

## Exit Semantics

`JitCExit` can represent:

- normal return;
- bailout;
- PHP exception/error marker;
- runtime helper call request.

Resume points are encoded as block and instruction ids. `u32::MAX` means no
resume point is available. Later side-exit work items may refine reason-code
registries, but the ABI shape is already fixed and test-covered.

## Consequences

Cranelift lowering and native-entry work items can target stable C-compatible
records while the VM continues to own all PHP runtime state. The safe Rust
descriptors remain useful inside the engine, but they are not the native
runtime ABI.
