#!/usr/bin/env python3
"""Compare two performance PerfReport JSON files."""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_BASELINE = ROOT / "target/performance/baseline.json"
DEFAULT_CURRENT = ROOT / "target/performance/bench-performance-smoke.json"
DEFAULT_MARKDOWN = ROOT / "target/performance/perf-compare.md"
DEFAULT_JSON = ROOT / "target/performance/perf-compare.json"
SELF_TEST_BASELINE = ROOT / "tests/fixtures/performance/perf_compare/baseline.json"
SELF_TEST_CURRENT = ROOT / "tests/fixtures/performance/perf_compare/current.json"


@dataclass(frozen=True)
class MetricValue:
    scenario_id: str
    metric_name: str
    value: float
    unit: str
    lower_is_better: bool


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", nargs="?", type=Path, default=DEFAULT_BASELINE)
    parser.add_argument("current", nargs="?", type=Path, default=DEFAULT_CURRENT)
    parser.add_argument("--out", type=Path, default=DEFAULT_MARKDOWN)
    parser.add_argument("--json-out", type=Path, default=DEFAULT_JSON)
    parser.add_argument(
        "--fail-on-regression-percent",
        type=float,
        default=None,
        help="Exit 2 when any comparable metric regresses by at least this percent.",
    )
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def load_report(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise SystemExit(f"{path}: {error}") from error
    except json.JSONDecodeError as error:
        raise SystemExit(f"{path}: invalid JSON: {error}") from error
    if not isinstance(data, dict):
        raise SystemExit(f"{path}: report root must be a JSON object")
    if not isinstance(data.get("measurements", []), list):
        raise SystemExit(f"{path}: measurements must be a JSON array")
    return data


def scenario_id(measurement: dict[str, Any]) -> str | None:
    scenario = measurement.get("scenario")
    if not isinstance(scenario, dict):
        return None
    value = scenario.get("id")
    return value if isinstance(value, str) and value else None


def metric_values(report: dict[str, Any]) -> dict[tuple[str, str], MetricValue]:
    values = {}
    for measurement in report.get("measurements", []):
        if not isinstance(measurement, dict):
            continue
        scenario = scenario_id(measurement)
        if scenario is None:
            continue
        metrics = measurement.get("metrics", [])
        if not isinstance(metrics, list):
            continue
        for metric in metrics:
            if not isinstance(metric, dict):
                continue
            name = metric.get("name")
            value = metric.get("value")
            if not isinstance(name, str) or not isinstance(value, (int, float)):
                continue
            unit = metric.get("unit")
            lower = metric.get("lower_is_better", True)
            values[(scenario, name)] = MetricValue(
                scenario_id=scenario,
                metric_name=name,
                value=float(value),
                unit=unit if isinstance(unit, str) else "",
                lower_is_better=bool(lower),
            )
    return values


def scenario_ids(report: dict[str, Any]) -> set[str]:
    ids = set()
    for measurement in report.get("measurements", []):
        if isinstance(measurement, dict):
            scenario = scenario_id(measurement)
            if scenario is not None:
                ids.add(scenario)
    return ids


def percent_change(baseline: float, current: float) -> float:
    if baseline == 0.0:
        if current == 0.0:
            return 0.0
        return math.inf if current > 0 else -math.inf
    return ((current - baseline) / abs(baseline)) * 100.0


def is_regression(change_pct: float, lower_is_better: bool) -> bool:
    return change_pct > 0 if lower_is_better else change_pct < 0


def compare_reports(
    baseline: dict[str, Any],
    current: dict[str, Any],
    fail_on_regression_percent: float | None,
) -> tuple[dict[str, Any], int]:
    baseline_metrics = metric_values(baseline)
    current_metrics = metric_values(current)
    baseline_scenarios = scenario_ids(baseline)
    current_scenarios = scenario_ids(current)
    comparable = []
    regressions = []
    for key in sorted(set(baseline_metrics) & set(current_metrics)):
        before = baseline_metrics[key]
        after = current_metrics[key]
        change = percent_change(before.value, after.value)
        regression = is_regression(change, before.lower_is_better)
        row = {
            "scenario_id": before.scenario_id,
            "metric": before.metric_name,
            "unit": before.unit,
            "baseline": before.value,
            "current": after.value,
            "change_percent": change,
            "lower_is_better": before.lower_is_better,
            "regression": regression,
        }
        comparable.append(row)
        if (
            regression
            and fail_on_regression_percent is not None
            and abs(change) >= fail_on_regression_percent
        ):
            regressions.append(row)

    result = {
        "schema_version": 1,
        "summary": {
            "baseline_measurements": len(baseline.get("measurements", [])),
            "current_measurements": len(current.get("measurements", [])),
            "comparable_metrics": len(comparable),
            "missing_in_current": sorted(baseline_scenarios - current_scenarios),
            "added_in_current": sorted(current_scenarios - baseline_scenarios),
            "hard_regressions": len(regressions),
        },
        "comparisons": comparable,
        "hard_regressions": regressions,
    }
    return result, 2 if regressions else 0


def format_pct(value: float) -> str:
    if math.isinf(value):
        return "+inf%" if value > 0 else "-inf%"
    return f"{value:+.2f}%"


def render_markdown(result: dict[str, Any]) -> str:
    summary = result["summary"]
    lines = [
        "# performance Performance Comparison",
        "",
        "| Field | Value |",
        "| --- | ---: |",
        f"| Baseline measurements | {summary['baseline_measurements']} |",
        f"| Current measurements | {summary['current_measurements']} |",
        f"| Comparable metrics | {summary['comparable_metrics']} |",
        f"| Missing scenarios | {len(summary['missing_in_current'])} |",
        f"| Added scenarios | {len(summary['added_in_current'])} |",
        f"| Hard regressions | {summary['hard_regressions']} |",
        "",
        "| Scenario | Metric | Baseline | Current | Change | Direction |",
        "| --- | --- | ---: | ---: | ---: | --- |",
    ]
    for row in result["comparisons"]:
        direction = "lower" if row["lower_is_better"] else "higher"
        lines.append(
            f"| `{row['scenario_id']}` | `{row['metric']}` | "
            f"{row['baseline']:.6g} {row['unit']} | {row['current']:.6g} {row['unit']} | "
            f"{format_pct(row['change_percent'])} | {direction} is better |"
        )
    if summary["missing_in_current"]:
        lines.extend(["", "Missing in current:"])
        lines.extend(f"- `{scenario}`" for scenario in summary["missing_in_current"])
    if summary["added_in_current"]:
        lines.extend(["", "Added in current:"])
        lines.extend(f"- `{scenario}`" for scenario in summary["added_in_current"])
    return "\n".join(lines) + "\n"


def write_outputs(result: dict[str, Any], markdown_path: Path, json_path: Path) -> None:
    markdown_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.parent.mkdir(parents=True, exist_ok=True)
    markdown_path.write_text(render_markdown(result), encoding="utf-8")
    json_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_self_test() -> int:
    result, code = compare_reports(
        load_report(SELF_TEST_BASELINE),
        load_report(SELF_TEST_CURRENT),
        fail_on_regression_percent=20.0,
    )
    summary = result["summary"]
    assert summary["comparable_metrics"] == 2, summary
    assert summary["missing_in_current"] == ["performance.perf_smoke.rust-vm.missing"], summary
    assert summary["added_in_current"] == ["performance.perf_smoke.rust-vm.added"], summary
    assert summary["hard_regressions"] == 1, result
    assert code == 2, code
    markdown = render_markdown(result)
    assert "performance Performance Comparison" in markdown
    assert "wall_time_median" in markdown
    print("[pass] compare_perf_json self-test")
    return 0


def main() -> int:
    args = parse_args()
    if args.self_test:
        return run_self_test()
    result, code = compare_reports(
        load_report(args.baseline),
        load_report(args.current),
        args.fail_on_regression_percent,
    )
    write_outputs(result, args.out, args.json_out)
    sys.stdout.write(render_markdown(result))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
