#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENGINE="${PHRUST_PHP_VM:-${CARGO_TARGET_DIR:-target}/debug/php-vm}"
OUT_DIR="target/phase7/quickening-smoke"
FIXTURES_DIR="tests/fixtures/phase7/perf_smoke"

if [ ! -x "$ENGINE" ]; then
    printf '[fail] Rust VM is not executable: %s\n' "$ENGINE" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*

for fixture in "$FIXTURES_DIR"/*.php; do
    name="$(basename "$fixture" .php)"
    expected="$fixture.out"
    if [ ! -f "$expected" ]; then
        printf '[fail] missing expected output for %s\n' "$fixture" >&2
        exit 1
    fi

    "$ENGINE" run \
        --quickening=off \
        --counters-json "$OUT_DIR/$name.off.counters.json" \
        "$fixture" \
        > "$OUT_DIR/$name.off.stdout" \
        2> "$OUT_DIR/$name.off.stderr"

    "$ENGINE" run \
        --quickening=on \
        --counters-json "$OUT_DIR/$name.on.counters.json" \
        "$fixture" \
        > "$OUT_DIR/$name.on.stdout" \
        2> "$OUT_DIR/$name.on.stderr"

    cmp -s "$OUT_DIR/$name.off.stdout" "$OUT_DIR/$name.on.stdout" || {
        printf '[fail] quickening stdout diverged for %s\n' "$fixture" >&2
        exit 1
    }
    cmp -s "$OUT_DIR/$name.off.stderr" "$OUT_DIR/$name.on.stderr" || {
        printf '[fail] quickening stderr diverged for %s\n' "$fixture" >&2
        exit 1
    }
    cmp -s "$expected" "$OUT_DIR/$name.on.stdout" || {
        printf '[fail] quickening output does not match fixture expectation for %s\n' "$fixture" >&2
        exit 1
    }
done

python3 - <<'PY'
import json
from pathlib import Path

out_dir = Path("target/phase7/quickening-smoke")
off = [json.loads(path.read_text(encoding="utf-8")) for path in sorted(out_dir.glob("*.off.counters.json"))]
on = [json.loads(path.read_text(encoding="utf-8")) for path in sorted(out_dir.glob("*.on.counters.json"))]
if not off or not on:
    raise SystemExit("[fail] missing quickening counter samples")
for sample in off + on:
    if "quickening_guard_hits" not in sample or "quickening_guard_misses" not in sample:
        raise SystemExit("[fail] quickening guard hit/miss counters missing from sample")
    for field in [
        "quickening_fallback_calls",
        "quickening_dequickens",
        "quickening_megamorphic",
        "quickening_disabled",
    ]:
        if field not in sample:
            raise SystemExit(f"[fail] quickening protocol counter missing from sample: {field}")
    if "string_concat_fast_path_hits" not in sample or "string_concat_fast_path_misses" not in sample:
        raise SystemExit("[fail] string concat fast-path counters missing from sample")
    if "packed_dim_fast_path_hits" not in sample or "packed_dim_fast_path_misses" not in sample:
        raise SystemExit("[fail] packed dim fast-path counters missing from sample")

off_attempts = sum(sample.get("quickening_attempts", 0) for sample in off)
on_attempts = sum(sample.get("quickening_attempts", 0) for sample in on)
on_specialized = sum(sample.get("quickening_specialized", 0) for sample in on)
on_guard_hits = sum(sample.get("quickening_guard_hits", 0) for sample in on)
on_guard_misses = sum(sample.get("quickening_guard_misses", 0) for sample in on)
on_concat_hits = sum(sample.get("string_concat_fast_path_hits", 0) for sample in on)
on_packed_hits = sum(sample.get("packed_dim_fast_path_hits", 0) for sample in on)
on_guard_failures = sum(sample.get("quickening_guard_failures", 0) for sample in on)
on_fallback_calls = sum(sample.get("quickening_fallback_calls", 0) for sample in on)
on_dequickens = sum(sample.get("quickening_dequickens", 0) for sample in on)
on_megamorphic = sum(sample.get("quickening_megamorphic", 0) for sample in on)
on_disabled = sum(sample.get("quickening_disabled", 0) for sample in on)

if off_attempts != 0:
    raise SystemExit(f"[fail] quickening=off recorded attempts: {off_attempts}")
if on_attempts <= 0:
    raise SystemExit("[fail] quickening=on recorded no attempts")
if on_specialized <= 0:
    raise SystemExit("[fail] quickening=on recorded no metadata specializations")
if on_guard_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no ADD_INT_INT guard hits")
if on_concat_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no CONCAT_STRING_STRING fast-path hits")
if on_packed_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no PACKED_ARRAY_INT_KEY fast-path hits")
if on_guard_failures != 0:
    raise SystemExit(f"[fail] quickening recorded guard failures: {on_guard_failures}")
if on_fallback_calls != on_guard_misses:
    raise SystemExit(f"[fail] quickening fallback calls {on_fallback_calls} != guard misses {on_guard_misses}")
if on_dequickens != 0:
    raise SystemExit(f"[fail] inert quickening recorded dequickens: {on_dequickens}")
if on_megamorphic != 0:
    raise SystemExit(f"[fail] inert quickening recorded megamorphic transitions: {on_megamorphic}")
if on_disabled != 0:
    raise SystemExit(f"[fail] inert quickening recorded disabled transitions: {on_disabled}")
PY

printf '[pass] quickening smoke compared %s fixture(s)\n' "$(find "$FIXTURES_DIR" -maxdepth 1 -name '*.php' | wc -l | tr -d ' ')"
