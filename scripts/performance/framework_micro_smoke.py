#!/usr/bin/env python3
"""Offline framework-like performance performance smoke comparisons."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_FIXTURES = ROOT / "tests/fixtures/performance/framework_smoke"
DEFAULT_OUT = ROOT / "target/performance/framework-smoke/summary.json"

VARIANTS = {
    "opt_off": [
        "--opt-level=0",
        "--quickening=off",
        "--inline-caches=off",
        "--bytecode-cache=off",
        "--jit=off",
    ],
    "opt_on": [
        "--opt-level=2",
        "--quickening=on",
        "--inline-caches=on",
        "--bytecode-cache=off",
        "--jit=off",
    ],
}


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, default=DEFAULT_ENGINE)
    parser.add_argument("--fixtures", type=Path, default=DEFAULT_FIXTURES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    return parser.parse_args()


def load_counters(path: Path) -> dict[str, int]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{rel(path)}: counters JSON root must be an object")
    return {key: value for key, value in data.items() if isinstance(key, str) and isinstance(value, int)}


def run_fixture(engine: Path, fixture: Path, out_dir: Path, variant: str, flags: list[str]) -> dict[str, Any]:
    stem = fixture.stem
    stdout_path = out_dir / f"{stem}.{variant}.stdout"
    stderr_path = out_dir / f"{stem}.{variant}.stderr"
    counters_path = out_dir / f"{stem}.{variant}.counters.json"
    command = [
        str(engine),
        "run",
        *flags,
        "--counters-json",
        str(counters_path),
        str(fixture),
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    counters = load_counters(counters_path) if counters_path.is_file() else {}
    return {
        "variant": variant,
        "command": command,
        "status": completed.returncode,
        "stdout": rel(stdout_path),
        "stderr": rel(stderr_path),
        "counters": counters,
        "counters_path": rel(counters_path),
    }


def main() -> int:
    args = parse_args()
    engine = args.engine
    fixtures_dir = args.fixtures
    out = args.out
    out_dir = out.parent

    if not engine.is_file():
        raise SystemExit(f"[fail] missing engine: {rel(engine)}")
    if not fixtures_dir.is_dir():
        raise SystemExit(f"[fail] missing framework smoke fixture directory: {rel(fixtures_dir)}")

    out_dir.mkdir(parents=True, exist_ok=True)
    for path in out_dir.glob("*"):
        if path.is_file():
            path.unlink()

    scenarios = []
    for fixture in sorted(fixtures_dir.glob("*.php")):
        expected = fixture.with_suffix(fixture.suffix + ".out")
        if not expected.is_file():
            raise SystemExit(f"[fail] missing expected output for {rel(fixture)}")
        expected_stdout = expected.read_text(encoding="utf-8")
        runs = {
            variant: run_fixture(engine, fixture, out_dir, variant, flags)
            for variant, flags in VARIANTS.items()
        }
        for variant, run in runs.items():
            if run["status"] != 0:
                raise SystemExit(f"[fail] {rel(fixture)} {variant} exited {run['status']}")
            actual = (ROOT / run["stdout"]).read_text(encoding="utf-8")
            if actual != expected_stdout:
                raise SystemExit(f"[fail] stdout mismatch for {rel(fixture)} {variant}")
        off_stdout = (ROOT / runs["opt_off"]["stdout"]).read_text(encoding="utf-8")
        on_stdout = (ROOT / runs["opt_on"]["stdout"]).read_text(encoding="utf-8")
        off_stderr = (ROOT / runs["opt_off"]["stderr"]).read_text(encoding="utf-8")
        on_stderr = (ROOT / runs["opt_on"]["stderr"]).read_text(encoding="utf-8")
        if off_stdout != on_stdout:
            raise SystemExit(f"[fail] opt off/on stdout diverged for {rel(fixture)}")
        if off_stderr != on_stderr:
            raise SystemExit(f"[fail] opt off/on stderr diverged for {rel(fixture)}")
        scenarios.append(
            {
                "id": fixture.stem,
                "fixture": rel(fixture),
                "variants": runs,
                "counter_focus": {
                    key: runs["opt_on"]["counters"].get(key, 0)
                    for key in [
                        "instructions_executed",
                        "function_calls",
                        "method_calls",
                        "internal_function_dispatch_cache_hits",
                        "inline_cache_hits",
                        "inline_cache_polymorphic",
                        "output_bytes",
                    ]
                },
            }
        )

    summary = {
        "schema_version": 1,
        "status": "ok",
        "engine": rel(engine),
        "fixture_count": len(scenarios),
        "variant_count": len(VARIANTS),
        "scenarios": scenarios,
    }
    out.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"[pass] framework smoke compared {len(scenarios)} fixture(s), "
        f"{len(scenarios) * len(VARIANTS)} run(s); wrote {rel(out)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
