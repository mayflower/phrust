# Phase 5 Handoff

This handoff records the Phase 4 closeout status for the PHP engine work. It is
an audit document, not a compatibility claim.

## Validation Results

Prompt 39 validation set, run on 2026-06-21:

| Command | Result | Notes |
| --- | --- | --- |
| `nix develop -c just verify-phase0` | pass | Foundation gate preserved |
| `nix develop -c just verify-phase1` | pass | Lexer gate preserved |
| `nix develop -c just verify-phase2` | pass | Parser/CST gate preserved |
| `nix develop -c just verify-phase3` | pass | Semantic frontend gate preserved |
| `nix develop -c just verify-phase4` | pass | IR/VM/runtime gate, including corpus smoke |
| `nix develop -c cargo test --workspace` | pass | Workspace tests pass |

The hard Phase 4 gate also runs `runtime-fixtures`, `runtime-corpus-smoke`,
`phpt-smoke`, `runtime-known-gaps`, bytecode snapshots, Rust formatting,
Clippy, and the Phase 3 gate. `runtime-diff` remains reference-gated outside
`verify-phase4`; run it with `REFERENCE_PHP` set when a PHP reference binary is
available.

## Decision Record

Phase 4 decisions are captured in these ADRs:

| Decision | ADR |
| --- | --- |
| IR style | `docs/adr/0017-phase4-register-ir.md` |
| VM dispatch | `docs/adr/0018-phase4-vm-dispatch.md` |
| Value representation | `docs/adr/0019-phase4-runtime-value-representation.md` |
| Array MVP | `docs/adr/0020-phase4-array-mvp.md` |
| Object MVP | `docs/adr/0021-phase4-object-mvp.md` |
| Exception model | `docs/adr/0022-phase4-exception-model.md` |
| Include model | `docs/adr/0023-phase4-include-model.md` |
| Known-gap policy | `docs/adr/0024-phase4-known-gap-policy.md` |

## Feature Matrix

| Feature | Syntax supported | HIR supported | IR supported | VM execution | Reference diff status | Known gap ID |
| --- | --- | --- | --- | --- | --- | --- |
| Scalars and echo | yes | yes | yes | yes | green curated fixtures | none |
| Local variables and assignment | yes | yes | yes | partial | undefined variable wording differs | `E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING` |
| Arithmetic, concat, comparisons, casts | yes | yes | yes | partial | numeric-string edge cases differ | `E_PHP_RUNTIME_NUMERIC_STRING_MATRIX` |
| Direct user functions | yes | yes | yes | yes | green curated fixtures | none |
| Defaults, variadics, returns | yes | yes | yes | partial | PHP type/coercion wording differs | `E_PHP_RUNTIME_WEAK_STRICT_TYPES_COERCION` |
| Closures and arrow functions | yes | yes | yes | partial | by-value captures only | `E_PHP_VM_UNSUPPORTED_BY_REF_CAPTURE` |
| Dynamic function/callable forms | yes | partial | partial | partial | not PHP-compatible | `E_PHP_IR_UNSUPPORTED_DYNAMIC_FUNCTION_CALL` |
| PHP 8.5 pipe MVP | yes | yes | yes | partial | simple callables only | `E_PHP_VM_PIPE_RHS_NOT_CALLABLE` |
| Selected builtins | yes | yes | yes | partial | strict MVP only | `E_PHP_RUNTIME_UNSUPPORTED_STDLIB` |
| Arrays | yes | yes | yes | partial | key/COW/reference edges differ | `E_PHP_RUNTIME_ARRAY_REFERENCE_COW` |
| By-value foreach over arrays | yes | yes | yes | partial | arrays only, snapshot MVP | `E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE` |
| References | yes | partial | partial | partial | simple local alias only | `E_PHP_RUNTIME_UNSUPPORTED_REFERENCE_SEMANTICS` |
| Global and magic constants | yes | partial | partial | partial | limited predefined constants | `E_PHP_RUNTIME_PREDEFINED_CONSTANT_MATRIX` |
| Include/require | yes | yes | yes | partial | root-constrained local model | `E_PHP_RUNTIME_INCLUDE_SCOPE_MATRIX` |
| Concrete classes and `new` | yes | yes | yes | partial | public concrete class MVP | `E_PHP_IR_UNSUPPORTED_CLASSLIKE_OBJECT` |
| Public properties and methods | yes | yes | yes | partial | visibility/inheritance missing | `E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER` |
| Static methods | yes | partial | partial | partial | explicit class names only | `E_PHP_IR_UNSUPPORTED_LATE_STATIC_BINDING` |
| Clone and clone-with | yes | yes | yes | partial | public shallow MVP only | `E_PHP_RUNTIME_UNSUPPORTED_CLONE_WITH_PROPERTY_RULES` |
| Exceptions | yes | yes | yes | partial | internal Exception MVP | `E_PHP_RUNTIME_UNSUPPORTED_THROWABLE_HIERARCHY` |
| Runtime type checks | yes | yes | yes | partial | exact family checks only | `E_PHP_RUNTIME_WEAK_STRICT_TYPES_COERCION` |
| Superglobals | yes | partial | partial | partial | controlled CLI subset only | `E_PHP_RUNTIME_SUPERGLOBALS_FULL_MATRIX` |
| Generators and `yield from` | yes | yes | classified | no | known gap | `E_PHP_IR_UNSUPPORTED_GENERATOR` |
| Fibers | yes | yes | classified | no | known gap | `E_PHP_IR_UNSUPPORTED_FIBER` |
| Eval | yes | yes | classified | no | known gap | `E_PHP_IR_UNSUPPORTED_EVAL` |
| Autoload, traits, enums, reflection | yes | yes | classified | no | known gap | `E_PHP_IR_UNSUPPORTED_AUTOLOAD` |

