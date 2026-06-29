# Class Relation Caches

This document describes the request-local class relation cache used by the
inline-cache performance path. It does not change PHP-visible lookup order,
autoload behavior, reflection output, or declaration validation.

## Cached Relations

Relation slots are keyed by `ClassRelationCacheKey`:

- `kind`: one of class extends class, class implements interface, trait
  composition metadata, `instanceof` object/class, method override slot, final
  method/class fast check, visibility context check, or abstract/interface
  method relation.
- `subject`: normalized receiver class or class-like name.
- `target`: normalized target class, interface, trait, or declaring class.
- `member`: optional method/member name for method-table relations.
- `visibility_context`: optional normalized caller scope.
- `config_fingerprint`: request-local unit and strict-types fingerprint.

The currently integrated runtime fast path uses the relation cache for repeated
`instanceof` checks. Method override, final-method, and visibility metadata are
guarded by the existing method-call IC slot through `MethodCallGuardMetadata`.
The cache schema exposes the remaining relation kinds so follow-on optimizer
work can install them without adding another relation representation.

## Invalidation

Every entry is guarded by `ClassRelationEpochs`:

- `class_table_epoch`: bumps when dynamic class declarations are added.
- `autoload_epoch`: bumps when the autoload stack or registry changes.
- `include_eval_epoch`: combines include-path configuration and eval execution.
- `trait_interface_map_version`: currently follows the class-table epoch.
- `method_table_version`: currently follows the function/method lookup epoch.

A stale epoch removes the entry, records
`class_relation_cache_invalidations`, and recomputes through the existing slow
helper before installing a fresh request-local entry.

## Guardrails

- `instanceof` fallback still calls the existing `object_instanceof` helper.
- No cache lookup triggers or suppresses autoload; autoload side effects happen
  only in the existing class loading paths.
- Eval/include/autoload changes are guarded by epochs, so speculative relation
  answers are not reused across declaration or configuration changes.
- Reflection-visible class, interface, trait, final, abstract, and visibility
  behavior remains owned by the existing metadata and validation code.

## Counters

- `class_relation_cache_hits`: relation entry reused with matching epochs.
- `class_relation_cache_misses`: relation entry was absent and fell back.
- `class_relation_cache_invalidations`: relation entry was stale and removed.
- `instanceof_cache_hits`: `instanceof` relation hit.
- `instanceof_cache_misses`: `instanceof` relation miss or stale fallback.
- `method_override_cache_hits`: method-call IC hit with override/final/visibility
  metadata available.
- `method_override_cache_misses`: method-call IC miss requiring normal method
  resolution.

`inline_cache_class_relation_slots` counts observed `instanceof`/dynamic
`instanceof` IC slots in the generic inline-cache table.
