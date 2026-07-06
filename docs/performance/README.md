# Performance Documentation

This directory owns performance methodology, optimization contracts, benchmark
fixtures, and committed performance summaries.

## Stable Contracts

- [Methodology](methodology.md): measurement and reporting policy.
- [Runtime optimization contract](runtime.md): behavior-preserving VM and
  runtime optimization rules.
- [Optimization gates](optimization-gates.md): allowed, subset-allowed, and
  blocked optimization classes.
- [Bytecode cache](bytecode-cache.md): cache format, validation, and CLI
  behavior.
- [Known gaps](known-gaps.md): performance gaps that remain intentionally open.

## Optimization Areas

- [Quickening and inline caches](quickening-inline-caches.md)
- [Optimizer passes](optimizer-passes.md)
- [Array fast paths](array-fast-paths.md)
- [Internal function dispatch cache](internal-function-dispatch-cache.md)
- [Output buffer fast paths](output-buffer-fast-paths.md)
- [SIMD byte kernels](simd-byte-kernels.md)

## Reports

Committed summaries live here when they are part of the reviewable project
state. Raw counters, JSON, profiler captures, and benchmark outputs stay under
`target/`.

- [Benchmark results](results.md)
- [Application flow results](app-flow-results.md)
- [Fastest engine results](fastest-engine-results.md)
- [Fastest hot paths](fastest-hotpaths.md)
- [Hot path inventory](reports/hotpath-inventory.md)

## Native Tier

Cranelift and native-tier documents are grouped under
[cranelift/](cranelift/README.md). The native tier remains experimental and
default-off unless a separate ADR changes that policy.
