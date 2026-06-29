# Runtime IR Templates

Runtime IR templates describe common guarded PHP operations as metadata before
they become executable code. The first catalog lives under
`php_jit::region_ir::templates` and is intentionally no-exec: a template may
lower to safe scalar region IR, emit rule-selection metadata, or report a
stable rejection reason when PHP semantics are not proven safe.

## Template Format

Each `RuntimeTemplate` declares:

- `TemplateParam` entries with input value classes and VM slots;
- `TemplateGuard` entries with snapshot requirements;
- required VM slots and reference/COW restrictions;
- possible side exits and fallback helper names;
- unsupported PHP semantic cases that must stay on the interpreter path;
- a lowering implementation that returns `TemplateLoweringOutcome`.

`TemplateLoweringContext` carries the proof facts available to the lowering
attempt, such as exact string inputs, packed-array read-only shape,
no-reference/no-COW constraints, interned keys, and absence of magic property
hooks. A successful lowering returns `TemplateLoweredArtifact` with optional
region IR, optional rule-selection metadata, metadata-only facts, and required
snapshot slots.

## Initial Catalog

The initial templates are:

| Template | Initial status |
| --- | --- |
| `int_add_checked` | Lowers to scalar region IR with checked-overflow side-exit metadata. |
| `int_compare` | Lowers to scalar region IR for exact `i64` comparison. |
| `known_builtin_strlen_exact` | Metadata-only when the argument is an exact string and conversion is impossible. |
| `known_builtin_count_packed_exact` | Metadata-only when the argument is a packed array without ref/COW sensitivity. |
| `packed_foreach_int_sum_metadata_only` | Metadata-only for read-only packed integer arrays. |
| `string_concat_exact` | Declared but rejected until string handles and allocation semantics are modeled in region IR. |
| `packed_array_fetch_readonly` | Declared but rejected for reference/COW-sensitive cases and until array value handles are modeled. |
| `property_slot_fetch_guarded` | Declared but rejected when magic properties or hooks may run. |
| `isset_array_key_interned_exact` | Declared but rejected until exact array isset semantics are modeled. |

The report smoke writes `target/performance/templates/templates.json` and
`target/performance/templates/templates.md` with `templates_considered`,
`templates_lowered`, `templates_rejected_by_reason`, `guards_required`, and
`snapshot_slots_required`.

## Rejected Semantic Cases

Templates reject rather than approximate behavior for:

- object/string conversion, including `__toString`;
- numeric-string, array, object, resource, and diagnostic-emitting conversions;
- reference cells, by-reference foreach, and COW separation;
- missing-key warnings, `isset` null-vs-missing behavior, and `ArrayAccess`;
- magic properties, property hooks, dynamic properties, visibility checks, and
  uninitialized typed property errors;
- Countable objects and recursive count modes.

These cases remain on existing runtime helpers and interpreter paths until the
region metadata can prove the same PHP-visible output, diagnostics, exit status,
and side-effect order.

## Future Relationship

The catalog is a construction aid for future copy-and-patch and
PHP-semantics-aware mid-tier compilation. It can produce region IR for the
validation interpreter today, and it can produce metadata for future stencils,
guards, snapshots, and fallback helpers. It does not allocate executable memory,
does not bypass runtime correctness helpers, and does not enable broad builtin
specialization without fixtures.
