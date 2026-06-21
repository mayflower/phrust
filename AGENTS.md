# Project Guidelines

## Repository Workflow

- Inspect the repository before changing files.
- Use `nix develop -c ...` for validation commands.
- Complete every change with relevant checks and report skipped checks clearly.
- If a check cannot run because of missing network, missing reference binaries,
  or platform support, report the skipped check and exact reason.
- Do not silently skip checks.
- Keep scripts deterministic and provide clear error messages.
- Use `bash` scripts with `set -euo pipefail`.
- Make script files executable when they are added.
- Update documentation together with tooling changes.
- In a dirty worktree, stage only files intentionally changed for the current
  task and never revert unrelated user changes.

## Reference Target

- PHP series: `8.5`
- PHP version: `8.5.7`
- Git tag: `php-8.5.7`
- Repository: `https://github.com/php/php-src.git`

Do not automatically update the target PHP version without a new ADR.

## Scope Boundaries

- Do not implement VM, runtime values, JIT, extensions, or Zend ABI emulation
  unless the user explicitly asks for that layer.
- Do not implement AST/HIR lowering, name resolution, compile-time semantics,
  constant expression evaluation, type checking, bytecode generation, or IR
  generation inside the parser/CST layer.
- Parser and CST work must reuse the existing lexer. Do not introduce a second
  lexer.
- Do not hardcode numeric PHP token values.
- Compare reference behavior by token names, token text, diagnostics, and
  source positions rather than raw numeric token IDs.
- Preserve byte-based spans as the source of truth. Treat line and column as
  derived display information.
- Public lexer and parser APIs must not panic on invalid input.
- Reference-dependent checks must skip clearly when no PHP reference binary is
  available and must be strict when `REFERENCE_PHP` is explicitly set.
- Do not commit generated reports under `target/`.
- Do not commit extracted `php-src` corpus files or a vendored `php-src` copy.
- Keep local reference checkouts under `third_party/`.

## Future Work Boundaries

- Semantic work should consume `php_syntax` CST APIs and produce separate
  declaration tables, typed views, and semantic diagnostics.
- Parser diagnostics and semantic diagnostics should remain separate so parser
  acceptance stays comparable with the PHP lint oracle.
- Any execution layer must be introduced as a new bounded layer with its own
  validation gates and must not change lexer/parser contracts opportunistically.
- New tools should prefer existing source maps, token kinds, CST ranges, and
  fixture harnesses over adding parallel representations.

## Phase 3 Semantic Frontend Boundaries

- Phase 3 work starts from `docs/phase-3/phase-3-definition-of-done.md`.
- Add typed AST views in a dedicated `php_ast` layer, not in `php_syntax`.
- Add HIR, declarations, scopes, name resolution, type lowering,
  constant-expression validation, attribute metadata, and semantic diagnostics
  in `php_semantics`.
- Keep `php_frontend_cli` as a consumer of `php_semantics`; do not add a second
  parser inside the CLI.
- Include, require, eval, function lookup fallback, attribute instantiation, and
  autoload-sensitive behavior must be represented as deferred metadata or known
  gaps, not executed.
- Every semantic diagnostic ID needs a fixture or an explicit reserved/known-gap
  note before it is considered complete.
- Phase 3 must preserve Phase 0, Phase 1, and Phase 2 validation commands.

## Phase 4 Runtime and VM Boundaries

- Phase 4 work starts from `docs/phase4-runtime-contract.md`.
- Bytecode/IR, VM, and runtime work must consume Phase 3 HIR and semantic
  metadata through `php_semantics`; do not add a second lexer, parser, or
  semantic frontend.
- Keep `php_syntax` and `php_semantics` responsible for syntax and compile-time
  frontend diagnostics. Runtime diagnostics must live in the Phase 4 runtime/VM
  layer.
- Do not implement a full PHP standard library, Zend extension ABI, FPM/SAPI,
  Opcache, quickening, inline caches, or JIT in Phase 4.
- Unsupported runtime features must produce deterministic diagnostics or known
  gaps. Do not silently return plausible but incorrect results.
- Phase 4 must preserve Phase 0, Phase 1, Phase 2, and Phase 3 validation
  commands.

## Validation Commands

- Use the narrowest relevant check while iterating.
- Use `nix develop -c just help` to discover the current canonical gates.
- Before finishing foundation, reference-tooling, lexer, parser, or CST work,
  run the strongest relevant verification target available in `just help`.
- Parser fixture, diff, and roundtrip gates should be run when available.
- Before finishing Phase 3 work, run `nix develop -c just verify-phase3` once
  that target exists. Until then, run the strongest implemented gates for the
  current slice and clearly report missing Phase 3 targets.
- For Phase 3 semantic changes, also prefer the narrow relevant gate first:
  `just semantic-fixtures`, `just semantic-diff`, or
  `just frontend-snapshots`.
- Before finishing Phase 4 work, run `nix develop -c just verify-phase4` once
  that target exists. Until then, run the strongest implemented gates for the
  current slice and clearly report missing Phase 4 targets.

## Codex Operating Profile

- Preferred launch command:

```bash
codex -p phrust-engine --cd /Volumes/CrucialMusic/src/phrust
```

- The matching profile is `~/.codex/phrust-engine.config.toml`.
- Keep work vertical and auditable: requirement mapping, implementation,
  focused tests, then the relevant `nix develop -c just ...` gate.

## Commit Message Rules

- Use conventional commits: `type(scope): description`.
- Keep the first line under 72 characters.
- Use imperative mood.
- Never mention Codex, Anthropic, assistants, or assisted development in commit
  messages.
