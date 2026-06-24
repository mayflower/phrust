# Runtime VM Bench Smoke

`just bench-vm-smoke` runs a small local baseline over the Runtime VM CLI. The
cases cover echo loops, arithmetic loops, user-function calls, array append,
and method calls.

The output is written to `target/runtime/bench-vm-smoke/bench-vm-smoke.txt`.
Generated benchmark reports are not committed.

These numbers are not Runtime performance goals. They exist so Runtime semantics and
later optimization work can detect obvious regressions while still treating
correctness, diagnostics, and reference behavior as the primary gates.

The smoke is intentionally optional. CI may run it as a soft check, but
`verify-runtime` does not depend on local wall-clock timing.
