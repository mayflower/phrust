# Native production replacement contracts

This directory is the auditable control plane for Cranelift architecture
replacements. A contract is required when a task retires an existing production
execution route. It prevents a request to delete a fallback from being completed
as a wrapper, adapter, dual route, or renamed copy of the same route.

The compatibility boundary is externally observable PHP 8.5 behavior. Internal
compatibility with a retired VM path, helper ABI, generic binder, interpreter
route, or fallback implementation is not a requirement.

## Required workflow

1. Create one concrete `*.json` contract for the tranche before changing the
   production path.
2. Name the exact legacy symbols, paths, and call edges that must disappear.
3. Name the genuinely runtime-dynamic PHP semantic slow paths that remain. Use
   an empty list when none remain; never invent a slow path to satisfy the schema.
4. Implement the smallest **complete vertical replacement**. Do not leave old
   and new production routes coexisting.
5. Run the replacement guard, then every correctness, application, and
   performance command listed in the contract.
6. Keep the contract after merge. It is the permanent deletion and acceptance
   record, not a temporary task note.

```bash
python3 scripts/verify/native_replacement_guard.py \
  --require-contract \
  --diff-policy
```

Codex project hooks activate this mode automatically whenever a prompt combines
an explicit removal/replacement instruction with a fallback, wrapper, adapter,
bridge, legacy route, interpreter route, generic binder, or comparable old
execution path. Add `[native-replacement]` to activate it unambiguously. Use
`[native-replacement:off]` only for discussion or analysis that does not request
a production cutover.

## Allowed PHP-semantic slow paths

A slow path is allowed when the dynamic condition is observable PHP semantics
and cannot be decided when the native image is published. Typical examples are:

- a callable or class target created at runtime;
- magic methods, property hooks, visibility failures, and user callbacks;
- references and Copy-on-Write separation;
- array-key coercion, hash growth, and genuinely mixed layouts;
- runtime `eval`, conditional declarations, and dynamic include targets;
- exception, warning, destructor, generator, and fiber state transitions.

These paths must be typed and out of line. They must not re-enter a retired
interpreter, VM executor, generic stable-call binder, or old runtime ABI. The
contract list may be empty when the replacement needs no semantic slow path.

## Forbidden engine fallbacks

The following are not PHP semantic slow paths and cannot be used to satisfy a
replacement task:

- a wrapper or adapter from the new API to the old implementation;
- dual dispatch, shadow execution, or a feature-gated old production route;
- a renamed legacy helper or generic catch-all recovery path;
- interpreter or old-VM re-entry after a native guard failure;
- generic argument binding for a statically known target;
- repeated ABI, helper-ID, callsite, or class-table validation after publication;
- a complete generic trampoline embedded under every optimized operation;
- preparation-only refactors that defer deletion to a later tranche.

## Contract schema

Copy `template.example.json` and replace every example value. Contracts with
placeholders, no removal target, no application/correctness gate, or no expected
structural metric movement are rejected.

`diff_allowlist` is deliberately narrow. It permits a source line matching the
regular expression only under the listed path prefixes and only with a concrete
reason. It is for genuine PHP-semantic slow paths, not for retaining an engine
fallback. Adding a broad expression such as `fallback` defeats the contract and
must be rejected in review.

The guard proves source-level absence for the named symbols and paths. The
contract's `required_validation` commands must supply the stronger runtime,
correctness, and performance evidence for that tranche.
