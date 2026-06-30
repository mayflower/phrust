# Runtime Compatibility Report

- Fixtures: 151
- Pass: 22
- Unexpected failures: 0
- Skipped: 117
- Expected known gaps: 12
- Unexpected passes: 0

## Categories

- `DiagnosticMismatch`: 1
- `ExpectedKnownGap`: 8
- `RuntimeExitMismatch`: 1
- `StdoutMismatch`: 1
- `UnsupportedFeature`: 1

## Feature Areas

- `By-reference foreach over temporary or nonlocal sources`: 1
- `Complete `var_dump` formatting matrix`: 1
- `Full PHP builtin type coercion and diagnostic matrix`: 1
- `Full PHP include/require warning text and include_path search`: 1
- `Full PHP standard library and extensions`: 1
- `Full PHP undefined-variable warning wording and variable names`: 1
- `Full PHP warning output channel and wording compatibility`: 3
- `Property modifier edges outside the covered object MVP`: 2
- `arrays`: 6
- `autoload`: 1
- `builtins`: 5
- `constants`: 7
- `control_flow`: 14
- `corpus_smoke`: 7
- `division-by-zero.php`: 1
- `enums`: 1
- `errors`: 1
- `eval`: 1
- `exceptions`: 7
- `fibers`: 1
- `foreach`: 5
- `functions`: 18
- `generators`: 2
- `hello.php`: 1
- `includes`: 8
- `match-no-arm.php`: 1
- `objects`: 16
- `php85`: 5
- `property_hooks`: 1
- `references`: 5
- `reflection`: 1
- `runtime-error.php`: 1
- `runtime_types`: 9
- `scalars`: 4
- `superglobals`: 5
- `traits`: 1
- `type-error.php`: 1
- `variables`: 4

## Diagnostic IDs

- `E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH`: 1
- `E_PHP_RETURN_VALUE_FROM_VOID_FUNCTION`: 1
- `E_PHP_RUNTIME_DIVISION_BY_ZERO`: 2
- `E_PHP_RUNTIME_NON_NUMERIC_STRING`: 1
- `E_PHP_RUNTIME_UNDEFINED_ARRAY_KEY_WARNING`: 1
- `E_PHP_RUNTIME_UNDEFINED_CONSTANT`: 1
- `E_PHP_RUNTIME_UNDEFINED_FUNCTION`: 4
- `E_PHP_STD_TYPE_ERROR`: 1
- `E_PHP_VM_INCLUDE_MISSING`: 3
- `E_PHP_VM_RETURN_TYPE_MISMATCH`: 2
- `E_PHP_VM_UNCAUGHT_EXCEPTION`: 11
- `E_PHP_VM_UNHANDLED_MATCH`: 1
- `E_PHP_VM_UNKNOWN_CLASS`: 2

## Owner Streams

- `runtime-semantics`: 11

## Non-Pass Fixtures

