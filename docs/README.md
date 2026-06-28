# Phrust Documentation

This directory documents the current Phrust engine, tooling, compatibility
policy, and validation workflow. It is organized as a practical entry point
over the existing subsystem documentation rather than as a chronological record.

## Start Here

- New to the repository: read the top-level [README](../README.md), then
  [First local run](how-to/first-local-run.md).
- Making a code or documentation change: use
  [Validate a change](how-to/validate-a-change.md).
- Working with PHP compatibility tests: use
  [Work with PHPT](how-to/work-with-phpt.md), then the [PHPT guide](phpt/README.md).
- Running the integrated HTTP server: use
  [Run the web server](how-to/run-the-web-server.md).
- Looking for current gaps: start with [Known gaps](known_gaps/README.md),
  [runtime known gaps](runtime-known-gaps.md), [stdlib known gaps](stdlib-known-gaps.md),
  [performance known gaps](performance-known-gaps.md), and
  [PHPT known gaps](phpt/known-gaps.md).

## Documentation Model

Phrust uses a Diataxis-style split:

- Tutorials introduce a workflow end-to-end.
- How-to guides solve specific tasks.
- Reference docs give exact contracts, commands, schemas, and compatibility
  data.
- Explanation docs describe architecture, design decisions, boundaries, and
  tradeoffs.

The source tree still groups many documents by engine subsystem because those
boundaries match the codebase and validation gates. This page provides the
task-oriented route through those subsystem docs.

## Tutorials

Tutorial-style docs should get a contributor from a clean checkout to a useful
result without requiring prior knowledge of the engine internals.

- [First local run](how-to/first-local-run.md)
- [Run the web server](how-to/run-the-web-server.md)
- [Work with PHPT](how-to/work-with-phpt.md)

## How-To Guides

Use these for repeatable tasks.

- [Validate a change](how-to/validate-a-change.md)
- [First local run](how-to/first-local-run.md)
- [Run the web server](how-to/run-the-web-server.md)
- [Work with PHPT](how-to/work-with-phpt.md)

## Reference

Reference documents should be stable, exact, and easy to cite from code,
tests, and CI.

- Compatibility target: [PHP 8.5.7 target](foundation/compatibility-target.md)
  and [ADR 0001](adr/0001-target-php-version.md).
- Layer boundaries: [runtime contract](runtime-contract.md),
  [runtime semantics contract](runtime-semantics-contract.md),
  [frontend/runtime boundary](frontend/runtime-boundary.md),
  [parser/frontend boundary](parser/frontend-boundary.md), and
  [lexer/parser boundary](parser/lexer-parser-boundary.md).
- API surfaces: [API facades](api-facades.md).
- Server configuration and behavior: [server functionality](server-functionality.md)
  and [server architecture](server-architecture.md).
- PHPT infrastructure: [PHPT guide](phpt/README.md),
  [source integrity](phpt/source-integrity.md), [binary discovery](phpt/binary-discovery.md),
  [generated tests](phpt/generated-tests.md), and
  [full PHPT gate](phpt/full-phpt-gate.md).
- Machine-readable known gaps: [known-gap manifests](known_gaps/README.md).
- Callable inventory: [PHP source callables reference](php-src-callables-reference.md).

## Explanation

Use these when you need the design rationale or current architecture before
changing behavior.

- Foundation: [definition of done](foundation/definition-of-done.md),
  [test matrix](foundation/test-matrix.md), [risk register](foundation/risk-register.md),
  and [license/copying policy](foundation/license-and-copying-policy.md).
- Lexer and parser: [lexer architecture](lexer/lexer-architecture.md),
  [token model](lexer/token-model.md), [parser architecture](parser/parser-architecture.md),
  and [CST model](parser/cst-model.md).
- Frontend: [semantic frontend architecture](frontend/semantic-frontend-architecture.md),
  [HIR model](frontend/hir-model.md), [declaration model](frontend/declaration-model.md),
  and [semantic known gaps](frontend/semantic-known-gaps.md).
- Runtime and VM: [runtime reference](runtime-reference.md),
  [runtime VM structure](runtime-vm-structure.md), [runtime values](runtime-values.md),
  [runtime semantics status](runtime-semantics-status.md), and
  [runtime semantics coverage matrix](runtime-semantics-coverage-matrix.md).
- Standard library: [standard library](stdlib-standard-library.md),
  [extension coverage](stdlib-extension-coverage.md),
  [function coverage](stdlib-function-coverage.md), and
  [standard library roadmap](stdlib-roadmap.md).
- Performance: [performance methodology](performance-methodology.md),
  [performance results](performance-results.md), [performance runtime](performance-runtime.md),
  [bytecode cache](performance-bytecode-cache.md), and
  [Cranelift known gaps](performance-cranelift-known-gaps.md).
- Server: [server architecture](server-architecture.md),
  [server functionality](server-functionality.md), and
  [server known gaps](server-known-gaps.md).
- Architecture decisions: [ADR directory](adr/).
- Research notes: [research directory](research/).

## Reports And Audits

Reports and audits should summarize current status, evidence, and remaining
gaps. They should not describe how a change was produced. Keep them concise and
link to commands, fixtures, and known-gap IDs when possible.

Current report families include:

- `docs/phpt/reports/`
- `docs/*final-audit.md`
- `docs/*known-gaps.md`
- `docs/performance-*results.md`

## Maintenance Rules

- Prefer task guides for user workflows and subsystem docs for architecture.
- Keep generated run artifacts under `target/`; do not commit raw run output.
- Keep original `php-src` tests read-only and local reference checkouts under
  `third_party/`.
- Add or update a known-gap ID when documenting unsupported current behavior.
- Update this index when adding a new documentation family or a new primary
  workflow.
