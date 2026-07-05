# WordPress Root Performance Report

Date: 2026-07-05

Scope: live WordPress demo at `http://127.0.0.1:18080/`, using
`/Volumes/CrucialMusic/src/phrust_branches/phrustwordpress` with this Phrust
checkout at commit `44e4ded`.

## Executive Summary

The WordPress root page is slow because the request spends essentially all wall
time in Phrust VM execution. The web server, static file path, generated mysqli
smoke endpoint, request routing, response building, and entry script cache
lookup are not the bottleneck in the measured run.

The active Docker image is using the expected performance-oriented build and
runtime knobs: `experimental-jit`, script cache preload, `php_vm/jit-cranelift`,
`-C target-cpu=native`, and `-C target-feature=+neon`. Those knobs are active,
but they do not make this WordPress request native. The traced root render shows
`vm_jit_compile_attempts=3`, `vm_jit_compiled=0`, and `vm_jit_executed=0`.

The expensive part is therefore interpreted PHP execution of the WordPress
bootstrap/render path. Dense bytecode is active now, but it is still interpreted
bytecode plus rich VM/helper execution, and the current workload creates a very
large amount of runtime churn per request.

## Current Configuration Evidence

The running `phrust` container reports:

```text
engine_preset=experimental-jit
script_cache=true
script_cache_shards=16
script_cache_max_entries=4096
perf_trace=-
```

The container build metadata reports:

```text
rustc_host=aarch64-unknown-linux-gnu
rustflags=-C target-cpu=native -C target-feature=+neon
server_features=php_vm/jit-cranelift
```

The compose runner enables:

```text
PHRUST_ENGINE_PRESET=experimental-jit
PHRUST_SCRIPT_CACHE_PRELOAD=1
PHRUST_SERVER_FEATURES=php_vm/jit-cranelift
```

The live `18080` process did not have perf tracing enabled (`perf_trace=-`), so
VM-counter attribution was collected from a second same-container listener with
the same docroot, engine preset, preload file, and `Host: 127.0.0.1:18080`.

## Live Latency Measurements

Five sequential requests to the live root page:

| Run | HTTP | Total seconds | Bytes |
| --- | ---: | ---: | ---: |
| 1 | 200 | 4.892979 | 21705 |
| 2 | 200 | 5.382565 | 21705 |
| 3 | 200 | 4.885864 | 21705 |
| 4 | 200 | 4.659578 | 21705 |
| 5 | 200 | 5.280105 | 21705 |

Summary: min 4.660s, median 4.893s, max 5.383s.

Control paths on the same server:

| URL | HTTP | Total seconds | Bytes | Interpretation |
| --- | ---: | ---: | ---: | --- |
| `/wp-content/themes/phrust-demo/style.css` | 200 | 0.020312 | 7207 | static path is fast |
| `/wp-content/themes/phrust-demo/assets/elephrant-black.png` | 200 | 0.023879 | 1093892 | static path is fast |
| `/wp-includes/js/wp-emoji-release.min.js` | 200 | 0.004124 | 22762 | static path is fast |
| `/phrust-db-smoke.php` | 200 | 0.008202 | 45 | simple mysqli path is fast |
| `/__phrust/metrics` | 200 | 0.029284 | 4691 | metrics path is fast |

The access log agrees with the curl timings:

```text
path="/" status=200 bytes=21705 duration_ms=4889 route=php cache=hit
path="/" status=200 bytes=21705 duration_ms=5374 route=php cache=hit
path="/" status=200 bytes=21705 duration_ms=4883 route=php cache=hit
path="/" status=200 bytes=21705 duration_ms=4656 route=php cache=hit
path="/" status=200 bytes=21705 duration_ms=5277 route=php cache=hit
```

## Server Metrics Delta

The metrics delta over the five root requests plus one overlapping mysqli smoke
request shows the time is in `vm_execution`:

| Metric | Delta |
| --- | ---: |
| PHP responses | 6 |
| entry script cache lookups | 6 |
| entry script cache hits | 6 |
| entry script cache misses | 0 |
| compiles avoided | 6 |
| include compile hits | 2450 |
| include compile misses | 0 |
| include source reads | 0 |
| `vm_execution` time | 25.086603970s |
| `script_cache` time | 0.000342792s |
| `request_context` time | 0.000134708s |
| `route_resolution` time | 0.000330915s |
| `response_build` time | 0.000053084s |

