# ADR 0783: Cranelift Side-Exit Model

Date: 2026-06-23.

Status: Accepted for Performance Cranelift addendum Work item.14.

## Context

The Cranelift backend now has constrained native execution for constant integer
returns and helper-call integer arithmetic. Helper calls can detect runtime
conditions, such as integer overflow, where native output must not be trusted.
Before later work items add inline fast paths and guards, the VM/JIT boundary needs
a stable side-exit ABI, reporting model, and interpreter fallback rule.

## Decision

JIT side exits are represented with a stable `SideExitReason` enum in
`php_jit::abi`:

| Reason | Code | Report spelling | Meaning |
| --- | ---: | --- | --- |
| `TypeMismatch` | 1 | `type_mismatch` | Runtime value type does not match the compiled specialization. |
| `Overflow` | 2 | `overflow` | Checked arithmetic or conversion overflowed. |
| `UnsupportedValue` | 3 | `unsupported_value` | Runtime value shape is outside the compiled subset. |
| `GuardFailed` | 4 | `guard_failed` | A generated guard rejected the current frame. |
| `HelperStatus` | 5 | `helper_status` | A runtime helper returned a non-OK status. |
| `ExceptionPending` | 6 | `exception_pending` | PHP exception or error state is pending. |
| `AbiMismatch` | 7 | `abi_mismatch` | VM/JIT ABI hash or call boundary mismatch. |

The native invocation API maps every recoverable `JitInvokeError` to
`JitSideExit` metadata before falling back to the interpreter. VM counters record
both the total side-exit count and a per-reason JSON map. The compact
`php-vm run --jit-stats=json` payload exposes the same per-reason map as
`side_exit_reasons`.

FPE-16 adds VM-owned report-only deoptimization metadata in `php_vm::deopt`.
`VmDeoptReason` keeps the Cranelift reason codes above as its code 1 through 7
prefix, then extends the VM-level model with call-frame, reference/COW,
foreach, pending-finally, generator/fiber, output-buffer, and unsupported
control-flow reasons. Future optimized tiers should consume or extend the
VM-owned metadata instead of inventing tier-specific resume records.

## Resume Point Reporting

`JitSideExit` carries optional `resume_block` and `resume_instruction` values
using the existing IR `BlockId` and `InstrId` types. The C-facing `JitCExit`
record carries the same information as `u32` fields with `u32::MAX` as the
"not available" sentinel.

For Work item.14, helper-call side exits do not resume in the middle of a
compiled region. The VM discards the native out slot and re-runs the selected
function through the interpreter from the normal entry point. Future inline
fast paths may provide a precise resume block/instruction only when the
interpreter can continue from that point with identical live state.

If the backend cannot prove a precise resume point and live-state mapping, the
region is not eligible for JIT execution. It must not compile a speculative
native path that would require an unsafe or guessed resume.

## Live-Slot Rules

The interpreter frame remains the authoritative PHP state.

- Parameters and locals already committed to the interpreter frame before the
  native call remain valid and VM-owned after a side exit.
- JIT code may write durable frame slots before an exit only when the write is
  in the same order and with the same value the interpreter would have produced.
- The current helper-call subset writes only native temporary stack slots and
  the final out pointer. The out pointer is valid only after native status is
  OK; on non-OK status it is discarded.
- Native stack temporaries, Cranelift SSA values, helper scratch memory, and
  partially computed out values are invalid after a side exit unless explicitly
  materialized in a future side-exit record.
- Exception or fatal-error state must be reported through a side-exit reason or
  a later runtime diagnostic channel before interpreter fallback observes it.

No OSR is introduced by this work item. Side exits return to interpreter fallback
only; they do not transfer execution into another compiled region.

## Reporting And Validation

`just cranelift-guard-report` runs a deterministic side-exit fixture that
overflows inside a checked helper call, verifies JIT off/on output parity, and
writes `target/performance/cranelift/guard-report.json`. The report records
`side_exits`, `side_exit_reasons`, `guard_failures`, and bailout counters.

The required validation gates are:

```bash
nix develop -c just jit-cranelift-diff
nix develop -c just cranelift-guard-report
nix develop -c cargo test --workspace --features jit-cranelift
```

## Consequences

Later guard, inline-arithmetic, loop, array, string, property, and method
work items can add precise side-exit points without changing the stable reason
spelling or counter schema. Unsupported or ambiguous live state remains a
compile-time ineligibility reason instead of a runtime resume guess.
