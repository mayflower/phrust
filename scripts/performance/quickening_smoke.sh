#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENGINE="${PHRUST_PHP_VM:-${CARGO_TARGET_DIR:-target}/debug/php-vm}"
OUT_DIR="target/performance/quickening-smoke"
FIXTURES_DIR="tests/fixtures/performance/perf_smoke"
DENSE_FIXTURES_DIR="$OUT_DIR/dense-fixtures"

if [ ! -x "$ENGINE" ]; then
    printf '[fail] Rust VM is not executable: %s\n' "$ENGINE" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
rm -rf "$OUT_DIR"/*
mkdir -p "$DENSE_FIXTURES_DIR"

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

cat > "$DENSE_FIXTURES_DIR/dense_arithmetic.php" <<'PHP'
<?php
$total = 1;
for ($i = 0; $i < 12; $i++) {
    $total = (($total + 2) * 1) - 1;
}
echo "dense-arith:", $total, "\n";
PHP
printf 'dense-arith:13\n' > "$DENSE_FIXTURES_DIR/dense_arithmetic.php.out"

cat > "$DENSE_FIXTURES_DIR/dense_concat.php" <<'PHP'
<?php
$text = "";
for ($i = 0; $i < 12; $i++) {
    $text = $text . "x";
}
echo "dense-concat:", $text, "\n";
PHP
printf 'dense-concat:xxxxxxxxxxxx\n' > "$DENSE_FIXTURES_DIR/dense_concat.php.out"

cat > "$DENSE_FIXTURES_DIR/dense_bool_branch.php" <<'PHP'
<?php
$count = 0;
$flag = true;
for ($i = 0; $i < 12; $i++) {
    if ($flag) {
        $count = $count + 1;
    }
}
echo "dense-bool:", $count, "\n";
PHP
printf 'dense-bool:12\n' > "$DENSE_FIXTURES_DIR/dense_bool_branch.php.out"

for fixture in "$DENSE_FIXTURES_DIR"/*.php; do
    name="$(basename "$fixture" .php)"
    expected="$fixture.out"

    "$ENGINE" run \
        --exec-format=bytecode \
        --quickening=off \
        --counters-json "$OUT_DIR/$name.off.counters.json" \
        "$fixture" \
        > "$OUT_DIR/$name.off.stdout" \
        2> "$OUT_DIR/$name.off.stderr"

    "$ENGINE" run \
        --exec-format=bytecode \
        --quickening=on \
        --counters-json "$OUT_DIR/$name.on.counters.json" \
        "$fixture" \
        > "$OUT_DIR/$name.on.stdout" \
        2> "$OUT_DIR/$name.on.stderr"

    cmp -s "$OUT_DIR/$name.off.stdout" "$OUT_DIR/$name.on.stdout" || {
        printf '[fail] dense quickening stdout diverged for %s\n' "$fixture" >&2
        exit 1
    }
    cmp -s "$OUT_DIR/$name.off.stderr" "$OUT_DIR/$name.on.stderr" || {
        printf '[fail] dense quickening stderr diverged for %s\n' "$fixture" >&2
        exit 1
    }
    cmp -s "$expected" "$OUT_DIR/$name.on.stdout" || {
        printf '[fail] dense quickening output does not match expectation for %s\n' "$fixture" >&2
        exit 1
    }
done

python3 - <<'PY'
import json
from pathlib import Path

out_dir = Path("target/performance/quickening-smoke")
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
    if "concat_prealloc_hits" not in sample or "concat_fallback_by_reason" not in sample:
        raise SystemExit("[fail] concat prealloc/fallback counters missing from sample")
    if "packed_dim_fast_path_hits" not in sample or "packed_dim_fast_path_misses" not in sample:
        raise SystemExit("[fail] packed dim fast-path counters missing from sample")
    for field in [
        "packed_fetch_fast_hits",
        "packed_fetch_bounds_fallbacks",
        "packed_fetch_layout_fallbacks",
        "packed_append_fast_hits",
        "packed_foreach_fast_hits",
        "cow_or_reference_fallbacks",
        "array_fast_path_hits_by_family",
        "array_fast_path_fallback_by_reason",
    ]:
        if field not in sample:
            raise SystemExit(f"[fail] packed-array fast-path counter missing from sample: {field}")

off_attempts = sum(sample.get("quickening_attempts", 0) for sample in off)
on_attempts = sum(sample.get("quickening_attempts", 0) for sample in on)
on_specialized = sum(sample.get("quickening_specialized", 0) for sample in on)
on_guard_hits = sum(sample.get("quickening_guard_hits", 0) for sample in on)
on_guard_misses = sum(sample.get("quickening_guard_misses", 0) for sample in on)
on_concat_hits = sum(sample.get("string_concat_fast_path_hits", 0) for sample in on)
on_packed_hits = sum(sample.get("packed_dim_fast_path_hits", 0) for sample in on)
on_packed_fetch_hits = sum(sample.get("packed_fetch_fast_hits", 0) for sample in on)
on_packed_append_hits = sum(sample.get("packed_append_fast_hits", 0) for sample in on)
on_packed_foreach_hits = sum(sample.get("packed_foreach_fast_hits", 0) for sample in on)
on_array_family_hits = sum(
    sample.get("array_fast_path_hits_by_family", {}).get("packed_int_fetch", 0)
    for sample in on
)
on_numeric_string_key_fallbacks = sum(
    sample.get("array_fast_path_fallback_by_reason", {}).get("numeric_string_key", 0)
    for sample in on
)
on_guard_failures = sum(sample.get("quickening_guard_failures", 0) for sample in on)
on_fallback_calls = sum(sample.get("quickening_fallback_calls", 0) for sample in on)
on_dequickens = sum(sample.get("quickening_dequickens", 0) for sample in on)
on_megamorphic = sum(sample.get("quickening_megamorphic", 0) for sample in on)
on_disabled = sum(sample.get("quickening_disabled", 0) for sample in on)
dense_on = [sample for path, sample in ((path, json.loads(path.read_text(encoding="utf-8"))) for path in sorted(out_dir.glob("dense_*.on.counters.json")))]
dense_lower_successes = sum(sample.get("bytecode_lower_successes", 0) for sample in dense_on)
dense_quickening_attempts = sum(sample.get("quickening_attempts", 0) for sample in dense_on)
dense_quickening_specialized = sum(sample.get("quickening_specialized", 0) for sample in dense_on)
dense_quickening_guard_hits = sum(sample.get("quickening_guard_hits", 0) for sample in dense_on)

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
if on_packed_fetch_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no packed_fetch_fast_hits")
if on_packed_append_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no packed_append_fast_hits")
if on_packed_foreach_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no packed_foreach_fast_hits")
if on_array_family_hits <= 0:
    raise SystemExit("[fail] quickening=on recorded no packed_int_fetch array family hits")
if on_numeric_string_key_fallbacks <= 0:
    raise SystemExit("[fail] quickening=on recorded no numeric_string_key array fallback")
if on_guard_failures != on_numeric_string_key_fallbacks:
    raise SystemExit(
        f"[fail] quickening guard failures {on_guard_failures} != numeric-string key fallbacks {on_numeric_string_key_fallbacks}"
    )
if on_fallback_calls != on_guard_misses:
    raise SystemExit(f"[fail] quickening fallback calls {on_fallback_calls} != guard misses {on_guard_misses}")
if on_dequickens != 0:
    raise SystemExit(f"[fail] inert quickening recorded dequickens: {on_dequickens}")
if on_megamorphic != 0:
    raise SystemExit(f"[fail] inert quickening recorded megamorphic transitions: {on_megamorphic}")
if on_disabled != 0:
    raise SystemExit(f"[fail] inert quickening recorded disabled transitions: {on_disabled}")
if len(dense_on) != 3:
    raise SystemExit(f"[fail] expected 3 dense quickening samples, found {len(dense_on)}")
if dense_lower_successes != 3:
    raise SystemExit(f"[fail] expected all dense fixtures to lower, got {dense_lower_successes}")
if dense_quickening_attempts <= 0:
    raise SystemExit("[fail] dense quickening recorded no attempts")
if dense_quickening_specialized <= 0:
    raise SystemExit("[fail] dense quickening recorded no metadata specializations")
if dense_quickening_guard_hits <= 0:
    raise SystemExit("[fail] dense quickening recorded no guard hits")
PY

printf '[pass] quickening smoke compared %s IR fixture(s) and %s dense fixture(s)\n' \
    "$(find "$FIXTURES_DIR" -maxdepth 1 -name '*.php' | wc -l | tr -d ' ')" \
    "$(find "$DENSE_FIXTURES_DIR" -maxdepth 1 -name '*.php' | wc -l | tr -d ' ')"