| Fixture | Status | Category | Known gap | Feature area | Owner | First differing line | Message |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `fixtures/runtime/corpus_smoke/class-methods.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/corpus_smoke/config-array.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/corpus_smoke/include-graph.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/corpus_smoke/lib/routes.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/corpus_smoke/lib/settings.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/corpus_smoke/router-dispatch.php` | `Skipped` | - | - | corpus_smoke | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/governance/diagnostic-mismatch.php` | `KnownGap` | `DiagnosticMismatch` | `E_PHP_RUNTIME_BUILTIN_TYPE` | Full PHP builtin type coercion and diagnostic matrix | runtime-semantics | - | - |
| `fixtures/runtime/governance/expected-known-gap.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT` | Full PHP warning output channel and wording compatibility | runtime-semantics | - | - |
| `fixtures/runtime/governance/runtime-exit-mismatch.php` | `KnownGap` | `RuntimeExitMismatch` | `E_PHP_VM_INCLUDE_MISSING` | Full PHP include/require warning text and include_path search | runtime-semantics | - | - |
| `fixtures/runtime/governance/stdout-mismatch.php` | `KnownGap` | `StdoutMismatch` | `E_PHP_RUNTIME_VAR_DUMP_FORMAT_MATRIX` | Complete `var_dump` formatting matrix | runtime-semantics | - | - |
| `fixtures/runtime/governance/unexpected-pass.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_RUNTIME_UNDEFINED_VARIABLE_WARNING` | Full PHP undefined-variable warning wording and variable names | runtime-semantics | - | - |
| `fixtures/runtime/governance/unsupported-feature.php` | `KnownGap` | `UnsupportedFeature` | `E_PHP_RUNTIME_UNSUPPORTED_STDLIB` | Full PHP standard library and extensions | runtime-semantics | - | - |
| `fixtures/runtime/known_gaps/autoload/spl-autoload-register.php` | `KnownGap` | `ExpectedKnownGap` | - | autoload | - | - | - |
| `fixtures/runtime/known_gaps/foreach/by-ref.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_IR_UNSUPPORTED_BY_REF_FOREACH` | By-reference foreach over temporary or nonlocal sources | runtime-semantics | - | - |
| `fixtures/runtime/known_gaps/objects/clone-with-private.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER` | Property modifier edges outside the covered object MVP | runtime-semantics | - | - |
| `fixtures/runtime/known_gaps/objects/clone-with-readonly.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_IR_UNSUPPORTED_OBJECT_PROPERTY_MODIFIER` | Property modifier edges outside the covered object MVP | runtime-semantics | - | - |
| `fixtures/runtime/valid/arrays/append-overwrite.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/arrays/indexed.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/arrays/isset-empty-unset.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/arrays/missing-key.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT` | Full PHP warning output channel and wording compatibility | runtime-semantics | - | - |
| `fixtures/runtime/valid/arrays/nested-fetch.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/arrays/string-keys.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/arrays/var-dump-mixed.php` | `Skipped` | - | - | arrays | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/builtins/gettype.php` | `Skipped` | - | - | builtins | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/builtins/is-types.php` | `Skipped` | - | - | builtins | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/builtins/print.php` | `Skipped` | - | - | builtins | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/builtins/var-dump-array.php` | `Skipped` | - | - | builtins | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/builtins/var-dump-scalars.php` | `Skipped` | - | - | builtins | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/builtin.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/define.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/global.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/magic-function.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/magic-method.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/constants/magic-top-level.php` | `Skipped` | - | - | constants | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/break.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/continue.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/do-while-once.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/for-loop.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/for-multi-expression.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/if-true-false.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/match-success.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/nested-if.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/null-coalesce.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/return.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/short-circuit.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/switch-fallthrough.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/ternary.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/control_flow/while-counter.php` | `Skipped` | - | - | control_flow | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/enums/unit-enum.php` | `Skipped` | - | - | enums | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/errors/warning-continuation.php` | `KnownGap` | `ExpectedKnownGap` | `E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT` | Full PHP warning output channel and wording compatibility | runtime-semantics | - | - |
| `fixtures/runtime/valid/eval/eval.php` | `Skipped` | - | - | eval | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/exceptions/catch-exception.php` | `Skipped` | - | - | exceptions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/exceptions/catch-finally.php` | `Skipped` | - | - | exceptions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/exceptions/finally-return.php` | `Skipped` | - | - | exceptions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/fibers/fiber.php` | `Skipped` | - | - | fibers | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/foreach/break-continue.php` | `Skipped` | - | - | foreach | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/foreach/key-value.php` | `Skipped` | - | - | foreach | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/foreach/nested.php` | `Skipped` | - | - | foreach | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/foreach/snapshot-mutation.php` | `Skipped` | - | - | foreach | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/foreach/values.php` | `Skipped` | - | - | foreach | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/arrow-capture.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/by-ref-capture.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/closure-return.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/closure-simple.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/closure-use.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/defaults.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/dynamic-call.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/extra-arg.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/factorial.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/local-scope.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/return-no-value.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/return-types.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/simple.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/two-args.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/functions/variadic-sum.php` | `Skipped` | - | - | functions | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/generators/yield-from.php` | `Skipped` | - | - | generators | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/generators/yield.php` | `Skipped` | - | - | generators | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/hello.php` | `Skipped` | - | - | hello.php | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/includes/include-missing.php` | `Skipped` | - | - | includes | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/includes/include-once.php` | `Skipped` | - | - | includes | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/includes/include-return.php` | `Skipped` | - | - | includes | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/includes/lib/once.php` | `Skipped` | - | - | includes | - | - | fixture metadata requested skip |
| `fixtures/runtime/valid/includes/lib/return-value.php` | `Skipped` | - | - | includes | - | - | fixture metadata requested skip |
| `fixtures/runtime/valid/includes/lib/share-variable.php` | `Skipped` | - | - | includes | - | - | fixture metadata requested skip |
| `fixtures/runtime/valid/includes/share-variable.php` | `Skipped` | - | - | includes | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/clone-independent.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/clone-object.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/clone-with.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/constructor-property.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/instantiate.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/method-call.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/method-return.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/private-method.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/private-property.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/property-read-write.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/static-method.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/static-property.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/this-property-method.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/objects/two-objects.php` | `Skipped` | - | - | objects | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/php85/pipe-builtin.php` | `Skipped` | - | - | php85 | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/php85/pipe-closure.php` | `Skipped` | - | - | php85 | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/php85/pipe-side-effects.php` | `Skipped` | - | - | php85 | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/php85/pipe-user-function.php` | `Skipped` | - | - | php85 | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/property_hooks/get-hook.php` | `Skipped` | - | - | property_hooks | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/references/array-element-ref.php` | `Skipped` | - | - | references | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/references/by-ref-param.php` | `Skipped` | - | - | references | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/references/by-ref-return.php` | `Skipped` | - | - | references | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/references/by-value.php` | `Skipped` | - | - | references | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/references/local-alias.php` | `Skipped` | - | - | references | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/reflection/reflection-class.php` | `Skipped` | - | - | reflection | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/runtime_types/nullable-simple.php` | `Skipped` | - | - | runtime_types | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/runtime_types/param-int.php` | `Skipped` | - | - | runtime_types | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/runtime_types/property-type.php` | `Skipped` | - | - | runtime_types | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/runtime_types/return-string.php` | `Skipped` | - | - | runtime_types | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/runtime_types/void-return.php` | `Skipped` | - | - | runtime_types | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/scalars/casts.php` | `Skipped` | - | - | scalars | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/scalars/comparisons.php` | `Skipped` | - | - | scalars | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/scalars/echo.php` | `Skipped` | - | - | scalars | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/scalars/expressions.php` | `Skipped` | - | - | scalars | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/superglobals/argc.php` | `Skipped` | - | - | superglobals | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/superglobals/argv.php` | `Skipped` | - | - | superglobals | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/superglobals/empty-superglobals.php` | `Skipped` | - | - | superglobals | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/superglobals/globals-alias.php` | `Skipped` | - | - | superglobals | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/superglobals/server-argv.php` | `Skipped` | - | - | superglobals | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/traits/trait-use.php` | `Skipped` | - | - | traits | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/variables/assignment.php` | `Skipped` | - | - | variables | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/variables/compound.php` | `Skipped` | - | - | variables | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/variables/inc-dec.php` | `Skipped` | - | - | variables | - | - | REFERENCE_PHP is not set |
| `fixtures/runtime/valid/variables/undefined.php` | `Skipped` | - | - | variables | - | - | REFERENCE_PHP is not set |
