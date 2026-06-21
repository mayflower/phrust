# Semantic Known Gaps

Known gaps are explicit, reviewed differences between Phase 3 behavior and the
pinned PHP 8.5.7 reference.

Known gaps are tracked only after a fixture demonstrates the specific
difference. New unrelated mismatches must still fail the diff.

The machine-readable source of truth is
`fixtures/semantic/known_gaps.toml`.

```toml
[[gap]]
fixture = "fixtures/semantic/control_flow/goto-invalid-known-gap.php"
reason = "Full PHP goto boundary validation deferred until CFG pass."
phase = "phase3"
expected_resolution_phase = "phase4-control-flow"

[[gap]]
fixture = "fixtures/semantic/declarations/duplicate-class-invalid.php"
reason = "Rust performs a deterministic same-file duplicate class declaration check that PHP lint defers until compile/runtime loading."
phase = "phase3"
expected_resolution_phase = "phase4-declaration-linking"
```

## Entry Format

```toml
[[gap]]
fixture = "fixtures/semantic/path/example.php"
reason = "Concrete reason"
phase = "phase3"
expected_resolution_phase = "phase4-or-phase5"
```

Known gaps must be narrow enough that new unrelated mismatches still fail the
diff.

`scripts/compare_semantic_acceptance.py` only reports `KnownGap` when a fixture
has an explicit TOML entry and the observed Rust/reference acceptance differs.
Unexpected differences remain `RustAcceptsReferenceRejects` or
`RustRejectsReferenceAccepts`.

## Deferred Runtime Effects

Include, include_once, require, require_once, and eval are intentionally not
known-gap entries when PHP lint accepts the source. Phase 3 lowers them to HIR
with deferred-effect metadata and note diagnostics instead of executing them.

- Include paths may be dynamic and cannot be resolved safely at analysis time.
- Include and require execute code in the current runtime scope.
- Include and require can define functions, classes, and constants.
- Eval requires runtime parsing plus security and scope rules that belong to
  later phases.
