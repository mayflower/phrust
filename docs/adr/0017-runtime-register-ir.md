# ADR 0017: Runtime Register IR

## Status

Accepted

## Context

Runtime needs an executable contract between the Semantic frontend semantic frontend and
the VM. The contract must be deterministic, snapshot-friendly, and independent
from Zend opcode numbers, Zend ABI details, and extension internals.

## Decision

Use a small register-based IR with typed IDs, explicit basic blocks, explicit
terminators, source maps, and stable diagnostic IDs for unsupported features.
The IR is the local execution contract for `php_vm`; it is not Zend bytecode.

## Consequences

- IR snapshots can be reviewed without requiring a PHP reference binary.
- Lowering can classify unsupported PHP forms before execution.
- Compatibility work in Runtime semantics must map PHP behavior onto this IR or revise
  the IR intentionally with versioned snapshots.
