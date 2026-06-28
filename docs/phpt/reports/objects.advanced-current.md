# objects.advanced Current Focus Report

Generated from Prompt Pack Branch 2.

## Scope

This branch owns advanced object behavior only:

- `objects.magic`: `__get`, `__set`, `__isset`, `__unset`, `__call`, `__callStatic`, `__invoke`, `__toString`
- `objects.clone`: clone identity, independent properties, `__clone`, and public clone-with MVP behavior
- `objects.traits`: focused trait method import, alias, and simple precedence
- `objects.enums`: unit/backed enum cases, `cases()`, `from()`, `tryFrom()`, and enum methods

Object-core construction, property visibility, and basic method dispatch failures remain owned by `phpt/b3-objects-core`.

## Selected PHPTs

| Submodule | Count | Selected PHPTs |
| --- | ---: | --- |
| `objects.magic` | 8 | `magic-get`, `magic-set`, `magic-isset`, `magic-unset`, `magic-call`, `magic-call-static`, `magic-invoke`, `magic-to-string` |
| `objects.clone` | 7 | `clone-identity`, `clone-independent-properties`, `clone-magic-method`, `clone-with-public-property`, `clone-with-typed-property`, `clone-with-type-mismatch`, `clone-with-unsupported-private` |
| `objects.traits` | 3 | `trait-method`, `trait-method-alias`, `trait-method-precedence` |
| `objects.enums` | 5 | `enum-unit-case`, `enum-backed-case`, `enum-cases`, `enum-from-tryfrom`, `enum-method` |

## Current Gate Status

Prompt 2.1 split gates:

| Gate | Reference | Target | Status |
| --- | ---: | ---: | --- |
| `nix develop -c just phpt-dev-module MODULE=objects.magic` | 8 PASS | 8 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.clone` | 7 PASS | 7 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.traits` | 3 PASS | 3 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.enums` | 5 PASS | 5 PASS | PASS |
| `nix develop -c just verify-phpt` | n/a | n/a | PASS |

All four submodule gates use curated selected manifests under
`tests/phpt/manifests/modules/` and isolated Prompt 2.1 work output under
`target/phpt-work-b3-objects-advanced`.

Prompt 2.2 through Prompt 2.5 acceptance:

| Prompt | Gate | Status |
| --- | --- | --- |
| 2.2 magic | `nix develop -c cargo test -p php_vm` | PASS |
| 2.2 magic | `nix develop -c just phpt-dev-module MODULE=objects.magic` | PASS |
| 2.3 clone | `nix develop -c cargo test -p php_runtime object` | PASS |
| 2.3 clone | `nix develop -c cargo test -p php_vm` | PASS |
| 2.3 clone | `nix develop -c just phpt-dev-module MODULE=objects.clone` | PASS |
| 2.4 traits | `nix develop -c cargo test -p php_ir` | PASS |
| 2.4 traits | `nix develop -c cargo test -p php_vm` | PASS |
| 2.4 traits | `nix develop -c just phpt-dev-module MODULE=objects.traits` | PASS |
| 2.5 enums | `nix develop -c cargo test -p php_runtime object` | PASS |
| 2.5 enums | `nix develop -c cargo test -p php_vm` | PASS |
| 2.5 enums | `nix develop -c just phpt-dev-module MODULE=objects.enums` | PASS |

Prompt 2.6 pre-closeout gates that can run before `phpt/b3-objects-core` is
available:

| Gate | Reference | Target | Status |
| --- | ---: | ---: | --- |
| `nix develop -c just phpt-dev-module MODULE=objects.magic` | 8 PASS | 8 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.clone` | 7 PASS | 7 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.traits` | 3 PASS | 3 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.enums` | 5 PASS | 5 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.classes` | 200 PASS | 164 PASS / 36 FAIL | FAIL |
| `nix develop -c just verify-runtime` | n/a | n/a | PASS |
| `nix develop -c just verify-phpt` | n/a | n/a | PASS |

## Prompt 2.2 Magic Method MVP

Supported by the selected gate:

- property magic: `__get`, `__set`, `__isset`, `__unset`
- method magic: `__call`, `__callStatic`
- callable/string magic: `__invoke`, `__toString`
- VM magic dispatch routes through the normal call path and has deterministic
  recursion diagnostics for selected property and method recursion cases.

Remaining gaps:

- serialization magic
- `__debugInfo`
- recursive by-reference magic lvalues
- Reflection of magic methods

## Prompt 2.3 Clone and Clone-With MVP

Supported by the selected gate:

- shallow clone with a distinct object identity
- independent property map after clone
- `__clone`
- PHP 8.5 clone-with public property replacement
- simple typed public replacement checks and selected mismatch diagnostics

Remaining gaps:

- private/protected clone-with replacements stay explicit unsupported cases
- readonly clone-with matrix
- full property-hook clone-with matrix
- serialization magic

## Prompt 2.4 Traits MVP

Supported by the selected gate:

- importing trait methods into classes
- simple trait method alias
- simple `insteadof` precedence
- method origin metadata preservation for later Reflection work

Remaining gaps:

- trait properties
- trait constants
- nested trait uses
- exhaustive conflict diagnostics
- Reflection trait APIs

## Prompt 2.5 Enums MVP

Supported by the selected gate:

- unit enum case singleton behavior
- backed enum case values
- `cases()`
- `from()`
- `tryFrom()`
- enum methods

Remaining gaps:

- enum serialization
- `ReflectionEnum`
- exhaustive invalid enum diagnostics

## Prompt 2.6 Closeout Status

The final closeout is pending because no completed `phpt/b3-objects-core`
branch exists locally or on `origin` at the time of this report. The required
pre-closeout branch search found no branch matching:

- `phpt/b3-objects-core`
- `b3-objects-core`
- `objects-core`

Remaining closeout work after that branch exists:

- rebase or merge the completed `phpt/b3-objects-core` branch without undoing
  object-core fixes
- rerun the four advanced submodule gates
- rerun `nix develop -c just phpt-dev-module MODULE=objects.classes`
- run `nix develop -c just verify-runtime`
- run `nix develop -c just verify-phpt`
- update `docs/phpt/modules/objects.classes.md` with the aggregate impact

Current aggregate `objects.classes` impact before the core branch merge:

- reference: 200 PASS
- target: 164 PASS, 36 FAIL
- dominant remaining areas: static properties and property references,
  serialization magic, iterator/autoload behavior, class constants, exception
  catch-type gaps, eval declaration merging, and Reflection-adjacent paths
- target summary:
  `target/phpt-work-b3-objects-advanced/module-runs/objects.classes/target/summary.md`

## Current Blockers

No blockers remain for Prompts 2.1 through 2.5. Prompt 2.6 remains blocked on
the missing completed `phpt/b3-objects-core` branch. Basic construction,
properties, methods, and visibility remain owned by that branch and were not
changed here.

## Non-Scope

- object-core failures
- SPL implementation
- Reflection implementation
- SAPI
- serialization magic
- trait properties/constants/nested uses
- exhaustive enum diagnostics
