# Performance CI policy

Local and CI validation use the same Nix entrypoint:

```bash
nix develop -c just verify-performance
```

The required gate covers native correctness regression fixtures, optimizer and
cache checks, `baseline`/`default` parity, mandatory native entry, benchmark
smokes, and safety checks. Heavy release measurements and reports run through
`verify-performance-extended`.

Focused bisection commands include:

```bash
nix develop -c just default-profile-smoke
nix develop -c just native-smoke
nix develop -c just cranelift-native-cache
nix develop -c just optimizer-diff
nix develop -c just inline-cache-smoke
```

Cranelift is a regular dependency in every product build; CI has no feature-on
or feature-off compiler job. Optional PGO, BOLT, profilers, and long benchmarks
must report a stable skip reason when host prerequisites are absent. Generated
artifacts remain under `target/` and are diagnostic evidence, not portable
performance budgets.
