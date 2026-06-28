# objects.advanced Current Focus Report

## Scope

This module owns advanced object behavior only:

- `objects.magic`: `__get`, `__set`, `__isset`, `__unset`, `__call`, `__callStatic`, `__invoke`, `__toString`
- `objects.clone`: clone identity, independent properties, `__clone`, and public clone-with MVP behavior
- `objects.traits`: focused trait method import, alias, and simple precedence
- `objects.enums`: unit/backed enum cases, `cases()`, `from()`, `tryFrom()`, and enum methods

Object-core construction, property visibility, and basic method dispatch fixes
are now present on `main` via `0405768 feat(runtime): add objects core PHPT
gate`.

## Selected PHPTs

| Submodule | Count | Selected PHPTs |
| --- | ---: | --- |
| `objects.magic` | 8 | `magic-get`, `magic-set`, `magic-isset`, `magic-unset`, `magic-call`, `magic-call-static`, `magic-invoke`, `magic-to-string` |
| `objects.clone` | 7 | `clone-identity`, `clone-independent-properties`, `clone-magic-method`, `clone-with-public-property`, `clone-with-typed-property`, `clone-with-type-mismatch`, `clone-with-unsupported-private` |
| `objects.traits` | 3 | `trait-method`, `trait-method-alias`, `trait-method-precedence` |
| `objects.enums` | 5 | `enum-unit-case`, `enum-backed-case`, `enum-cases`, `enum-from-tryfrom`, `enum-method` |

## Current Gate Status

Selected submodule gates:

| Gate | Reference | Target | Status |
| --- | ---: | ---: | --- |
| `nix develop -c just phpt-dev-module MODULE=objects.magic` | 8 PASS | 8 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.clone` | 7 PASS | 7 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.traits` | 3 PASS | 3 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.enums` | 5 PASS | 5 PASS | PASS |
| `nix develop -c just verify-phpt` | n/a | n/a | PASS |

All four submodule gates use curated selected manifests under
`tests/phpt/manifests/modules/`.

Selected acceptance:

| Area | Gate | Status |
| --- | --- | --- |
| magic | `nix develop -c cargo test -p php_vm` | PASS |
| magic | `nix develop -c just phpt-dev-module MODULE=objects.magic` | PASS |
| clone | `nix develop -c cargo test -p php_runtime object` | PASS |
| clone | `nix develop -c cargo test -p php_vm` | PASS |
| clone | `nix develop -c just phpt-dev-module MODULE=objects.clone` | PASS |
| traits | `nix develop -c cargo test -p php_ir` | PASS |
| traits | `nix develop -c cargo test -p php_vm` | PASS |
| traits | `nix develop -c just phpt-dev-module MODULE=objects.traits` | PASS |
| enums | `nix develop -c cargo test -p php_runtime object` | PASS |
| enums | `nix develop -c cargo test -p php_vm` | PASS |
| enums | `nix develop -c just phpt-dev-module MODULE=objects.enums` | PASS |

Post-core integration gates on `main`:

| Gate | Reference | Target | Status |
| --- | ---: | ---: | --- |
| `nix develop -c just phpt-dev-module MODULE=objects.magic` | 8 PASS | 8 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.clone` | 7 PASS | 7 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.traits` | 3 PASS | 3 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.enums` | 5 PASS | 5 PASS | PASS |
| `nix develop -c just phpt-dev-module MODULE=objects.classes` | 200 PASS | 164 PASS / 36 FAIL | FAIL |
| `nix develop -c just verify-runtime` | n/a | n/a | PASS |
| `nix develop -c just verify-phpt` | n/a | n/a | PASS |

## Magic Method MVP

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

## Clone and Clone-With MVP

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

## Traits MVP

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

## Enums MVP

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

## Closeout Status

The completed object-core branch is merged into `main` at `0405768`. The
advanced object gates pass on current `main` without undoing the object-core
fixes. The four advanced submodule gates pass independently.

Current aggregate `objects.classes` impact after the core branch merge:

- reference: 200 PASS
- target: 164 PASS, 36 FAIL
- dominant remaining areas: autoload and ReflectionException catch-type
  behavior, iterator/destructor ordering and exception behavior, serialization,
  `__sleep`, and `__toString` object formatting, class constant inheritance and
  dynamic lookup, property-reference and by-reference static-property
  assignment, static-as-instance edge cases, and broader object/reference COW
  behavior
- target summary:
  `/private/tmp/phrust-phpt-work/module-runs/objects.classes/target/summary.md`

## Current Blockers

No blockers remain for the selected advanced object gates. The selected gates
are integrated with the completed core branch, but the aggregate
`objects.classes` selected gate is still non-green at 164 PASS / 36 FAIL on the
target.

## Non-Scope

- object-core failures
- SPL implementation
- Reflection implementation
- SAPI
- serialization magic
- trait properties/constants/nested uses
- exhaustive enum diagnostics
