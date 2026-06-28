#!/usr/bin/env python3
"""Compare the explicit fast engine preset against the CLI baseline."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from normalize_perf_output import normalize


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_OUT_DIR = ROOT / "target/performance/fast-preset"

RUNTIME_FIXTURES = (
    "fixtures/runtime/valid/hello.php",
    "fixtures/runtime/valid/scalars/expressions.php",
    "fixtures/runtime/valid/functions/factorial.php",
    "fixtures/runtime/valid/arrays/indexed.php",
)
STDLIB_FIXTURES = (
    "tests/fixtures/stdlib/_harness/json-pcre-date/json_basics.php",
    "tests/fixtures/stdlib/_harness/stdlib/string_transform.php",
    "tests/fixtures/stdlib/corpus/array_manipulation.php",
)
PERFORMANCE_FIXTURES = (
    "tests/fixtures/performance/perf_smoke/array_fast_paths_v2.php",
    "tests/fixtures/performance/perf_smoke/output_batching_v2.php",
    "tests/fixtures/performance/perf_smoke/strings_concat.php",
)
FRAMEWORK_FIXTURES = (
    "tests/fixtures/performance/framework_smoke/router_dispatch.php",
    "tests/fixtures/performance/framework_smoke/template_output.php",
    "tests/fixtures/performance/framework_smoke/packed_mixed_array_traversal.php",
)
PHPT_FIXTURES = (
    "fixtures/phpt_smoke/hello.phpt",
    "fixtures/phpt_smoke/array.phpt",
    "fixtures/phpt_smoke/function.phpt",
)

FALLBACK_KEYWORDS = (
    "fallback",
    "deopt",
    "exit",
    "miss",
    "slow",
    "guard_failure",
    "unsupported",
)


@dataclass(frozen=True)
class Case:
    category: str
    label: str
    path: Path
    source: str


@dataclass(frozen=True)
class RunResult:
    elapsed_ms: float
    returncode: int
    stdout: str
    stderr: str
    counters: dict[str, Any]


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def safe_name(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "__", value)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, default=DEFAULT_ENGINE)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--timeout", type=float, default=10.0)
    return parser.parse_args()


def materialize_phpt_file(phpt_path: Path, generated_dir: Path) -> Path:
    text = phpt_path.read_text(encoding="utf-8")
    match = re.search(r"^--FILE--\n(?P<body>.*?)(?=^--[A-Z]+--\n|\Z)", text, re.M | re.S)
    if not match:
        raise SystemExit(f"{rel(phpt_path)}: missing --FILE-- section")
    generated_dir.mkdir(parents=True, exist_ok=True)
    output = generated_dir / f"{phpt_path.stem}.php"
    output.write_text(match.group("body"), encoding="utf-8")
    return output


def fixture_cases(out_dir: Path) -> list[Case]:
    generated_phpt_dir = out_dir / "generated-phpt"
    groups = (
        ("runtime", RUNTIME_FIXTURES),
        ("stdlib", STDLIB_FIXTURES),
        ("performance", PERFORMANCE_FIXTURES),
        ("framework", FRAMEWORK_FIXTURES),
    )
    cases: list[Case] = []
    for category, fixtures in groups:
        for fixture in fixtures:
            path = ROOT / fixture
            if not path.is_file():
                raise SystemExit(f"missing {category} fast-preset fixture: {fixture}")
            cases.append(Case(category, fixture, path, "fixture"))
    for phpt in PHPT_FIXTURES:
        path = ROOT / phpt
        if not path.is_file():
            raise SystemExit(f"missing phpt fast-preset fixture: {phpt}")
        generated = materialize_phpt_file(path, generated_phpt_dir)
        cases.append(Case("phpt", phpt, generated, "phpt-file-section"))
    return cases


def normalized_env(out_dir: Path, case: Case, preset: str) -> dict[str, str]:
    tmp_dir = out_dir / "tmp" / preset / safe_name(case.label)
    tmp_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.update(
        {
            "TZ": "UTC",
            "LC_ALL": "C",
            "LANG": "C",
            "TMPDIR": str(tmp_dir),
            "TMP": str(tmp_dir),
            "TEMP": str(tmp_dir),
            "PHRUST_RANDOM_SEED": "performance-fast-preset-smoke",
            "RUST_TEST_SEED": "performance-fast-preset-smoke",
        }
    )
    return env


def run_case(engine: Path, case: Case, preset: str, out_dir: Path, timeout: float) -> RunResult:
    run_dir = out_dir / "runs" / safe_name(case.label) / preset
    run_dir.mkdir(parents=True, exist_ok=True)
    counters_path = run_dir / "counters.json"
    command = [
        str(engine),
        "run",
        f"--engine-preset={preset}",
        "--counters-json",
        str(counters_path),
        rel(case.path),
    ]
    start = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=normalized_env(out_dir, case, preset),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    elapsed_ms = (time.perf_counter_ns() - start) / 1_000_000.0
    stdout = completed.stdout.replace("\r\n", "\n").replace("\r", "\n")
    stderr = normalize(completed.stderr)
    (run_dir / "stdout").write_text(stdout, encoding="utf-8")
    (run_dir / "stderr").write_text(stderr, encoding="utf-8")
    (run_dir / "status").write_text(f"{completed.returncode}\n", encoding="utf-8")
    counters: dict[str, Any] = {}
    if counters_path.is_file():
        counters = json.loads(counters_path.read_text(encoding="utf-8"))
    if not isinstance(counters, dict):
        raise SystemExit(f"{rel(counters_path)}: counters root is not an object")
    return RunResult(elapsed_ms, completed.returncode, stdout, stderr, counters)


def collect_fallback_deopt_counters(counters: dict[str, Any]) -> dict[str, int]:
    selected: dict[str, int] = {}
    for key, value in counters.items():
        key_lower = key.lower()
        if isinstance(value, int) and any(word in key_lower for word in FALLBACK_KEYWORDS):
            selected[key] = value
        elif isinstance(value, dict):
            nested_total = sum(item for item in value.values() if isinstance(item, int))
            if nested_total and any(word in key_lower for word in FALLBACK_KEYWORDS):
                selected[key] = nested_total
    return {key: value for key, value in sorted(selected.items()) if value != 0}


def compare_case(case: Case, baseline: RunResult, fast: RunResult) -> list[str]:
    differences: list[str] = []
    if fast.returncode != baseline.returncode:
        differences.append(
            f"exit status baseline={baseline.returncode} fast={fast.returncode}"
        )
    if fast.stdout != baseline.stdout:
        differences.append("stdout differs")
    if fast.stderr != baseline.stderr:
        differences.append("stderr/runtime diagnostics differ")
    if differences:
        return [f"{case.label}: " + "; ".join(differences)]
    return []


def verdict(rows: list[dict[str, Any]], failures: list[str]) -> tuple[str, list[str]]:
    if failures:
        return "not-allowed", failures
    categories = {row["category"] for row in rows}
    required = {"runtime", "stdlib", "performance", "framework", "phpt"}
    if categories >= required:
        return (
            "deferred",
            [
                "baseline remains the default until broader PHPT and production workload "
                "coverage approve default-on promotion",
                "fast preset excludes Cranelift and strict-bytecode-only superinstructions",
                "bytecode cache remains explicit because it reads and writes local artifacts",
            ],
        )
    return (
        "not-allowed",
        ["audit did not cover runtime, stdlib, performance, framework, and PHPT cases"],
    )


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# Fast Engine Preset Smoke",
        "",
        "Generated by `nix develop -c just fast-preset-smoke`.",
        "Raw stdout, stderr, status, and counter artifacts are local-only under",
        "`target/performance/fast-preset/` and must not be committed.",
        "",
        "## Summary",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | `{summary['status']}` |",
        f"| Default-on verdict | `{summary['default_on_verdict']}` |",
        f"| Cases | {summary['case_count']} |",
        f"| Failures | {len(summary['failures'])} |",
        "",
        "## Default-On Notes",
        "",
    ]
    for reason in summary["default_on_reasons"]:
        lines.append(f"- {reason}")
    lines.extend(
        [
            "",
            "## Cases",
            "",
            "| Category | Fixture | Correctness | Fast fallback/deopt counters |",
            "| --- | --- | --- | --- |",
        ]
    )
    for row in summary["rows"]:
        fallback = ", ".join(
            f"{key}={value}" for key, value in row["fast_fallback_deopt_counters"].items()
        )
        lines.append(
            f"| `{row['category']}` | `{row['fixture']}` | `{row['correctness']}` | "
            f"{fallback or 'none'} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    engine = args.engine if args.engine.is_absolute() else ROOT / args.engine
    if not engine.is_file() or not os.access(engine, os.X_OK):
        raise SystemExit(f"engine is not executable: {engine}")
    out_dir = args.out_dir if args.out_dir.is_absolute() else ROOT / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    rows: list[dict[str, Any]] = []
    failures: list[str] = []
    for case in fixture_cases(out_dir):
        baseline = run_case(engine, case, "baseline", out_dir, args.timeout)
        fast = run_case(engine, case, "fast", out_dir, args.timeout)
        case_failures = compare_case(case, baseline, fast)
        failures.extend(case_failures)
        rows.append(
            {
                "category": case.category,
                "fixture": case.label,
                "source": case.source,
                "baseline": {
                    "returncode": baseline.returncode,
                    "elapsed_ms": baseline.elapsed_ms,
                },
                "fast": {
                    "returncode": fast.returncode,
                    "elapsed_ms": fast.elapsed_ms,
                },
                "correctness": "pass" if not case_failures else "fail",
                "fast_fallback_deopt_counters": collect_fallback_deopt_counters(
                    fast.counters
                ),
            }
        )

    default_on_verdict, default_on_reasons = verdict(rows, failures)
    summary: dict[str, Any] = {
        "status": "pass" if not failures else "fail",
        "gate": "fast-preset-smoke",
        "engine": rel(engine),
        "baseline_preset": "baseline",
        "fast_preset": "fast",
        "case_count": len(rows),
        "rows": rows,
        "failures": failures,
        "default_on_verdict": default_on_verdict,
        "default_on_reasons": default_on_reasons,
    }
    json_path = out_dir / "summary.json"
    markdown_path = out_dir / "summary.md"
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    markdown_path.write_text(render_markdown(summary), encoding="utf-8")
    if failures:
        for failure in failures:
            print(f"[fail] {failure}", file=sys.stderr)
        return 1
    print(
        "[pass] fast preset matched baseline across "
        f"{len(rows)} case(s); default-on verdict={default_on_verdict}; "
        f"wrote {rel(json_path)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
