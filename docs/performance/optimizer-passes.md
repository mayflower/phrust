# Performance Optimizer Passes

Performance optimizer work is a correctness-preserving IR rewrite layer. It runs
after frontend lowering and before VM execution, and every optimized unit must
still pass the IR verifier. `--opt-level=0` is the semantic baseline.

## CLI Surface

```bash
nix develop -c cargo build -p php_vm_cli --bin php-vm
nix develop -c target/debug/php-vm dump-ir tests/fixtures/performance/optimizer/arithmetic.php
nix develop -c target/debug/php-vm run --opt-level=0 tests/fixtures/performance/optimizer/arithmetic.php
nix develop -c target/debug/php-vm run --opt-level=1 tests/fixtures/performance/optimizer/arithmetic.php
nix develop -c target/debug/php-vm run --opt-level=2 tests/fixtures/performance/optimizer/arithmetic.php
```

`--opt-level=0` skips the optimizer pipeline. `--opt-level=1` and
`--opt-level=2` run the Performance pass pipeline and are tested against the
baseline by `optimizer-diff`, `performance-regression`, and `perf-flag-matrix`.

## Implemented Pass Families

| Pass family | Purpose | Required guardrail |
| --- | --- | --- |
| No-op/direct pipeline plumbing | Establishes pass reports and verifier boundaries. | Must preserve IR exactly. |
| Constant folding | Folds safe scalar constants such as integer arithmetic, boolean not, and string concatenation. | Must avoid overflow, diagnostics, conversions, references, and observable PHP behavior changes. |
| Peepholes | Removes no-op instructions and self moves where safe. | Must keep effectful and register-defining instructions. |
| CFG cleanup | Simplifies constant branches, forwards empty blocks, and trims unreachable empty tails. | Must preserve exception boundaries, source maps, and verifier-valid definitions. |
| Literal pooling/string interning | Reuses immutable literals through existing IR/runtime mechanisms. | Must not expose identity changes through PHP-visible mutation or references. |

## Validation

Use the narrow gate while iterating:

```bash
nix develop -c cargo test -p php_optimizer
nix develop -c cargo test -p php_ir verify --lib
nix develop -c just optimizer-diff
```

Before finishing optimizer work, run:

```bash
nix develop -c just performance-regression
nix develop -c just verify-performance
```

`optimizer-diff` compares opt levels 0, 1, and 2 across optimizer fixtures and
prints clear differences if stdout, stderr, exit status, or diagnostics diverge.

## Troubleshooting

- If a fixture changes output under `--opt-level=1` or `2`, first rerun with
  `--opt-level=0` and inspect `dump-ir` before and after the suspected pass.
- If the verifier rejects optimized IR, keep the verifier failure as the root
  signal and fix the pass; do not weaken verifier rules for convenience.
- If a fold looks profitable but can change warnings, exceptions, references,
  COW, destructors, magic methods, or conversion order, leave it unfused and add
  a fixture documenting the blocked case.
- If `perf-compare` does not show a speedup, do not treat that as an optimizer
  correctness failure. Performance wall-clock budgets are advisory.
