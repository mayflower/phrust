#!/usr/bin/env python3
"""Deterministic smoke gate for performance Cranelift helper-call reporting."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_OUT = ROOT / "target/performance/cranelift/bench-smoke.json"

SAMPLES = (
    {
        "name": "add_params",
        "fixture": ROOT / "tests/fixtures/performance/cranelift/helper-call/add-params.php",
        "expect_helper_calls": 1,
        "expect_native_execution": True,
    },
    {
        "name": "add_mul_expression",
        "fixture": ROOT / "tests/fixtures/performance/cranelift/helper-call/add-mul-expression.php",
        "expect_helper_calls": 2,
        "expect_native_execution": True,
    },
    {
        "name": "overflow_add",
        "fixture": ROOT / "tests/fixtures/performance/cranelift/helper-call/overflow-add.php",
        "expect_helper_calls": 0,
        "expect_native_execution": False,
    },
)


@dataclass(frozen=True)
class Run:
    command: list[str]
    returncode: int
    stdout: str
    stderr: str
    jit_stats: dict[str, Any] | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_ENGINE)))
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--timeout", type=float, default=float(os.getenv("PHRUST_CRANELIFT_BENCH_TIMEOUT", "10.0")))
    return parser.parse_args()


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def normalized_env(tmp_dir: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(
        {
            "TZ": "UTC",
            "LC_ALL": "C",
            "LANG": "C",
            "TMPDIR": str(tmp_dir),
            "TMP": str(tmp_dir),
            "TEMP": str(tmp_dir),
            "PHRUST_RANDOM_SEED": "performance-cranelift-bench-smoke",
            "RUST_TEST_SEED": "performance-cranelift-bench-smoke",
        }
    )
    return env


def normalize_text(value: str) -> str:
    return value.replace("\r\n", "\n").replace("\r", "\n")


def extract_jit_stats(stderr: str) -> dict[str, Any] | None:
    for line in stderr.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{"):
            continue
        try:
            decoded = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        jit = decoded.get("jit") if isinstance(decoded, dict) else None
        if isinstance(jit, dict):
            return jit
    return None


def stderr_without_jit_stats(stderr: str) -> str:
    lines = []
    for line in stderr.splitlines():
        stripped = line.strip()
        if stripped.startswith("{"):
            try:
                decoded = json.loads(stripped)
            except json.JSONDecodeError:
                pass
            else:
                if isinstance(decoded, dict) and isinstance(decoded.get("jit"), dict):
                    continue
        lines.append(line)
    return "\n".join(lines) + ("\n" if lines else "")


def run_engine(engine: Path, fixture: Path, mode: str, tmp_dir: Path, timeout: float) -> Run:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    command = [
        str(engine),
        "run",
        f"--jit={mode}",
        "--jit-eager",
        "--jit-stats=json",
        rel(fixture),
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=normalized_env(tmp_dir),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    stderr = normalize_text(completed.stderr)
    return Run(
        command=[rel(Path(command[0])), *command[1:]],
        returncode=completed.returncode,
        stdout=normalize_text(completed.stdout),
        stderr=stderr,
        jit_stats=extract_jit_stats(stderr),
    )


def require(condition: bool, failures: list[str], message: str) -> None:
    if not condition:
        failures.append(message)


def main() -> int:
    args = parse_args()
    engine = args.engine
    if not engine.is_file() or not os.access(engine, os.X_OK):
        raise SystemExit(f"Rust VM is not executable: {engine}")

    rows: list[dict[str, Any]] = []
    failures: list[str] = []
    for sample in SAMPLES:
        fixture = sample["fixture"]
        if not fixture.is_file():
            failures.append(f"{sample['name']}: missing fixture {rel(fixture)}")
            continue
        off = run_engine(
            engine,
            fixture,
            "off",
            ROOT / "target/performance/cranelift/bench-smoke-tmp" / sample["name"] / "off",
            args.timeout,
        )
        cranelift = run_engine(
            engine,
            fixture,
            "cranelift",
            ROOT / "target/performance/cranelift/bench-smoke-tmp" / sample["name"] / "cranelift",
            args.timeout,
        )
        require(
            off.returncode == cranelift.returncode,
            failures,
            f"{sample['name']}: exit mismatch off={off.returncode} cranelift={cranelift.returncode}",
        )
        require(
            off.stdout == cranelift.stdout,
            failures,
            f"{sample['name']}: stdout mismatch",
        )
        require(
            stderr_without_jit_stats(off.stderr) == stderr_without_jit_stats(cranelift.stderr),
            failures,
            f"{sample['name']}: stderr mismatch after removing jit stats",
        )
        stats = cranelift.jit_stats
        require(stats is not None, failures, f"{sample['name']}: missing Cranelift jit stats JSON")
        if stats is not None:
            require(stats.get("mode") == "cranelift", failures, f"{sample['name']}: stats mode is not cranelift")
            require(stats.get("compiled", 0) > 0, failures, f"{sample['name']}: expected native compile")
            if sample["expect_native_execution"]:
                require(stats.get("executed", 0) > 0, failures, f"{sample['name']}: expected native execution")
                require(stats.get("bailouts", 0) == 0, failures, f"{sample['name']}: unexpected bailout")
                require(
                    stats.get("helper_calls") == sample["expect_helper_calls"],
                    failures,
                    f"{sample['name']}: expected helper_calls={sample['expect_helper_calls']}, got {stats.get('helper_calls')}",
                )
            else:
                require(stats.get("executed", 0) == 0, failures, f"{sample['name']}: overflow should not count as executed")
                require(stats.get("bailouts", 0) > 0, failures, f"{sample['name']}: overflow should bailout")
                side_exit_reasons = stats.get("side_exit_reasons")
                require(
                    isinstance(side_exit_reasons, dict)
                    and side_exit_reasons.get("helper_status", 0) > 0,
                    failures,
                    f"{sample['name']}: expected helper_status side-exit reason",
                )

        rows.append(
            {
                "name": sample["name"],
                "fixture": rel(fixture),
                "jit_off": {
                    "command": off.command,
                    "exit_code": off.returncode,
                },
                "jit_cranelift": {
                    "command": cranelift.command,
                    "exit_code": cranelift.returncode,
                    "stats": cranelift.jit_stats,
                },
            }
        )

    report = {
        "gate": "jit-cranelift-bench-smoke",
        "status": "fail" if failures else "pass",
        "rows": rows,
        "failures": failures,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        for failure in failures:
            print(f"[fail] {failure}", file=sys.stderr)
        print(f"[fail] Cranelift bench smoke wrote {rel(args.out)}", file=sys.stderr)
        return 1

    print(f"[pass] Cranelift bench smoke validated {len(rows)} fixture(s); wrote {rel(args.out)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
