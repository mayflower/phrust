#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENGINE="${PHRUST_PHP_VM:-${CARGO_TARGET_DIR:-target}/debug/php-vm}"
FIXTURE="tests/fixtures/performance/regressions/polymorphic-method-property-ic.php"
EXPECTED="${FIXTURE}.out"
OUT_DIR="target/performance/polymorphic-inline-cache-smoke"

if [ ! -x "$ENGINE" ]; then
    printf '[fail] Rust VM is not executable: %s\n' "$ENGINE" >&2
    exit 1
fi
if [ ! -f "$FIXTURE" ] || [ ! -f "$EXPECTED" ]; then
    printf '[fail] missing polymorphic inline-cache fixture or expected output\n' >&2
    exit 1
fi

mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*

"$ENGINE" run \
    --inline-caches=off \
    --counters-json "$OUT_DIR/off.counters.json" \
    "$FIXTURE" \
    > "$OUT_DIR/off.stdout" \
    2> "$OUT_DIR/off.stderr"

"$ENGINE" run \
    --inline-caches=on \
    --counters-json "$OUT_DIR/on.counters.json" \
    "$FIXTURE" \
    > "$OUT_DIR/on.stdout" \
    2> "$OUT_DIR/on.stderr"

cmp -s "$OUT_DIR/off.stdout" "$OUT_DIR/on.stdout" || {
    printf '[fail] polymorphic IC stdout diverged\n' >&2
    exit 1
}
cmp -s "$OUT_DIR/off.stderr" "$OUT_DIR/on.stderr" || {
    printf '[fail] polymorphic IC stderr diverged\n' >&2
    exit 1
}
cmp -s "$EXPECTED" "$OUT_DIR/on.stdout" || {
    printf '[fail] polymorphic IC output does not match fixture expectation\n' >&2
    exit 1
}

python3 - <<'PY'
import json
from pathlib import Path

out_dir = Path("target/performance/polymorphic-inline-cache-smoke")
off = json.loads((out_dir / "off.counters.json").read_text(encoding="utf-8"))
on = json.loads((out_dir / "on.counters.json").read_text(encoding="utf-8"))

for field in [
    "inline_cache_monomorphic",
    "inline_cache_polymorphic",
    "inline_cache_megamorphic",
    "method_ic_hits",
    "method_ic_misses",
    "property_ic_hits",
    "property_ic_misses",
]:
    if field not in on:
        raise SystemExit(f"[fail] missing counter: {field}")

if off.get("inline_cache_slots", 0) != 0:
    raise SystemExit("[fail] inline-caches=off recorded IC slots")
if on["inline_cache_polymorphic"] <= 0:
    raise SystemExit("[fail] polymorphic IC fixture recorded no polymorphic hits")
if on["inline_cache_megamorphic"] <= 0:
    raise SystemExit("[fail] polymorphic IC fixture recorded no megamorphic fallback")
if on["method_ic_hits"] <= 0 or on["method_ic_misses"] <= 0:
    raise SystemExit("[fail] polymorphic IC fixture missing method IC hit/miss counters")
if on["property_ic_hits"] <= 0 or on["property_ic_misses"] <= 0:
    raise SystemExit("[fail] polymorphic IC fixture missing property IC hit/miss counters")
PY

printf '[pass] polymorphic inline-cache smoke compared fixture and counters\n'
