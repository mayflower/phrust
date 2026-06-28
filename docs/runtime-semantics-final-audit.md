# Runtime semantics Final Audit

Runtime semantics closes with executable semantics, fixture evidence, and explicit gaps.
The runtime path remains:

```text
php_lexer -> php_syntax -> php_ast -> php_semantics/HIR -> php_ir -> php_runtime -> php_vm -> php_vm_cli
```

No second lexer, parser, semantic frontend, or source-string execution path was
introduced.

## Final Gate

The required final gates are:

```bash
nix develop -c just verify-foundation
nix develop -c just verify-lexer
nix develop -c just verify-frontend
nix develop -c just verify-frontend
nix develop -c just verify-runtime
nix develop -c just verify-runtime
nix develop -c cargo test --workspace
```

`verify-runtime` includes formatting, Clippy, workspace tests, Runtime
verification, Runtime semantics fixture gates, the Runtime semantics diff harness, PHPT smoke
allowlist checks, hardening lints, the devshell toolchain audit, and final
documentation checks.

## Evidence Map

| Area | Evidence |
| --- | --- |
| Runtime contract | `docs/runtime-semantics-contract.md` |
| Coverage matrix | `docs/runtime-semantics-coverage-matrix.md` |
| Known gaps | `docs/runtime-semantics-known-gaps.md` |
| References and COW | `docs/runtime-semantics-reference-cow.md`, `docs/adr/0027-runtime-semantics-slot-reference-cow.md` |
| Arrays and foreach | `docs/runtime-semantics-array-semantics.md`, `docs/runtime-semantics-foreach-semantics.md`, `docs/adr/0028-runtime-semantics-array-element-reference-foreach.md` |
| Objects, traits, enums, hooks | `docs/runtime-semantics-object-semantics.md`, `docs/adr/0029-runtime-semantics-object-model-traits-enums-hooks.md` |
| Generators and fibers | `docs/runtime-semantics-generators-fibers.md`, `docs/adr/0030-runtime-semantics-generator-fiber-control-flow.md` |
| Reflection and attributes | `docs/runtime-semantics-reflection-attributes.md` |
| Destructors and GC | `docs/adr/0025-runtime-semantics-destructor-queue.md`, `docs/adr/0026-runtime-semantics-gc-skeleton.md` |
| Unsafe and hardening audit | `docs/runtime-semantics-unsafe-audit.md` |
| Standard library roadmap | `docs/stdlib-roadmap.md` |

## Docs and CI Consistency

- `README.md` points to the Runtime semantics contract, known-gap catalog, coverage
  matrix, unsafe audit, and standard library roadmap.
- `AGENTS.md` keeps Runtime semantics boundaries and requires `verify-runtime` before
  closing Runtime semantics work.
- `.github/workflows/runtime-semantics.yml` runs `nix develop -c just verify-runtime` and
  uploads Runtime semantics/Runtime report artifacts when present.
- `scripts/verify-runtime.sh` asserts final docs, PHPT allowlist categories,
  regression metadata, and minimization tooling.

## Closure Criteria

Runtime semantics is complete when the final gates above pass. Any red gate must be
classified as an existing baseline issue, a new regression, or an allowed
known gap before the layer is considered closed. New regressions are not
accepted as Runtime semantics completion.
