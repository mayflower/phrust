# Phase 5 Generators and Fibers

Phase 5 models generators and fibers as runtime objects owned by
`php_runtime` and resumed by `php_vm`. They consume Phase 3 HIR and Phase 4 IR;
there is no alternate parser, bytecode format, or string-matched execution path.

## Generators

Generator functions are identified during HIR/IR lowering. Calling a generator
function creates a `Value::Generator(GeneratorRef)` without running the body.
The VM executes the body only when the generator is resumed by `foreach` or a
visible `Generator` method.

`GeneratorRef` stores:

- raw function ID and call arguments;
- lifecycle state: Created, Running, Suspended, Closed, or Errored;
- current key/value pair;
- optional return value after normal completion.

The VM stores the non-public continuation state: frame, current block,
instruction offset, foreach iterator state, exception handlers, and pending
finally/return/throw control. `send()` writes the supplied value into the saved
`yield` expression result. `throw()` injects the supplied Throwable-compatible
object through the saved handler stack. `yield from` delegates to arrays and
Phase 5 generator objects and forwards yielded keys, yielded values, and the
delegate return value.

Known generator boundaries remain explicit:

- by-reference generator yields are rejected with
  `E_PHP_RUNTIME_GENERATOR_BY_REF_YIELD_GAP`;
- delegation to Iterator/SPL objects is outside Phase 5;
- generator methods and closures beyond the current free-function MVP are not
  complete;
- exact Zend diagnostics, destructor timing, and all exception/finally
  interaction edges are Phase 6+ work.

## Fibers

`new Fiber($callable)` creates `Value::Fiber(FiberRef)` with a stored structured
callable. The runtime tracks NotStarted, Running, Suspended, Terminated, and
Errored states. The VM owns the continuation stack needed by
`Fiber::suspend()`, `start()`, `resume()`, `throw()`, and `getReturn()`.

Suspension stores all active frames that must be restored to continue the fiber,
including nested function/static/method/closure calls covered by the fixtures.
`resume($value)` writes `$value` into the suspended `Fiber::suspend()`
expression. `throw($exception)` resumes by injecting a Throwable-compatible
object. Normal callable completion records the return value and moves the fiber
to Terminated; uncaught errors move it to Errored.

Known fiber boundaries remain explicit:

- wider stack switching across include, magic-method, property-hook, destructor,
  and generator edges needs more fixtures before it is claimed complete;
- public `Fiber::getCurrent()` is not implemented;
- public GC/refcount timing for suspended stacks is not Zend-compatible;
- method failures are represented by deterministic diagnostics, not complete
  catchable `FiberError` object parity.

## Public APIs

Phase 6 code should treat these Rust APIs as the current integration surface:

- `php_runtime::GeneratorRef` and `GeneratorState` for generator identity,
  current key/value, return value, and lifecycle state;
- `php_runtime::FiberRef` and `FiberState` for fiber identity, callable storage,
  return value, and lifecycle state;
- `php_runtime::Value::{Generator,Fiber}` for value transport, type checks,
  GC scanning, and debug output;
- `php_vm::VmOptions::trace_runtime` and VM runtime trace events for
  deterministic failure minimization.

The continuation structs inside `php_vm` are intentionally private. Phase 6 may
extend their contents, but external layers should not depend on a serializable
or stable continuation representation until an ADR introduces one.

## Performance and Safety Notes

Generator and fiber continuations clone current frame state. That keeps the MVP
simple and deterministic, but it is a performance-critical area for Phase 6
framework execution. Likely optimizations are compact continuation frames,
shared immutable call metadata, and reduced cloning of foreach iterator state.

No real native stack switching is performed. Fiber scheduling is cooperative
inside the VM interpreter loop, which avoids unsafe stack manipulation but means
extensions and blocking I/O cannot observe Zend-like fiber behavior yet.

Suspended stacks must remain GC roots. Phase 5 has explicit root categories for
generator and fiber stacks; Phase 6 must keep those roots precise when adding
real cycle collection and public `gc_*` APIs.