This means the warmed run is not slow because the server is reading source files,
compiling the entry script, compiling includes, routing, or building the HTTP
response. The compiled script and include caches are working for this run. They
are caching compilation artifacts, not the runtime work WordPress does on every
request.

## VM Trace for the Root Render

The traced same-container 200 root render had a warm second request of:

```text
php_vm_execution=4.931229377s
script_cache_lookup=0.000098333s
route_resolution=0.000030916s
request_context=0.000052583s
response_build=0.000288667s
status=200
response_bytes=21705
script_cache_hit=true
```

Warm include/cache counters for that request:

```text
include_resolution_hits=499
include_resolution_misses=0
include_compile_hits=490
include_compile_misses=0
include_source_reads=0
entry_script_cache_hits=1
entry_script_source_reads=0
```

Execution counters for that request:

```text
vm_instructions_executed=754950
vm_bytecode_instructions_executed=974219
vm_entry_rich_instructions_executed=144
vm_include_rich_instructions_executed=754806
vm_entry_bytecode_instructions_executed=124
vm_include_bytecode_instructions_executed=974095
vm_dense_include_entry_attempts=490
vm_dense_include_entry_successes=469
vm_dense_include_entry_fallbacks=21
vm_dense_functions_executed=31856
vm_rich_fallback_functions_executed=1897
```

Call and allocation/churn counters for that request:

```text
vm_function_calls=104637
vm_method_calls=11469
vm_internal_function_dispatches=70554
vm_frame_allocations=16809
vm_frame_reuses=27680
vm_value_clones=99382364
vm_string_allocations=1469016
vm_array_handle_clones=27077072
vm_object_allocations=1562
vm_cow_separations=21855
vm_reference_cell_creations=2816
vm_array_dim_fetches=41289
vm_includes=499
vm_autoloads=18
```

Optimization counters for that request:

```text
vm_quickening_attempts=1005452
vm_quickening_specialized=6531
vm_inline_cache_hits=129923
vm_inline_cache_misses=19244
vm_builtin_call_ic_hits=70886
vm_builtin_call_ic_misses=2044
vm_jit_compile_attempts=3
vm_jit_compiled=0
vm_jit_executed=0
```

## Where the Time Is Spent

1. The root page is dominated by VM execution.

   The live aggregate delta put about 25.087 seconds into `vm_execution` for the
   measured root batch. Other measured phases were sub-millisecond noise in the
   same interval. Static files and the smoke mysqli endpoint also return in
   milliseconds.

2. The compiled script and include caches are active but insufficient.

   Warm root requests hit the entry script cache and include compile cache. The
   traced warm request had zero include source reads and zero include compile
   misses. The request is still slow because WordPress bootstrap and rendering
   are replayed through the VM on every request.

3. Dense bytecode is active, but this is not native execution.

   The current root trace is no longer the old "only a few dense instructions"
   situation. It executes about 974k dense bytecode instructions per warm root
   render. However, dense bytecode is still interpreted, and the request also
   executes about 755k rich VM instructions. The page remains far from a native
   compiled WordPress request.

4. The Cranelift JIT is effectively not contributing to this page.

   The image includes `php_vm/jit-cranelift` and the server runs
   `experimental-jit`, but the traced root render reports zero compiled and zero
   executed JIT functions. The JIT path currently targets narrow leaf patterns;
   this WordPress request is dominated by dynamic calls, includes, arrays,
   objects, internal function dispatch, references/COW behavior, and rich helper
   paths.

5. Runtime value churn is a primary suspect for the real CPU cost.

   A single warm root render reports about 99.4 million value clones, 27.1
   million array-handle clones, 1.47 million string allocations, 104k user
   function calls, 70k internal dispatches, and 11k method calls. Those counts
   are too high for a competitive request path even when the code is cached.

6. The persistent engine currently reuses immutable metadata but rejects
   request-local state persistence.

   Server metrics increment
   `phrust_server_persistent_engine_rejected_persistence_total{reason="request_local_state"}`
   once per PHP request. The request still constructs a fresh executor/request
   state for PHP-visible runtime state. That is semantically correct, but it
   means the current persistent layer is not an OPcache-style warmed WordPress
   runtime.

