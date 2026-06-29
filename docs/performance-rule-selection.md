# Performance Rule Selection

Rule-selection metadata records which dense bytecode instructions or region IR
nodes match a report-only execution pattern. It does not enable native code,
does not change PHP-visible behavior, and does not replace the dense bytecode
interpreter. The first version exists so superinstruction selection, future
copy-and-patch stencils, and future BURS-style code generation can share the
same vocabulary.

## Max-Munch v1

Dense bytecode rule selection uses a deterministic max-munch pass over each
dense function block. Adjacent supported pairs are selected before single
instructions so the report matches the same shape the interpreter can already
execute as superinstructions:

- `LoadConst` plus `Echo` becomes `LoadConstEcho`.
- `LoadLocal` plus `Echo` becomes `LoadLocalEcho`.
- `BinaryConcat` plus `Echo` becomes `ConcatEcho`.
- supported compare instructions feeding a conditional jump become
  `CompareAndBranch`.

The selector then records safe single-instruction rules such as constants,
moves, scalar binary operators, comparisons, packed fetches, and returns.
Unsupported or effectful shapes are marked as `Skipped` with a stable reason.

## Superinstructions

Rule selection is metadata-only, but it deliberately mirrors the current
superinstruction surface. A fused parent rule keeps every source instruction
index it covers, while the fused child instruction gets a separate
`FusedInto(parent)` record. This lets smoke reports explain why a child
instruction no longer appears as an independent selected rule without losing
source-index traceability.

The superinstruction smoke gate writes rule-selection dumps under:

```text
target/performance/rules/
```

Those generated files are validation artifacts and are not committed.

## Region IR

Region IR rule selection covers only the safe scalar subset: params, constants,
copies, integer-like arithmetic, compares, compare-and-branch pairs, and
returns. Control-only nodes are recorded as `NoRule`. Effectful or unsupported
nodes are skipped so future optimization work can see why a region is not ready
for stencil or native-tier selection.

## Future Copy-And-Patch and BURS

The shared metadata model includes rule IDs, rule kinds, operand constraints,
fused parents, skip reasons, and counters. Operand constraints are placeholders
for later target-specific requirements such as fixed registers, immediates, VM
stack slots, or stencil operands. Those constraints are not used by v1 because
there is no native execution path in this addendum.

## Counters

Every report includes:

- `rule_selection_candidates`
- `rule_selection_selected`
- `rule_selection_fused`
- `rule_selection_skipped`
- `rule_selection_by_kind`

The counters are intended for smoke gates and trend reports. They should not be
used to infer execution speed on their own; the selector only describes
supported shapes.

## Debug and Source Behavior

Fused and skipped records preserve original dense bytecode instruction indexes
or region node indexes. Source maps remain owned by the dense bytecode and IR
layers; rule selection only points back to those stable indexes. Because a
fused child is still represented in the report, debugging tools can explain the
parent rule without pretending the child had no source behavior.