## Top 20 Reference Deviations by Phase 5 Risk

1. Full references and Copy-on-Write: local aliases work, but parameters,
   returns, array elements, foreach references, and object-property references
   are gaps. ID: `E_PHP_RUNTIME_UNSUPPORTED_REFERENCE_SEMANTICS`.
2. Array COW and element references: array mutation is not zval-compatible
   across aliases. ID: `E_PHP_RUNTIME_ARRAY_REFERENCE_COW`.
3. Full array semantics: key conversion, spread, packed/hash transitions, and
   invalid-key behavior are incomplete. IDs:
   `E_PHP_RUNTIME_ARRAY_KEY_CONVERSION_EDGE_CASES`,
   `E_PHP_IR_UNSUPPORTED_ARRAY_SPREAD`.
4. Object model depth: inheritance, interfaces, traits, enums, visibility,
   readonly, hooks, dynamic names, and magic methods are not implemented.
   IDs: `E_PHP_IR_UNSUPPORTED_CLASSLIKE_OBJECT`,
   `E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER`,
   `E_PHP_RUNTIME_UNSUPPORTED_MAGIC_METHODS`.
5. Autoloading: class/function lookup never invokes autoload callbacks.
   ID: `E_PHP_IR_UNSUPPORTED_AUTOLOAD`.
6. Include compatibility: include_path, stream wrappers, cwd policy, and
   complete cross-file symbol side effects are missing. IDs:
   `E_PHP_VM_INCLUDE_MISSING`, `E_PHP_RUNTIME_INCLUDE_SCOPE_MATRIX`.
7. Standard library and extensions: only selected builtins exist. IDs:
   `E_PHP_RUNTIME_UNSUPPORTED_STDLIB`,
   `E_PHP_RUNTIME_BUILTIN_ARITY`,
   `E_PHP_RUNTIME_BUILTIN_TYPE`.
8. Throwable/Error hierarchy: exceptions execute through an internal MVP, not
   full PHP `Throwable` classes or stack traces. ID:
   `E_PHP_RUNTIME_UNSUPPORTED_THROWABLE_HIERARCHY`.
9. Type coercion: runtime parameter, return, and property checks do not
   implement PHP weak/strict coercion matrices. ID:
   `E_PHP_RUNTIME_WEAK_STRICT_TYPES_COERCION`.
10. Numeric-string conversion and comparison: only simple cases are covered.
    ID: `E_PHP_RUNTIME_NUMERIC_STRING_MATRIX`.
11. Superglobals and request state: CLI argv/env are controlled; SAPI request
    state and `$GLOBALS` aliasing are not complete. IDs:
    `E_PHP_RUNTIME_SUPERGLOBALS_FULL_MATRIX`,
    `E_PHP_RUNTIME_GLOBALS_ALIAS_MATRIX`.
12. Dynamic calls and callable resolution: variable functions, method
    callables, array callables, invokable objects, namespace fallback, and
    closure binding remain incomplete. IDs:
    `E_PHP_IR_UNSUPPORTED_DYNAMIC_FUNCTION_CALL`,
    `E_PHP_VM_UNSUPPORTED_METHOD_CALLABLE`.
13. Generators and `yield from`: classified at lowering, not executable.
    IDs: `E_PHP_IR_UNSUPPORTED_GENERATOR`,
    `E_PHP_IR_UNSUPPORTED_YIELD_FROM`.
14. Fibers: no scheduling or suspend/resume model. ID:
    `E_PHP_IR_UNSUPPORTED_FIBER`.
15. Eval: runtime source parsing/execution is not supported. ID:
    `E_PHP_IR_UNSUPPORTED_EVAL`.
