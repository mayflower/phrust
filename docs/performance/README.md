# Performance documentation

This directory owns performance methodology, optimization contracts, benchmark
fixtures, and performance gates for the mandatory native engine.

Stable contracts:

- [Native execution architecture](../adr/0017-native-execution-architecture.md)
- [Methodology](methodology.md)
- [Native telemetry families](counter-families.md)
- [Optimization gates](optimization-gates.md)
- [Benchmark corpus](benchmark-corpus.md)
- [CI policy](ci-policy.md)

Generated counters, JSON, profiles, and benchmark reports stay under
`target/performance/`. Every correctness comparison uses the `baseline` and
`default` policies of the same Cranelift compiler or the external PHP 8.5.7
oracle. Performance tooling must not introduce a second execution backend.
