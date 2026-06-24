# Semantic Fixtures

Semantic fixtures exercise the Semantic frontend frontend harness. They are intentionally
small and deterministic. Broad semantic rule coverage is added in later Semantic frontend
fixture sets.

Current categories:

- `valid`: accepted by PHP lint and the Rust frontend skeleton.
- `invalid`: rejected by PHP lint and the Rust frontend skeleton.
- `namespaces`: namespace forms and placement rules.
- `names`: import and name-resolution fixtures. Most are accepted; files with
  `invalid` in the name intentionally exercise semantic diagnostics such as
  duplicate import aliases.
- `declarations`: declaration-table fixtures for functions, constants,
  class-like declarations, conditional declarations, and safe local duplicates.
- `scopes`: lexical scope tree fixtures for namespaces, functions, closures,
  arrow functions, methods, and `global`/`static` metadata.
- `control_flow`: compile-time control-context diagnostics for break,
  continue, top-level return, yield, and initial goto/label handling.
- `types`: HIR type-lowering fixtures for parameters, returns, properties,
  composites, class-like special names, and invalid type contexts.
- `functions`: HIR signature-lowering fixtures for function-like parameters,
  return types, defaults, by-reference markers, variadics, and closure captures.
- `classes`: class-like semantic fixtures, including constructor property
  promotion metadata, invalid promotion contexts, class-context keywords, and
  magic-method checks.
- `declare`: file-level `declare` directive metadata and reference-safe
  `strict_types` diagnostics.
- `deferred`: include, require, and eval HIR nodes with explicit deferred
  runtime-effect metadata.
- `php85`: focused PHP 8.5 accepted forms.
- `expressions`: structural expression HIR fixtures for operators, nested
  calls, fetches, arrays/list destructuring, match, include/eval/exit, and PHP
  8.5 pipe, clone-with, and first-class callable forms.
- `statements`: structural statement HIR fixtures for control structures,
  declarations, globals/statics, unset/echo, labels/goto, exceptions, and
  returns.
- `const_expr`: constant-expression candidate fixtures for scalar, array,
  class constant fetch, parameter default, property default, promoted property,
  attribute argument, static local, invalid variable/call, and PHP 8.5.7
  closure, first-class callable, cast, and `new` forms.

Fixtures are analyzed, not executed.
Each category has a `README.md` with purpose, example rules, expected reference
classification, expected Rust diagnostic IDs, and known gaps.
Deterministic snapshots are generated under `fixtures/semantic/snapshots/` by
`scripts/run_semantic_fixtures.py --write-snapshots`.
`declarations/duplicate-class-invalid.php` is a semantic-only reject: PHP lint
accepts it, but Semantic frontend reports the safe same-file duplicate class declaration.
`control_flow/goto-invalid-known-gap.php` is a known reference-only reject:
PHP lint rejects jumping into a loop, while Semantic frontend currently records goto
labels but does not yet model goto target-entry restrictions.
The `types` fixtures match the pinned PHP 8.5.7 acceptance oracle; no
type-specific known gap is currently recorded.
The `functions` and constructor-promotion fixtures match the pinned PHP 8.5.7
acceptance oracle, including invalid constructor-promotion contexts.
The class-context fixtures match the pinned PHP 8.5.7 acceptance oracle for
`self`/`parent`/`static` usage, magic-method shape, and `$this` cases that are
lint-accepted but runtime-sensitive.
The declare fixtures match the pinned PHP 8.5.7 acceptance oracle for
`strict_types` placement and values. Runtime effects of `strict_types`, `ticks`,
and `encoding` are metadata-only in Semantic frontend.
The deferred fixtures match PHP lint acceptance while emitting note-level
runtime-deferred diagnostics. Analysis does not touch the filesystem, import
symbols from included files, or execute eval code.
The expression and statement HIR fixtures match the pinned PHP 8.5.7 acceptance
oracle. They are analyzed only; include, eval, and exit constructs are not
executed.
The constant-expression fixtures match the pinned PHP 8.5.7 acceptance oracle.
They annotate allowed and rejected forms without evaluating values.
