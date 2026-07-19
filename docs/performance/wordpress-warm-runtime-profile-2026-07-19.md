# WordPress warm-runtime profile, 2026-07-19

This report records where Phrust spends time while serving an already warmed
WordPress request. It is a measurement snapshot, not a stable performance
contract. Generated profiles and raw benchmark artifacts remain under
`target/performance/` and are not committed.

## Executive result

Phrust is CPU-bound in warm mode and remains substantially slower than the
pinned PHP reference:

| Metric | Phrust | PHP-FPM 8.5.7 | Ratio |
| --- | ---: | ---: | ---: |
| p50 latency | 1,075.7 ms | 32.8 ms | 32.8x |
| p95 latency | 2,067.6 ms | 37.0 ms | 55.9x |
| CPU per request | 1,221 ms | 28 ms | 43.6x |
| Peak RSS | 776 MB | 138 MB | 5.64x |

Response status, selected headers, body length, and body SHA-256 were equal
between Phrust and PHP-FPM. Phrust time-to-first-byte was effectively equal to
its total wall time, so routing, networking, and response serialization do not
explain the gap.

To beat the measured PHP result, Phrust must remove approximately 97% of its
current p50 latency and 98% of its current CPU work.

## Measurement conditions

- Host: Linux x86_64, Intel Core i7-12800H, 20 logical CPUs.
- WordPress: 6.8.3, installed fixture under `target/wordpress-cutover`.
- WordPress cron was disabled for deterministic request timing.
- Reference: PHP 8.5.7 FPM with Opcache enabled and Opcache JIT disabled.
- Phrust: optimized release server, default engine preset, immutable
  deployment, preloaded script cache, in-memory native warmup, persistent
  native cache disabled.
- Load: concurrency 1, five warmups, ten measured requests.
- Latency came from clean, uninstrumented requests.
- Runtime counters came from a separate instrumented request.
- CPU attribution came from a `profiling` build sampled with Linux `perf`.

The clean benchmark can be reproduced with:

```bash
nix develop -c cargo build --release -p php_server --bin phrust-server
nix develop -c scripts/performance/wordpress_root_benchmark.py \
  --mode clean \
  --wordpress-dir target/wordpress-cutover \
  --docroot target/wordpress-cutover \
  --server target/release/phrust-server \
  --host-header 127.0.0.1:37023 \
  --samples 10 \
  --warmups 5 \
  --concurrency 1 \
  --timeout-seconds 60
```

The instrumented request took about 4.13 seconds instead of 1.08 seconds.
Counter totals are therefore useful as exact event counts, while instrumented
helper times are suitable only for ranking helpers. Percentages below come
from the clean sampling profile unless explicitly identified as counters.

The sampling run retained about 11,000 usable samples and reported five lost
chunks. Root traversal is independently confirmed by runtime counters, so the
primary conclusion is not dependent on those lost chunks. Small percentages
should still be treated as approximate.

## CPU attribution

### Call-root membership construction

Recursive call-root membership construction is the largest measured cost.

| Sampled operation | CPU self |
| --- | ---: |
| Recursive `collect_root_membership` visit | 14.85% |
| Root identity hash insertion | 6.39% |
| PHP array iteration | 5.28% |
| Root hash-table reserve and rehash | 4.51% |
| Membership count updates | 2.51% |
| Other directly attributable root-index work | about 0.8% |

The directly attributable total is about 34%. Associated allocations, value
clones, and memory movement put the effective cost at approximately 40-45% of
the request.

The request recorded only four call-root membership cache misses and four
call-root traversals. Each miss is expensive because it recursively enumerates
a large WordPress graph reachable from call arguments and constructs fresh
object, array, and reference identity sets. These summaries are request-local,
so their cost recurs on every request.

The object-release counter reports zero request-root scans. That counter covers
release-triggered rebuilds, not proactive call-root membership construction;
the four expensive traversals are reported by the separate call-membership
counter family.

### Runtime helper volume

One warm request crosses 2,200,546 runtime helper boundaries.

| Helper family | Calls | Share |
| --- | ---: | ---: |
| Locals, references, and lifecycle | 979,517 | 44.5% |
| Arrays and iteration | 513,626 | 23.3% |
| Execution polling | 197,719 | 9.0% |
| Scalar operations | 177,023 | 8.0% |
| Function and method calls | 142,853 | 6.5% |
| Properties | 109,248 | 5.0% |

The highest individual counts are:

| Helper | Calls |
| --- | ---: |
| Local fetch | 310,454 |
| Value release | 309,900 |
| Array fetch | 227,134 |
| Execution poll | 197,719 |
| Local store | 195,569 |
| Reference bind | 140,344 |
| Array insert | 117,376 |
| Comparison | 116,819 |
| Foreach next | 97,075 |

