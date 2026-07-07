//! Copy-and-patch stencil sequencing over the flat `JitCValue` slot buffer.
//!
//! This is the driver primitive the copy-and-patch tier uses to lower a dense
//! function: each opcode becomes a self-contained stencil that reads its
//! operands from the caller's flat slot buffer, computes, and writes its result
//! back to a slot — the classic template-JIT "value file in memory" model
//! described by the Frame-Local Slot ABI in
//! `docs/research/copy-and-patch-stencil-tier.md`. Chaining steps through the
//! slot buffer (rather than registers) keeps each stencil independent and needs
//! no register allocator; a later pass can promote hot slots to registers.
//!
//! Only the guarded-int-add opcode is lowered today. Every other shape is the
//! full region compiler's job and is rejected there, not emitted here.

use crate::aarch64::{Aarch64Assembler, Cond, X0, X3, X4, X5, X6};
use crate::abi::JitCValueTag;

/// A single guarded PHP integer-add step: `slot[dst] = slot[lhs] + slot[rhs]`.
///
/// Slot indices address the flat `[JitCValue]` buffer the VM marshals in/out
/// around the region call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardedIntAddStep {
    /// Destination slot index (result written here as an `Int`).
    pub dst: u32,
    /// Left operand slot index.
    pub lhs: u32,
    /// Right operand slot index.
    pub rhs: u32,
}

/// Reason a slot-add sequence cannot be emitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotSequenceError {
    /// A slot index whose tag/payload byte offset exceeds the scaled-immediate
    /// range (`imm12`), so it cannot be addressed with a single load/store.
    SlotIndexOutOfRange(u32),
}

/// `JitCValue` is `repr(C)` and 24 bytes: `tag` (u32) at 0, `payload` (u64) at
/// 8, `aux` (u64) at 16. Slot `i` lives at `i * 24`.
const STRIDE: u32 = 24;
const TAG_OFF: u32 = 0;
const PAYLOAD_OFF: u32 = 8;

/// The `tag` word load (`ldr_w` at `slot * 24`, encoding `imm12 = slot * 6`) is
/// the binding scaled-immediate constraint: `slot * 6 <= 4095`. The payload
/// double-word (`imm12 = slot * 3 + 1`) is looser, so this bound covers both.
const MAX_SLOT: u32 = 4095 / 6;

const fn tag_off(slot: u32) -> u32 {
    slot * STRIDE + TAG_OFF
}

const fn payload_off(slot: u32) -> u32 {
    slot * STRIDE + PAYLOAD_OFF
}

/// Emit a native `extern "C" fn(slot_base: *mut JitCValue) -> i32` that applies
/// each guarded int-add step in order over the caller's flat slot buffer.
///
/// Returns `0` when every step succeeded. Returns `1` on a side exit: any
/// operand slot not tagged `Int`, or an addition that overflows `i64`. On a
/// side exit, slots written by already-completed steps keep their results —
/// those steps correspond to earlier opcodes that legitimately ran, so the
/// interpreter resumes at the failing step with the prior locals already
/// updated. (This primitive returns a single generic side-exit code; wiring it
/// into VM dispatch adds the per-step resume program point.)
pub fn emit_guarded_int_add_sequence(
    steps: &[GuardedIntAddStep],
) -> Result<Vec<u8>, SlotSequenceError> {
    const INT_TAG: u16 = JitCValueTag::Int as u16;

    for step in steps {
        for slot in [step.dst, step.lhs, step.rhs] {
            if slot > MAX_SLOT {
                return Err(SlotSequenceError::SlotIndexOutOfRange(slot));
            }
        }
    }

    let mut asm = Aarch64Assembler::new();
    let deopt = asm.new_label();
    for step in steps {
        // Guard both operand tags are Int; a mismatch takes the side exit.
        asm.ldr_w(X3, X0, tag_off(step.lhs));
        asm.cmp_imm_w(X3, INT_TAG);
        asm.b_cond(Cond::NotEqual, deopt);
        asm.ldr_w(X3, X0, tag_off(step.rhs));
        asm.cmp_imm_w(X3, INT_TAG);
        asm.b_cond(Cond::NotEqual, deopt);
        // Load payloads, add with an overflow guard.
        asm.ldr_x(X4, X0, payload_off(step.lhs));
        asm.ldr_x(X5, X0, payload_off(step.rhs));
        asm.adds(X6, X4, X5);
        asm.b_cond(Cond::Overflow, deopt);
        // Write the Int result back to the destination slot.
        asm.movz(X3, INT_TAG);
        asm.str_w(X3, X0, tag_off(step.dst));
        asm.str_x(X6, X0, payload_off(step.dst));
    }
    asm.movz(X0, 0);
    asm.ret();
    asm.bind(deopt);
    asm.movz(X0, 1);
    asm.ret();
    Ok(asm.finish())
}

#[cfg(test)]
mod tests {
    use super::{GuardedIntAddStep, MAX_SLOT, SlotSequenceError, emit_guarded_int_add_sequence};

    #[test]
    fn empty_sequence_emits_only_the_return_epilogue() {
        // movz x0,#0 ; ret ; movz x0,#1 ; ret = four 32-bit instructions.
        let code = emit_guarded_int_add_sequence(&[]).expect("empty sequence emits");
        assert_eq!(code.len(), 4 * 4);
    }

    #[test]
    fn sequence_length_grows_with_each_step() {
        let one = emit_guarded_int_add_sequence(&[GuardedIntAddStep {
            dst: 2,
            lhs: 0,
            rhs: 1,
        }])
        .expect("one step emits");
        let two = emit_guarded_int_add_sequence(&[
            GuardedIntAddStep {
                dst: 2,
                lhs: 0,
                rhs: 1,
            },
            GuardedIntAddStep {
                dst: 4,
                lhs: 2,
                rhs: 3,
            },
        ])
        .expect("two steps emit");
        // Each step emits the same fixed-size stencil, so two steps add exactly
        // one step's worth of instructions over one step.
        assert_eq!(two.len() - one.len(), one.len() - 4 * 4);
        assert!(one.len().is_multiple_of(4) && two.len().is_multiple_of(4));
    }

    #[test]
    fn out_of_range_slot_is_rejected_not_miscompiled() {
        let bad = MAX_SLOT + 1;
        assert_eq!(
            emit_guarded_int_add_sequence(&[GuardedIntAddStep {
                dst: bad,
                lhs: 0,
                rhs: 1,
            }]),
            Err(SlotSequenceError::SlotIndexOutOfRange(bad)),
        );
        // The last addressable slot is accepted.
        assert!(
            emit_guarded_int_add_sequence(&[GuardedIntAddStep {
                dst: MAX_SLOT,
                lhs: 0,
                rhs: 1,
            }])
            .is_ok()
        );
    }
}
