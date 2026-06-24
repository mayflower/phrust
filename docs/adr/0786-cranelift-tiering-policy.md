# ADR 0786: Cranelift Tiering Policy

Date: 2026-06-23

## Status

Accepted for Performance.

## Context

Performance Cranelift execution must stay conservative. Compiling on the first
call makes correctness fixtures convenient, but it is not a suitable default
policy for normal execution because cold functions pay compile cost before
there is evidence that native execution is useful.

The VM already records request-local function entries, loop backedges, guard
failures, blacklist decisions, and JIT counters. Work item.29 connects those
signals into the first explicit Cranelift tiering policy.

## Decision

The default Cranelift tiering policy is threshold based:

- `--jit-threshold=N` sets the minimum function-entry count before a function
  may compile. The default is `8`.
- `--tiering-loop-threshold=N` remains available as an optional backedge
  hotness signal.
- Guard failures continue to feed the request-local stability score and the
  process-local JIT blacklist. Blacklisted regions fall back to the interpreter.
- `--jit-max-functions=N` caps the number of functions compiled in one request.
- `--jit-max-compile-us=N` caps cumulative native compile time in one request.
- `--jit-eager` is a test-only convenience mode that admits the first eligible
  call immediately.

Compile-budget rejections and blacklist rejections never change PHP-visible
semantics. They skip native compilation or dispatch and resume through the
interpreter.

## Reporting

VM counter JSON and compact `--jit-stats=json` report tiering decisions:

- `tiering_cold_functions`
- `tiering_hot_functions`
- `tiering_eager_functions`
- `tiering_blacklist_rejections`
- `tiering_budget_rejections`

The benchmark matrix records `tiering_mode` per row. Existing fast-path rows
use eager mode to keep fixture validation focused. Dedicated threshold rows
prove that a cold fixture does not compile and a hot fixture does compile.

## Consequences

The default CLI no longer compiles eligible Cranelift functions on their first
call. Tests and validation gates that intentionally need first-call compilation
must pass `--jit-eager`.

This policy is request-local in Performance. A future process-global compile cache
or persistent policy must be introduced by a later ADR and must preserve the
same fallback semantics.
