# ADR-0030: Phase 5 Generator and Fiber Control Flow

## Status

Accepted for Phase 5.

## Context

Generators and fibers suspend VM execution and later resume with values or
exceptions. Phase 4 had no continuation model. Phase 5 needs real runtime
objects for visible PHP behavior, but native stack switching or Zend opcode
compatibility would exceed the current interpreter boundary.

## Decision

Generators and fibers are runtime values (`GeneratorRef` and `FiberRef`) with
public lifecycle state. The VM owns private continuation records containing the
frames, block IDs, instruction offsets, foreach state, exception handlers, and
pending control needed to resume execution.

Generators are lazy: a call creates the object, and the body runs on resume.
Fibers are cooperative: `Fiber::suspend()` saves VM frames and returns control
to `start()` or `resume()`. Resume values and injected throwables are written
back into the suspended expression or exception path.

## Alternatives Considered

- Lower generators and fibers to special functions without runtime objects.
  This would not support identity, lifecycle methods, type checks, or GC roots.
- Use native stack switching. That would require unsafe runtime work and still
  would not solve PHP-visible semantics around VM frames.
- Keep all generator/fiber behavior as known gaps. That would block framework
  smoke tests and destructor/GC root modeling.

## Consequences

The VM can now validate visible generator and fiber control flow and scan
suspended stacks as roots. Continuation cloning is intentionally simple and
performance-sensitive. Phase 6 should optimize continuation storage only after
the standard-library and Composer smoke paths identify real hot spots.
