# Region Optimizer IR

The region optimizer IR is a compact, index-based substrate for future hot
region optimization and no-exec native-tier experiments. It lives in
`php_jit::region_ir` because the intended owners are JIT, copy-and-patch, and
mid-tier research paths. Dense bytecode remains the authoritative fast
interpreter format, and rich `php_ir` remains the frontend lowering format.

This IR is not a default runtime path. The first implementation stores nodes,
constants, snapshots, def-use lists, and metadata in tables indexed by stable
newtypes. It can build and verify a tiny scalar graph, dump it deterministically,
and model guard/snapshot metadata without allocating executable memory or
changing PHP-visible behavior.

## Adopted Ideas

The initial skeleton adopts these ideas from the IR JIT framework material:

- compact positive indexes for nodes and constants;
- SSA-like data dependencies plus explicit control dependencies;
- separate def-use side tables instead of pointer-heavy graph structures;
- typed nodes with scheduling placement and effect metadata;
- snapshot and guard placeholders for future side exits and deopt;
- construction-time folding APIs that keep raw `emit_*` constructors separate
  from conservative `fold_*` constructors;
- bounded CSE for pure scalar floating nodes;
- stable textual dumps for audit and tests.

## Construction-Time Folding

The region builder exposes two API families:

- `emit_*` always creates a node and is useful for exact graph-shape tests.
- `fold_*` applies only audited scalar folds, then falls back to emission.

Current folds are limited to typed scalar region nodes:

| Rule | Status | Guardrail |
| --- | --- | --- |
| `copy(x) -> x` | allowed | Exact same value node. |
| `phi(x, x, ...) -> x` | allowed | All phi inputs must be the same node. |
| `add_i64(x, 0) -> x` | allowed | Region node is already exact `i64`. |
| `sub_i64(x, 0) -> x` | allowed | Region node is already exact `i64`. |
| `mul_i64(x, 1) -> x` | allowed | Region node is already exact `i64`. |
| `and_bool(x, true) -> x` | allowed | Region node is already exact `bool`. |
| `or_bool(x, false) -> x` | allowed | Region node is already exact `bool`. |
| constant `i64` add/sub/mul | allowed when checked | Rust checked arithmetic must prove no overflow. |
| constant `i64` compare | allowed | Exact scalar compare only. |
| constant boolean `and`/`or` | allowed | Exact scalar boolean nodes only. |
| division, modulo, generic PHP values, calls, loads, stores, arrays, objects, references, COW-sensitive values, include, eval, autoload | disallowed | These can affect PHP-visible behavior or require richer semantic metadata. |

CSE is available only for pure, control-free, floating `i64` and `bool` nodes.
Each lookup scans backward through at most `RegionBuilderOptions::fold_cse_limit`
nodes. The default budget is intentionally small and deterministic.

The fold layer does not fold operations that could alter PHP-visible warnings,
integer overflow fallback, division-by-zero behavior, object/array/resource
conversion diagnostics, reference or COW identity, evaluation order, function
calls, property hooks, magic methods, destructors, output, include, eval, or
autoload effects.

## SCCP And GCM Prototype

`php_jit::region_ir::opt` contains a no-exec optimizer prototype. It produces
analysis reports and stable before/after dumps; it does not rewrite VM
bytecode, lower to native code, or change runtime execution.

The SCCP lattice is intentionally small:

| Lattice value | Meaning |
| --- | --- |
| `Top` / `Unknown` | The node may become constant later, or the prototype lacks enough information. |
| `Const(value)` | The node is a known scalar region constant. |
| `Bottom` / `NotConstant` | The node is known not to be a foldable scalar constant or is effectful. |

SCCP tracks two deterministic worklist surfaces in its report: executable
control edges and SSA data dependencies. Constant boolean branch inputs prune
`IfTrue` or `IfFalse` successor edges in the report so dead branch marking is
visible without mutating the graph.

GCM only classifies pure, control-free, floating scalar nodes (`bool` and
`i64`). It computes an early anchor from dependencies and a late anchor from
uses, then avoids choosing an anchor in a deeper loop when a shallower safe
anchor is available. This provides LICM-like behavior only as a consequence of
safe placement; there is no separate aggressive LICM pass.

Allowed movement example:

| Node shape | Movement |
| --- | --- |
| Pure `i64` constant or arithmetic used inside a loop but depending only on loop-external values | May be anchored outside the loop. |
| Pure scalar compare feeding a branch | May be scheduled at a safe common dominator of its inputs and uses. |

Forbidden movement example:

| Node shape | Policy |
| --- | --- |
| Guard, snapshot, deopt point | Pinned. |
| Memory load/store or memory token | Pinned or rejected. |
| Runtime helper, function call, output, include, eval, autoload-sensitive operation | Rejected by effects. |
| Array/object/property/reference/COW/destructor-sensitive operation | Rejected until represented by richer semantic metadata. |

Pinned PHP semantics include diagnostics, guards, snapshots, memory effects,
calls, output, references and COW identity, property hooks and magic methods,
destructors, include, eval, and autoload behavior.

## Validation Interpreter

`php_jit::region_ir::interpret_region` is a validation-only interpreter for the
safe scalar subset. It exists to test region extraction and optimizer analysis
before any native execution path is trusted. It does not replace dense bytecode,
does not expose a user-facing runtime flag, does not emit PHP output or
diagnostics, and does not model generic PHP `Value` behavior.

The current executable subset is deliberately small:

| Node shape | Interpreter behavior |
| --- | --- |
| `Const` `bool`/`i64` | Produces the exact scalar. |
| `Param` | Reads an explicit test input by VM slot. |
| `Copy` | Forwards the input scalar. |
| `Phi` | Selects the active simple-test input; cyclic data dependencies fail explicitly. |
| `Add`/`Sub`/`Mul` | Uses checked `i64` arithmetic and reports overflow as unsupported. |
| `Compare` | Evaluates exact scalar integer comparisons. |
| `Select` | Evaluates the chosen scalar arm from a boolean condition. |
| `If`/`Return` | Walks reconstructed control edges and returns the scalar result. |
| `Guard` | Passing guards continue; failing guards return side-exit metadata with the guard node, snapshot ID, live snapshot entries, and resume control. |

All nodes outside this subset, including calls, loads, stores, arrays, objects,
runtime helpers, diagnostics, memory effects, and generic PHP values, return a
structured unsupported result. This keeps the interpreter useful for comparing
scalar expected results, construction-time folds, SCCP reports, GCM scheduling
decisions, guards, and snapshots without silently inventing PHP semantics.

## Deferred Ideas

The skeleton intentionally defers:

- native code generation;
- Cranelift lowering from region IR;
- use as a VM execution format;
- broad PHP value semantics;
- OSR and rule selection;
- PHP object, array, reference, COW, destructor, include, and autoload
  specialization.

Those features require stronger metadata and validation before they can be
enabled without risking PHP-visible behavior.

## Relationship To Existing Layers

Dense bytecode is still the hot interpreter input. Region IR is only an
optimizer substrate that can later feed metadata-only copy-and-patch stencils,
Cranelift experiments, or PHP-semantics-aware mid-tier planning. Any future
optimized path must carry fallback, snapshot, and resume metadata before it can
be considered for execution.

The current verifier enforces table index validity, scalar typed inputs,
control dependencies for pinned/effectful nodes, constant references, and
guard/snapshot references. This keeps the first region IR useful for tests and
reports while preserving the existing VM and dense bytecode contracts.
