#!/usr/bin/env python3
"""Smoke-test the no-exec baseline-native stencil report."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
ENGINE = ROOT / "target/debug/php-vm"
OUT_DIR = ROOT / "target/performance/baseline-native-stencil"
FIXTURES = (
    ROOT / "fixtures/runtime/valid/hello.php",
    ROOT / "fixtures/runtime/valid/scalars/echo.php",
    ROOT / "tests/fixtures/performance/perf_smoke/arrays_packed.php",
    ROOT / "tests/fixtures/performance/framework_smoke/packed_mixed_array_traversal.php",
)


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def run_report(fixture: Path) -> dict[str, Any]:
    completed = subprocess.run(
        [str(ENGINE), "dump-baseline-native-stencil", rel(fixture), "--json"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"[fail] baseline-native stencil failed for {rel(fixture)} "
            f"with status {completed.returncode}: {completed.stderr.strip()}"
        )
    try:
        report = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"[fail] invalid stencil JSON for {rel(fixture)}: {exc}") from exc
    if report.get("schema_version") != 1:
        raise SystemExit(f"[fail] unexpected schema version for {rel(fixture)}")
    if report.get("backend") != "baseline-native-stencil":
        raise SystemExit(f"[fail] unexpected backend for {rel(fixture)}")
    if report.get("status") != "no-exec":
        raise SystemExit(f"[fail] stencil report is not no-exec for {rel(fixture)}")
    if report.get("native_execution") is not False:
        raise SystemExit(f"[fail] native execution unexpectedly enabled for {rel(fixture)}")
    if report.get("executable_memory") is not False:
        raise SystemExit(f"[fail] executable memory unexpectedly enabled for {rel(fixture)}")
    if report.get("instructions", 0) <= 0:
        raise SystemExit(f"[fail] stencil report has no instructions for {rel(fixture)}")
    if report.get("compile_cost_units", 0) <= 0:
        raise SystemExit(f"[fail] stencil report has no compile-cost estimate for {rel(fixture)}")
    if report.get("required_deopt_slots", 0) <= 0:
        raise SystemExit(f"[fail] stencil report has no deopt-slot estimate for {rel(fixture)}")
    return report


def main() -> int:
    if not ENGINE.is_file():
        raise SystemExit(f"[fail] Rust VM is not executable: {rel(ENGINE)}")
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    reports = []
    unsupported_reasons: dict[str, int] = {}
    for fixture in FIXTURES:
        if not fixture.is_file():
            raise SystemExit(f"[fail] missing stencil fixture: {rel(fixture)}")
        report = run_report(fixture)
        reports.append(report)
        output = OUT_DIR / f"{rel(fixture).replace('/', '__')}.json"
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        for reason, count in report.get("unsupported_by_reason", {}).items():
            if isinstance(count, int):
                unsupported_reasons[reason] = unsupported_reasons.get(reason, 0) + count
    if "array_reference_cow_and_key_state" not in unsupported_reasons:
        raise SystemExit("[fail] stencil smoke did not exercise array/COW unsupported state")
    summary = {
        "status": "pass",
        "schema_version": 1,
        "fixture_count": len(reports),
        "native_execution": False,
        "executable_memory": False,
        "instructions": sum(int(report["instructions"]) for report in reports),
        "stencilable_instructions": sum(
            int(report["stencilable_instructions"]) for report in reports
        ),
        "unsupported_instructions": sum(
            int(report["unsupported_instructions"]) for report in reports
        ),
        "helper_calls_estimate": sum(int(report["helper_calls_estimate"]) for report in reports),
        "required_deopt_slots": sum(int(report["required_deopt_slots"]) for report in reports),
        "compile_cost_units": sum(int(report["compile_cost_units"]) for report in reports),
        "code_size_bytes_estimate": sum(
            int(report["code_size_bytes_estimate"]) for report in reports
        ),
        "unsupported_by_reason": unsupported_reasons,
        "fixtures": [report["path"] for report in reports],
    }
    (OUT_DIR / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "[pass] baseline-native stencil smoke compared "
        f"{len(reports)} fixture(s), {summary['instructions']} instruction(s), "
        f"and wrote {rel(OUT_DIR / 'summary.json')}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
