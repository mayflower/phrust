//! Phase 4 interpreter VM boundary.
//!
//! This crate will own compiled units, frames, registers, dispatch, calls,
//! control flow, exceptions, includes, tracing, and VM results. Prompt 01 keeps
//! it as a compile-tested skeleton only.

pub mod compiled_unit;
pub mod frame;
pub mod include;
pub mod todo_phase4;
pub mod vm;

pub use compiled_unit::CompiledUnit;
pub use frame::{CallStack, Frame, RegisterFile};
pub use include::{IncludeLoader, LoadedInclude};
pub use todo_phase4::{Phase4VmTodo, vm_skeleton_status};
pub use vm::{Vm, VmOptions, VmResult};

#[cfg(test)]
mod tests {
    use super::{Phase4VmTodo, vm_skeleton_status};

    #[test]
    fn exposes_prompt01_vm_skeleton() {
        let todo = Phase4VmTodo::new("compiled units, frames, registers, and dispatch");
        assert_eq!(
            todo.area(),
            "compiled units, frames, registers, and dispatch"
        );
        assert_eq!(vm_skeleton_status(), "phase4-vm-skeleton");
        assert_eq!(php_ir::ir_skeleton_status(), "phase4-ir-core-model");
        assert_eq!(
            php_runtime::runtime_skeleton_status(),
            "phase4-runtime-skeleton"
        );
        assert_eq!(
            php_testkit::reference_checkout_path(),
            "third_party/php-src"
        );
    }
}
