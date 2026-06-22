# Phase 5 Final Audit

Phase 5 closes with executable semantics, fixture evidence, and explicit gaps.
The runtime path remains:

```text
php_lexer -> php_syntax -> php_ast -> php_semantics/HIR -> php_ir -> php_runtime -> php_vm -> php_vm_cli
```

No second lexer, parser, semantic frontend, or source-string execution path was
introduced.

## Final Gate

The required final gates are:

```bash
nix develop -c just verify-phase0
nix develop -c just verify-phase1
nix develop -c just verify-phase2
nix develop -c just verify-phase3
nix develop -c just verify-phase4
nix develop -c just verify-phase5
nix develop -c cargo test --workspace
```

`verify-phase5` includes formatting, Clippy, workspace tests, Phase 4
verification, Phase 5 fixture gates, the Phase 5 diff harness, PHPT smoke
allowlist checks, hardening lints, the devshell toolchain audit, and final
documentation checks.

## Evidence Map

| Area | Evidence |
| --- | --- |
| Runtime contract | `docs/phase5-runtime-contract.md` |
| Coverage matrix | `docs/phase5-coverage-matrix.md` |
| Known gaps | `docs/phase5-known-gaps.md` |
| References and COW | `docs/phase5-reference-cow.md`, `docs/adr/0027-phase5-slot-reference-cow.md` |
| Arrays and foreach | `docs/phase5-array-semantics.md`, `docs/phase5-foreach-semantics.md`, `docs/adr/0028-phase5-array-element-reference-foreach.md` |
| Objects, traits, enums, hooks | `docs/phase5-object-semantics.md`, `docs/adr/0029-phase5-object-model-traits-enums-hooks.md` |
| Generators and fibers | `docs/phase5-generators-fibers.md`, `docs/adr/0030-phase5-generator-fiber-control-flow.md` |
| Reflection and attributes | `docs/phase5-reflection-attributes.md` |
| Destructors and GC | `docs/adr/0025-phase5-destructor-queue.md`, `docs/adr/0026-phase5-gc-skeleton.md` |
| Unsafe and hardening audit | `docs/phase5-unsafe-audit.md` |
| Phase 6 handoff | `docs/phase6-handoff.md` |

## Docs and CI Consistency

- `README.md` points to the Phase 5 contract, known-gap catalog, coverage
  matrix, unsafe audit, and Phase 6 handoff.
- `AGENTS.md` keeps Phase 5 boundaries and requires `verify-phase5` before
  closing Phase 5 work.
- `.github/workflows/phase5.yml` runs `nix develop -c just verify-phase5` and
  uploads Phase 5/Phase 4 report artifacts when present.
- `scripts/verify-phase5.sh` asserts final docs, PHPT allowlist categories,
  regression metadata, and minimization tooling.

## Closure Criteria

Phase 5 is complete when the final gates above pass. Any red gate must be
classified as an existing baseline issue, a new regression, or an allowed
known gap before the phase is considered closed. New regressions are not
accepted as Phase 5 completion.
