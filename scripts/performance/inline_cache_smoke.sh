#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENGINE="${PHRUST_PHP_VM:-${CARGO_TARGET_DIR:-target}/debug/php-vm}"
OUT_DIR="target/performance/inline-cache-smoke"
FIXTURE_DIRS=(
    "tests/fixtures/performance/perf_smoke"
    "tests/fixtures/performance/inline_cache"
)

if [ ! -x "$ENGINE" ]; then
    printf '[fail] Rust VM is not executable: %s\n' "$ENGINE" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*

fixture_count=0
for fixtures_dir in "${FIXTURE_DIRS[@]}"; do
    for fixture in "$fixtures_dir"/*.php; do
        name="$(basename "$fixtures_dir")-$(basename "$fixture" .php)"
        expected="$fixture.out"
        if [ ! -f "$expected" ]; then
            continue
        fi

        "$ENGINE" run \
            --inline-caches=off \
            --counters-json "$OUT_DIR/$name.off.counters.json" \
            "$fixture" \
            > "$OUT_DIR/$name.off.stdout" \
            2> "$OUT_DIR/$name.off.stderr"

        "$ENGINE" run \
            --inline-caches=on \
            --counters-json "$OUT_DIR/$name.on.counters.json" \
            "$fixture" \
            > "$OUT_DIR/$name.on.stdout" \
            2> "$OUT_DIR/$name.on.stderr"

        cmp -s "$OUT_DIR/$name.off.stdout" "$OUT_DIR/$name.on.stdout" || {
            printf '[fail] inline-cache stdout diverged for %s\n' "$fixture" >&2
            exit 1
        }
        cmp -s "$OUT_DIR/$name.off.stderr" "$OUT_DIR/$name.on.stderr" || {
            printf '[fail] inline-cache stderr diverged for %s\n' "$fixture" >&2
            exit 1
        }
        cmp -s "$expected" "$OUT_DIR/$name.on.stdout" || {
            printf '[fail] inline-cache output does not match fixture expectation for %s\n' "$fixture" >&2
            exit 1
        }
        fixture_count=$((fixture_count + 1))
    done
done

python3 - <<'PY'
import json
from pathlib import Path

out_dir = Path("target/performance/inline-cache-smoke")
off = [json.loads(path.read_text(encoding="utf-8")) for path in sorted(out_dir.glob("*.off.counters.json"))]
on = [json.loads(path.read_text(encoding="utf-8")) for path in sorted(out_dir.glob("*.on.counters.json"))]
if not off or not on:
    raise SystemExit("[fail] missing inline-cache counter samples")

required_fields = [
    "inline_cache_observations",
    "inline_cache_slots",
    "inline_cache_function_slots",
    "inline_cache_method_slots",
    "inline_cache_property_slots",
    "inline_cache_dim_slots",
    "inline_cache_hits",
    "inline_cache_misses",
    "inline_cache_invalidations",
    "inline_cache_guard_failures",
    "inline_cache_fallback_calls",
    "inline_cache_megamorphic",
    "inline_cache_disabled",
    "method_ic_hits",
    "method_ic_misses",
    "method_ic_guard_failures",
    "property_ic_hits",
    "property_ic_misses",
    "property_ic_guard_failures",
    "class_static_ic_hits",
    "class_static_ic_misses",
    "class_static_ic_guard_failures",
    "include_path_ic_hits",
    "include_path_ic_misses",
    "include_path_ic_invalidations",
    "include_path_ic_guard_failures",
    "autoload_class_lookup_ic_hits",
    "autoload_class_lookup_ic_misses",
    "autoload_class_lookup_ic_invalidations",
    "autoload_class_lookup_ic_guard_failures",
]
for sample in off + on:
    for field in required_fields:
        if field not in sample:
            raise SystemExit(f"[fail] inline-cache counter missing from sample: {field}")

def total(samples, field):
    return sum(sample.get(field, 0) for sample in samples)

off_slots = total(off, "inline_cache_slots")
off_observations = total(off, "inline_cache_observations")
on_slots = total(on, "inline_cache_slots")
on_observations = total(on, "inline_cache_observations")
on_function_slots = total(on, "inline_cache_function_slots")
on_method_slots = total(on, "inline_cache_method_slots")
on_property_slots = total(on, "inline_cache_property_slots")
on_dim_slots = total(on, "inline_cache_dim_slots")
on_hits = total(on, "inline_cache_hits")
on_misses = total(on, "inline_cache_misses")
on_invalidations = total(on, "inline_cache_invalidations")
on_guard_failures = total(on, "inline_cache_guard_failures")
on_fallback_calls = total(on, "inline_cache_fallback_calls")
on_megamorphic = total(on, "inline_cache_megamorphic")
on_disabled = total(on, "inline_cache_disabled")
on_method_hits = total(on, "method_ic_hits")
on_method_misses = total(on, "method_ic_misses")
on_method_guard_failures = total(on, "method_ic_guard_failures")
on_property_hits = total(on, "property_ic_hits")
on_property_misses = total(on, "property_ic_misses")
on_property_guard_failures = total(on, "property_ic_guard_failures")
on_class_static_hits = total(on, "class_static_ic_hits")
on_class_static_misses = total(on, "class_static_ic_misses")
on_class_static_guard_failures = total(on, "class_static_ic_guard_failures")
on_include_path_hits = total(on, "include_path_ic_hits")
on_include_path_misses = total(on, "include_path_ic_misses")
on_include_path_guard_failures = total(on, "include_path_ic_guard_failures")
on_autoload_class_lookup_hits = total(on, "autoload_class_lookup_ic_hits")
on_autoload_class_lookup_misses = total(on, "autoload_class_lookup_ic_misses")
on_autoload_class_lookup_guard_failures = total(on, "autoload_class_lookup_ic_guard_failures")

if off_slots != 0 or off_observations != 0:
    raise SystemExit(f"[fail] inline-caches=off recorded slots={off_slots} observations={off_observations}")
if on_slots <= 0:
    raise SystemExit("[fail] inline-caches=on recorded no slots")
if on_observations < on_slots:
    raise SystemExit(f"[fail] inline-cache observations {on_observations} below slots {on_slots}")
if on_function_slots <= 0:
    raise SystemExit("[fail] inline-caches=on recorded no function call slots")
if on_method_slots <= 0:
    raise SystemExit("[fail] inline-caches=on recorded no method call slots")
if on_property_slots <= 0:
    raise SystemExit("[fail] inline-caches=on recorded no property fetch slots")
if on_dim_slots <= 0:
    raise SystemExit("[fail] inline-caches=on recorded no dim fetch slots")
if on_hits <= 0:
    raise SystemExit("[fail] function-call inline cache recorded no hits")
if on_misses <= 0:
    raise SystemExit("[fail] function-call inline cache recorded no misses")
if on_method_hits <= 0:
    raise SystemExit("[fail] method-call inline cache recorded no hits")
if on_method_misses <= 0:
    raise SystemExit("[fail] method-call inline cache recorded no misses")
if on_property_hits <= 0:
    raise SystemExit("[fail] property-fetch inline cache recorded no hits")
if on_property_misses <= 0:
    raise SystemExit("[fail] property-fetch inline cache recorded no misses")
if on_class_static_hits <= 0:
    raise SystemExit("[fail] class-constant/static-property inline cache recorded no hits")
if on_class_static_misses <= 0:
    raise SystemExit("[fail] class-constant/static-property inline cache recorded no misses")
if on_include_path_hits <= 0:
    raise SystemExit("[fail] include-path inline cache recorded no hits")
if on_include_path_misses <= 0:
    raise SystemExit("[fail] include-path inline cache recorded no misses")
if on_autoload_class_lookup_hits <= 0:
    raise SystemExit("[fail] autoload class lookup inline cache recorded no hits")
if on_autoload_class_lookup_misses <= 0:
    raise SystemExit("[fail] autoload class lookup inline cache recorded no misses")
if on_invalidations <= 0:
    raise SystemExit("[fail] inline-cache smoke recorded no IC invalidation")
if on_guard_failures != 0:
    raise SystemExit(f"[fail] function-call inline cache recorded unexpected guard failures: {on_guard_failures}")
if on_fallback_calls < on_misses:
    raise SystemExit(f"[fail] inline-cache fallback calls {on_fallback_calls} below misses {on_misses}")
if on_method_guard_failures != 0:
    raise SystemExit(f"[fail] method-call inline cache recorded unexpected guard failures: {on_method_guard_failures}")
if on_property_guard_failures != 0:
    raise SystemExit(f"[fail] property-fetch inline cache recorded unexpected guard failures: {on_property_guard_failures}")
if on_class_static_guard_failures != 0:
    raise SystemExit(f"[fail] class-constant/static-property inline cache recorded unexpected guard failures: {on_class_static_guard_failures}")
if on_include_path_guard_failures != 0:
    raise SystemExit(f"[fail] include-path inline cache recorded unexpected guard failures: {on_include_path_guard_failures}")
if on_autoload_class_lookup_guard_failures != 0:
    raise SystemExit(f"[fail] autoload class lookup inline cache recorded unexpected guard failures: {on_autoload_class_lookup_guard_failures}")
if on_megamorphic != 0:
    raise SystemExit(f"[fail] function-call inline cache recorded unexpected megamorphic transitions: {on_megamorphic}")
if on_disabled != 0:
    raise SystemExit(f"[fail] function-call inline cache recorded unexpected disabled transitions: {on_disabled}")
PY

printf '[pass] inline-cache smoke compared %s fixture(s)\n' "$fixture_count"
