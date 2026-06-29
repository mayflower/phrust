# Dense Bytecode Block Layout

Dense bytecode block layout is an optional interpreter-local layout pass. The
default mode is `source`, which preserves the block order produced by IR
lowering. The `profiled` mode is default-off and only applies to dense bytecode
when a local block-frequency profile is supplied for the current run.

## CLI Modes

`php-vm run --exec-format=bytecode --bytecode-layout=source file.php`
executes dense bytecode with the original block descriptor order.

`php-vm run --exec-format=bytecode --bytecode-layout=profiled
--bytecode-layout-profile target/performance/bytecode-layout/block-frequency.json
file.php` reorders dense block descriptors from a local profile artifact. If no
profile is supplied, or the supplied profile has no block-frequency data, the VM
keeps source order.

The profile artifact may contain either:

- `block_entries`: the dedicated block-frequency report format.
- `dense_block_entry_counts`: raw VM counter JSON from `--counters-json`.

No global persistent profile is read. No native block layout is performed. Rich
IR execution is not affected.

## Correctness Policy

The pass only reorders `DenseBlock` descriptors. It does not reorder the dense
instruction array, source spans, constants, names, or cache slots. Dense
instruction indexes therefore remain stable.

Jump targets are remapped through an old-block to new-block table. Conditional
terminators with implicit fallthrough are rewritten to the explicit two-target
conditional form before block descriptors move, so source-order fallthrough does
not become a different semantic edge.

Dense verification runs after layout. If verification fails, strict bytecode
execution reports an unsupported dense layout verification error and auto mode
uses the existing fallback policy.

Unsupported or fallback-only opcodes are not layout candidates because they do
not lower into dense bytecode. The pass does not cross exception, finally,
trace, diagnostic, return, or unsupported fallback boundaries; those cases stay
outside the current dense subset.

## Counters

- `dense_branch_executions`: conditional dense branches executed.
- `dense_branch_true`: conditional branches whose condition was truthy.
- `dense_branch_false`: conditional branches whose condition was falsey.
- `dense_branch_fallthrough_chosen`: implicit source-order fallthrough selected
  by `JumpIfTrue` or `JumpIfFalse`.
- `dense_block_entries`: dense block dispatch entries.
- `dense_block_entry_counts`: per-block entries keyed as `f{function}:b{block}`.
- `dense_branch_edge_counts`: per-edge branch counts keyed as
  `f{function}:b{from}->b{to}`.

## Layout Algorithm

For each dense function:

1. Start with entry block `0`.
2. Append the most frequent unscheduled successor according to block-entry
   counts.
3. Use source order for ties and unknown counts.
4. Append remaining unscheduled blocks in source order.

The pass emits a mapping table internally for every function. Identity layouts
are accepted and reported as skipped by reason `identity_order`.

## Smoke Gate

`scripts/performance/bytecode_layout_smoke.sh` runs the supported bytecode
fixture once in `source` layout, writes
`target/performance/bytecode-layout/block-frequency.json`, then runs the same
fixture in `profiled` layout with that local artifact. The gate compares stdout
and stderr and writes
`target/performance/bytecode-layout/summary.json`.

Run it with:

```bash
nix develop -c just bytecode-layout-smoke
```
