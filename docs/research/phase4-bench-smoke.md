# Phase 4 VM Bench Smoke

`just bench-vm-smoke` runs a small local baseline over the Phase 4 VM CLI. The
cases cover echo loops, arithmetic loops, user-function calls, array append,
and method calls.

The output is written to `target/phase4/bench-vm-smoke/bench-vm-smoke.txt`.
Generated benchmark reports are not committed.

These numbers are not Phase 4 performance goals. They exist so Phase 5 and
later optimization work can detect obvious regressions while still treating
correctness, diagnostics, and reference behavior as the primary gates.

The smoke is intentionally optional. CI may run it as a soft check, but
`verify-phase4` does not depend on local wall-clock timing.