## Why Bytecode-Cached IR Is Still Slower Than Expected

Bytecode-cached IR removes parsing/lowering/optimization work from warmed
requests. It does not currently remove the dominant work here:

- executing hundreds of WordPress include bodies per request;
- rebuilding request-local globals, function/class state, hooks, options, query
  objects, template state, and output buffers;
- running almost 116k user/method calls per root render;
- dispatching about 70k internal functions per render;
- performing tens of millions of value and array-handle clone operations;
- falling back from dense bytecode to rich VM/helpers for language features that
  are still not represented by a fast native tier;
- compiling or executing no JIT code for this workload.

The cache is therefore solving a real compile-time problem, but the current
root-page latency is an execution-time problem.

## Root Fix Direction

The real fixes are not WordPress patches. They are Phrust runtime and VM fixes:

1. Add a lower-overhead always-on root attribution mode.

   Current VM counters are useful but heavy and only enabled with perf trace.
   Phrust needs per-request breakdowns for includes, user calls, method calls,
   internal calls, array operations, object/property operations, COW/reference
   operations, output buffering, database calls, and time spent in dense vs rich
   execution.

2. Reduce value and array clone traffic.

   The 99M value clone and 27M array-handle clone counts are the clearest hot
   symptom. The VM needs owned/borrowed operand paths, cheaper stack/local/global
   transfers, and fewer defensive clones around array/object/reference helpers.

3. Make dense execution cover more of the request without rich helper traffic.

   Dense include entry succeeds for most includes, but rich instruction count is
   still high. The next work should identify the 21 include fallback bodies and
   the 1,897 rich fallback function executions, then implement missing generic
   PHP features in dense bytecode rather than adding WordPress-specific paths.

4. Make persistent immutable PHP metadata materially reusable.

   Request-local state must stay isolated, but immutable declarations, class
   metadata, function metadata, include dependency metadata, call-shape feedback,
   and validated layout/type feedback should survive across requests without
   rebuilding or revalidating more than needed.

5. Broaden the native tier beyond narrow leaf functions.

   The current root render shows the JIT configured but unused. Competitive
   WordPress performance needs either broader baseline-native execution for
   dense bytecode regions or a much broader Cranelift eligibility model with
   safe side exits for arrays, objects, calls, COW/references, foreach, and
   output behavior.

6. Keep the benchmark tied to real WordPress.

   The right regression target is the combination used here:
   `/Volumes/CrucialMusic/src/phrust_branches/phrustwordpress` plus this Phrust
   checkout. The gate should record wall time, phase deltas, VM counters, cache
   counters, and correctness status for the root page. Synthetic fixtures are
   useful only when they explain a measured real-WordPress bottleneck.

## Commands Used

Live root timing:

```bash
/usr/bin/curl -fsS -H 'Authorization: Bearer dev-metrics-token' \
  http://127.0.0.1:18080/__phrust/metrics > /tmp/phrust-metrics-before.prom

for i in 1 2 3 4 5; do
  /usr/bin/curl -sS -o "/tmp/phrust-wp-root-${i}.html" \
    -w "root run=${i} code=%{http_code} starttransfer=%{time_starttransfer} total=%{time_total} size=%{size_download}\n" \
    http://127.0.0.1:18080/
done

/usr/bin/curl -fsS -H 'Authorization: Bearer dev-metrics-token' \
  http://127.0.0.1:18080/__phrust/metrics > /tmp/phrust-metrics-after.prom
```

Trace listener:

```bash
phrust-server \
  --listen 127.0.0.1:18081 \
  --docroot /var/www/html \
  --index index.php \
  --front-controller index.php \
  --engine-preset experimental-jit \
  --max-body-bytes 67108864 \
  --max-execution-ms 120000 \
  --upload-temp-dir /var/tmp/phrust-uploads \
  --session-save-path /var/tmp/phrust-sessions \
  --metrics-token dev-metrics-token \
  --access-log - \
  --script-cache-check-interval-ms 1000 \
  --script-cache-preload /var/tmp/phrust-script-preload.txt \
  --perf-trace /tmp/phrust-root-perf-200.jsonl \
  --perf-trace-vm-counters
```
