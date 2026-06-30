# Runtime VM Structure

The VM crate keeps the public `php_vm::vm` API stable while splitting the
implementation into a module directory:

- `crates/php_vm/src/vm/mod.rs` owns the interpreter state, dispatch loop, call
  execution, object integration, builtins, include handling, and tests.
- `crates/php_vm/src/vm/prelude.rs` owns private VM implementation imports for
  VM submodules. It is not part of the public API surface.
- `crates/php_vm/src/vm/options.rs` owns `VmOptions` and the public execution
  mode enums re-exported from `php_vm::vm`.
- `crates/php_vm/src/vm/result.rs` owns `VmResult`, `VmControlFlow`, and
  `VmResult` constructor helpers.
- `crates/php_vm/src/vm/arguments.rs` owns user-function argument preparation.
- `crates/php_vm/src/vm/dense_method_dispatch.rs` owns dense bytecode method
  call dispatch helpers.
- `crates/php_vm/src/vm/generator_fiber.rs` owns generator and fiber runtime
  method handling.

This split is structural only. It does not add Zend, function, callable, object,
or standard-library behavior. New VM behavior should continue to enter through
the existing frontend-to-IR-to-VM pipeline and should move into focused VM
submodules only when that reduces ownership ambiguity.

`scripts/verify/source_integrity.py` pins the expected VM module wiring, the
non-empty Rust source rule, and the `VmResult` helper ownership. It also rejects
direct `use super::*` imports from focused VM submodules so broad parent-module
reach-through is visible in one private prelude instead of being duplicated.