The cost is distributed across many generated-code-to-runtime boundaries.
Consequently, no individual helper accounts for the entire remaining sample.
At PHP's measured 32.8 ms p50, the whole request would have an average budget
of approximately 15 ns for each existing helper call. Most boundaries must be
removed rather than merely made incrementally faster.

### Value arena and ownership traffic

The instrumented request recorded:

- 750,216 value-table allocations;
- 169,857 value-table slot reuses;
- a 386,341-slot value-table high-water mark;
- 309,900 releases, 169,988 of which reached zero;
- 23,250 retains;
- 23,250 ownership clones; and
- 67,348 ownership escapes.

Linux sampling attributes about 15-16% directly to libc allocation and free
machinery. Some of that cost is caused by root-set construction, but value
cloning, dropping, encoding, decoding, and arena churn remain visible outside
the root traversal.

### Call dispatch and frame traffic

One request recorded:

- 252,545 call sites;
- 155,662 calls classified as direct;
- 96,883 calls classified as dynamic;
- 72,280 same-unit native transitions;
- 51.34 MB of native call-frame writes; and
- 1.65 MB of separately allocated call-argument data.

Frequent dynamic-call reasons include default, variadic, named, or unpacked
argument shapes; by-reference arguments; unpublished targets; and signature
mismatches. Reason counters are diagnostic classifications and are not assumed
to be mutually exclusive.

Sampling still finds work in the general native call dispatcher, argument
binding, continuation lookup, class and method lookup, dynamic entry lookup,
and external-function invocation. A call classified as direct therefore does
not yet imply a minimal compiled-to-compiled machine call.

### Hashing and memory movement

Additional clean samples include:

- `memmove`: 2.60%;
- `memcmp`: 2.17%;
- repeated SipHash and map hashing;
- class-name normalization;
- class, method, property, and constant lookup; and
- string interning and ordered-map operations.

These costs are spread through helper, call, and value-management paths rather
than concentrated in one PHP-visible operation.

## Hardware work

A separate run pinned the release server to a performance core and measured
ten warm requests. Per request, Phrust executed approximately:

- 7.70 billion instructions;
- 4.51 billion cycles;
- 1.34 billion branches;
- 21.1 million branch misses; and
- 30.4 million cache misses.

Measured IPC was 1.71. The primary problem is not IPC alone: the engine
executes far too many instructions and performs substantial pointer-heavy,
cache-unfriendly graph and metadata work.

## Tiering behavior

The instrumented warm request reported zero compile attempts, zero compile
time, and zero newly published versions. Compilation therefore does not
explain the roughly 1.1-second median.

Background Cranelift and regalloc2 activity was nevertheless visible during
clean server sampling and coincided with periodic latency outliers.

| Configuration | p50 | p95 | CPU/request | Peak RSS |
| --- | ---: | ---: | ---: | ---: |
| Baseline preset | 1,114.5 ms | 1,219.9 ms | 1,128 ms | 763 MB |
| Default, five warmups | 1,075.7 ms | 2,067.6 ms | 1,221 ms | 776 MB |
| Default, 25 warmups | 1,193.0 ms | 2,262.1 ms | 1,266 ms | 918 MB |

The default tier produced a small favorable median in the first run but worse
CPU, tail latency, and memory. Additional warmups did not make it converge.

Persistent feedback was neutral in an isolated A/B run:

- feedback disabled: 1,122.4 ms p50;
- feedback enabled: 1,127.2 ms p50.

## Effective warm-runtime breakdown

The sampled and counter-backed result is approximately:

```text
40-45%  recursive call-root membership construction
30-35%  helper boundaries and runtime dispatch
15-20%  allocation, value arena, and ownership traffic
 8-12%  call frames and dynamic linkage
 5-10%  hashing, lookup, and memory movement
  small  native computation itself
```

Ranges overlap slightly because root traversal itself causes allocations,
clones, hashing, and memory movement. They must not be summed as independent
percentages.

## Conclusion

The warm bottleneck is execution architecture rather than request routing or
foreground compilation. The measured request is dominated by recursive root
graph construction and by millions of runtime boundary, value-lifetime, and
call-frame operations. Eliminating the entire root-membership block would
still leave Phrust roughly twenty times slower than PHP in this workload.

Any work intended to beat PHP must therefore demonstrate large reductions in
all of the following executable metrics, not only a lower cost for one helper:

- call-root nodes visited;
- runtime helper calls;
- value-table allocations and high-water slots;
- dynamic calls and call-frame bytes;
- instructions and CPU time per request; and
- warm RSS and tail latency across a long run.
