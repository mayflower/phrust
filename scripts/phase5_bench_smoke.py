#!/usr/bin/env python3
"""Opt-in Phase 5 microbenchmark smoke for runtime categories."""

from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_VM = ROOT / "target/debug/php-vm"

SCENARIOS = {
    "arrays": ROOT / "fixtures/phase5/arrays/append-after-gaps.php",
    "calls": ROOT / "fixtures/phase5/callables/named-basic.php",
    "objects": ROOT / "fixtures/phase5/objects/inheritance-public-method-property.php",
    "generators": ROOT / "fixtures/phase5/generators/key-value-yield.php",
    "fibers": ROOT / "fixtures/phase5/fibers/suspend-resume-basic.php",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repeats", type=int, default=int(os.getenv("PHRUST_PHASE5_BENCH_REPEATS", "5")))
    parser.add_argument("--out", type=Path, default=ROOT / "target/phase5/bench-smoke")
    parser.add_argument("--rust-vm", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_VM)))
    return parser.parse_args()


def run_once(vm: Path, fixture: Path) -> tuple[float, int, str, str]:
    started = time.perf_counter()
    completed = subprocess.run(
        [str(vm), "run", str(fixture)],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=5.0,
        check=False,
    )
    elapsed_ms = (time.perf_counter() - started) * 1000.0
    return elapsed_ms, completed.returncode, completed.stdout, completed.stderr


def main() -> int:
    args = parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    if not args.rust_vm.exists():
        print(f"[skip] php-vm binary not found: {args.rust_vm}")
        return 0

    results: list[dict[str, object]] = []
    failures = 0
    for name, fixture in SCENARIOS.items():
        if not fixture.exists():
            results.append({"scenario": name, "fixture": str(fixture.relative_to(ROOT)), "skip": "missing fixture"})
            continue
        samples: list[float] = []
        last_stdout = ""
        last_stderr = ""
        last_exit = 0
        for _ in range(args.repeats):
            elapsed_ms, exit_code, stdout, stderr = run_once(args.rust_vm, fixture)
            samples.append(elapsed_ms)
            last_stdout = stdout
            last_stderr = stderr
            last_exit = exit_code
        if last_exit != 0:
            failures += 1
        results.append(
            {
                "scenario": name,
                "fixture": str(fixture.relative_to(ROOT)),
                "repeats": args.repeats,
                "exit": last_exit,
                "min_ms": min(samples),
                "median_ms": statistics.median(samples),
                "max_ms": max(samples),
                "stdout_bytes": len(last_stdout.encode()),
                "stderr": last_stderr.replace(str(ROOT), "$ROOT"),
            }
        )

    report = {
        "note": "Smoke-only local timings for regression spotting; not a PHP/Zend benchmark.",
        "fail": failures,
        "results": results,
    }
    (args.out / "phase5-bench-smoke-report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    lines = ["Phase 5 bench smoke (local, non-comparative)"]
    for result in results:
        if "skip" in result:
            lines.append(f"- {result['scenario']}: skip {result['skip']}")
        else:
            lines.append(
                f"- {result['scenario']}: median={result['median_ms']:.3f}ms "
                f"min={result['min_ms']:.3f}ms max={result['max_ms']:.3f}ms exit={result['exit']}"
            )
    (args.out / "phase5-bench-smoke.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    if failures:
        print(f"[fail] Phase 5 bench smoke: failures={failures} path={args.out}")
        return 1
    print(f"[ok] Phase 5 bench smoke: scenarios={len(results)} path={args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