16. Reflection/SPL metadata: reflection is classified as unsupported and SPL
    behavior is absent. ID: `E_PHP_IR_UNSUPPORTED_REFLECTION`.
17. Foreach beyond arrays: Traversable objects and by-reference foreach are
    not executable. IDs: `E_PHP_VM_UNSUPPORTED_FOREACH_SOURCE`,
    `E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH`.
18. Constants: runtime `define()`, complete predefined constants, and full
    constant-expression behavior are incomplete. IDs:
    `E_PHP_RUNTIME_DEFINE`,
    `E_PHP_RUNTIME_PREDEFINED_CONSTANT_MATRIX`.
19. Warning and fatal text compatibility: VM emits structured diagnostics
    instead of PHP CLI wording. ID: `E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT`.
20. Zend ABI, opcache/JIT, resources, and stream wrappers are intentionally
    deferred beyond this runtime MVP. IDs:
    `E_PHP_RUNTIME_UNSUPPORTED_ZEND_ABI`,
    `E_PHP_RUNTIME_UNSUPPORTED_JIT`,
    `E_PHP_RUNTIME_UNSUPPORTED_STREAM_WRAPPER`.

## Runtime/VM TODO, Unwrap, Expect, Panic Audit

Scope checked: `crates/php_runtime`, `crates/php_vm`, `crates/php_vm_cli`,
`crates/php_ir`, and runtime testkit paths.

| Class | Current occurrences | Classification |
| --- | --- | --- |
| `todo_phase4` modules | skeleton status exports in runtime, VM, CLI | informational markers retained for phase status APIs, not unimplemented execution paths |
| `expect("frame was pushed")`, `expect("caller frame is active")` in `php_vm/src/vm.rs` | repeated dispatch invariants after frame setup | internal VM invariant; should become controlled diagnostics only if a reachable malformed-state fixture appears |
| `expect("target bounds checked")` in `php_vm/src/frame.rs` | frame target mutation after prior bounds check | internal invariant guarded by VM code |
| Builtin `expect("checked arity")` | builtin implementations after registry arity validation | internal invariant; arity errors are surfaced before this point |
| `panic!("expected server array")` in runtime context tests/support path | protects controlled superglobal construction invariant | not user PHP input reachable in normal VM execution |
| Test and snapshot `unwrap`/`expect` | Rust tests, fixtures, snapshot serialization | acceptable test assertions |
| Testkit `panic!("{reason}")` | reference smoke test failure path | test-only failure reporting |

No `TODO phase4` marker currently indicates a silent runtime branch that
pretends to execute unsupported PHP. Unsupported runtime behavior is expected
to use known-gap IDs, diagnostics, or planned/deferred rows in
`docs/phase4-known-gaps.md`.

## Known-Gap Coverage Status

Known gaps are tracked in `docs/phase4-known-gaps.md`. The final Phase 4 gate
requires representative fixture files for executable known-gap categories such
as generators, `yield from`, fibers, eval, autoload, reflection, traits,
enums, property hooks, reference categories, foreach by reference, `$GLOBALS`
aliasing, clone-with visibility/readonly, and catch types.

Rows marked `planned` or `deferred` keep explicit examples or scope notes.
They are not counted as implemented and must gain fixtures when Phase 5 starts
work on that behavior.

## Phase 5 Task List

1. References and Copy-on-Write: replace the local-alias MVP with PHP-like
   zval/reference storage for variables, parameters, returns, array elements,
   object properties, and closure captures.
2. Arrays complete: implement full key normalization, spread/unpack, COW,
   element references, sorting/order edge cases, invalid-key diagnostics, and
   array/reference `var_dump` behavior.
3. Objects complete: add visibility, inheritance, interfaces, traits, enums,
   readonly/asymmetric visibility, property hooks, magic methods, dynamic
   class/property/method lookup, `__clone`, and late static binding.
4. Generators: execute `yield`, `yield from`, generator return values, send,
   throw, close, and foreach integration.
5. Fibers: model suspend/resume, scheduling state, errors, and interaction
   with exceptions and generators.
6. Standard library basis: expand selected builtins into a compatibility
   matrix with arity/type behavior and warning/error surfaces.
7. Reflection and SPL: expose runtime metadata, class/function inspection,
   iterator interfaces, core SPL containers, and autoload-sensitive behavior.
8. PHPT expansion: grow local PHPT smoke coverage, classify skips and known
   gaps explicitly, and run reference comparisons where stable.

## Handoff Position

Phase 4 is a green executable MVP for curated runtime fixtures. It is not
Composer-compatible, framework-compatible, Zend-bytecode-compatible, or ABI
compatible. Phase 5 should treat every row in the deviation list as open until
new implementation, fixtures, and reference-diff evidence prove otherwise.
