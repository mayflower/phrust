#!/usr/bin/env python3
"""Smoke-test the metadata-only PHP-aware mid-tier plan report."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
ENGINE = ROOT / "target/debug/php-vm"
OUT_DIR = ROOT / "target/performance/mid-tier"
FIXTURES = (
    ROOT / "tests/fixtures/performance/perf_smoke/arithmetic.php",
    ROOT / "tests/fixtures/performance/perf_smoke/arrays_packed.php",
    ROOT / "tests/fixtures/performance/perf_smoke/stdlib_dispatch.php",
    ROOT / "tests/fixtures/performance/perf_smoke/function_calls.php",
    ROOT / "tests/fixtures/performance/perf_smoke/loops.php",
    ROOT / "tests/fixtures/performance/perf_smoke/strings_concat.php",
)


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def run_report(fixture: Path) -> dict[str, Any]:
    completed = subprocess.run(
        [str(ENGINE), "dump-mid-tier-plan", rel(fixture), "--json"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"[fail] mid-tier plan failed for {rel(fixture)} "
            f"with status {completed.returncode}: {completed.stderr.strip()}"
        )
    try:
        report = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise SystemExit(
            f"[fail] invalid mid-tier plan JSON for {rel(fixture)}: {exc}"
        ) from exc
    if report.get("schema_version") != 1:
        raise SystemExit(f"[fail] unexpected schema version for {rel(fixture)}")
    if report.get("backend") != "php-mid-tier-plan":
        raise SystemExit(f"[fail] unexpected backend for {rel(fixture)}")
    if report.get("status") != "metadata-only":
        raise SystemExit(f"[fail] mid-tier report is not metadata-only for {rel(fixture)}")
    if report.get("native_execution") is not False:
        raise SystemExit(f"[fail] native execution unexpectedly enabled for {rel(fixture)}")
    if report.get("executable_memory") is not False:
        raise SystemExit(f"[fail] executable memory unexpectedly enabled for {rel(fixture)}")
    input_metadata = set(report.get("input_metadata", []))
    required_inputs = {
        "quickened_dense_bytecode",
        "inline_cache_feedback",
        "array_object_shapes",
        "numeric_string_classifications",
        "alias_reference_state",
        "branch_bias",
        "persistent_feedback",
        "deopt_live_state_maps",
    }
    missing_inputs = sorted(required_inputs.difference(input_metadata))
    if missing_inputs:
        raise SystemExit(
            f"[fail] mid-tier report missed input metadata for {rel(fixture)}: "
            + ", ".join(missing_inputs)
        )
    if report.get("eligible_functions", 0) + report.get("rejected_functions", 0) <= 0:
        raise SystemExit(f"[fail] mid-tier report has no function classifications for {rel(fixture)}")
    if report.get("deopt_points", 0) <= 0:
        raise SystemExit(f"[fail] mid-tier report has no deopt points for {rel(fixture)}")
    if not report.get("functions"):
        raise SystemExit(f"[fail] mid-tier report has no function detail for {rel(fixture)}")
    return report


def add_counts(target: dict[str, int], report: dict[str, Any], key: str) -> None:
    values = report.get(key, {})
    if not isinstance(values, dict):
        raise SystemExit(f"[fail] mid-tier report field is not a map: {key}")
    for name, count in values.items():
        if isinstance(count, int):
            target[name] = target.get(name, 0) + count


def main() -> int:
    if not ENGINE.is_file():
        raise SystemExit(f"[fail] Rust VM is not executable: {rel(ENGINE)}")
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    reports = []
    candidate_optimizations: dict[str, int] = {}
    rejection_reasons: dict[str, int] = {}
    expected_guards: dict[str, int] = {}
    required_helpers: dict[str, int] = {}
    for fixture in FIXTURES:
        if not fixture.is_file():
            raise SystemExit(f"[fail] missing mid-tier fixture: {rel(fixture)}")
        report = run_report(fixture)
        reports.append(report)
        output = OUT_DIR / f"{rel(fixture).replace('/', '__')}.json"
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        add_counts(candidate_optimizations, report, "candidate_optimizations")
        add_counts(rejection_reasons, report, "rejection_reasons")
        add_counts(expected_guards, report, "expected_guards")
        add_counts(required_helpers, report, "required_helpers")

    required_optimizations = {
        "tiny_leaf_method_inlining_candidate",
        "builtin_intrinsic_inlining",
        "packed_array_loop_specialization",
        "numeric_string_guard_specialization",
        "branch_layout",
        "allocation_scratch_buffer_elision",
    }
    missing_optimizations = sorted(required_optimizations.difference(candidate_optimizations))
    if missing_optimizations:
        raise SystemExit(
            "[fail] mid-tier smoke missed optimization(s): "
            + ", ".join(missing_optimizations)
        )

    required_rejections = {
        "references_or_unknown_aliasing",
        "cow_mutation_ambiguity",
        "magic_hooks_or_dynamic_calls",
        "eval_include_mutation_requires_invalidation",
        "exceptions_try_finally_need_live_state_support",
        "generators_fibers_require_suspend_state",
        "destructor_sensitive_values_need_materialization",
        "method_property_shape_metadata_missing",
    }
    missing_rejections = sorted(required_rejections.difference(rejection_reasons))
    if missing_rejections:
        raise SystemExit(
            "[fail] mid-tier smoke missed rejection(s): " + ", ".join(missing_rejections)
        )

    required_guards = {
        "int_or_numeric_string_operands",
        "packed_array_shape",
        "branch_bias_feedback",
        "destructor_sensitive_value_state",
    }
    missing_guards = sorted(required_guards.difference(expected_guards))
    if missing_guards:
        raise SystemExit("[fail] mid-tier smoke missed guard(s): " + ", ".join(missing_guards))
    if "known_builtin_helper" not in required_helpers:
        raise SystemExit("[fail] mid-tier smoke missed known builtin helper requirement")

    summary = {
        "status": "pass",
        "schema_version": 1,
        "fixture_count": len(reports),
        "native_execution": False,
        "executable_memory": False,
        "eligible_functions": sum(int(report["eligible_functions"]) for report in reports),
        "rejected_functions": sum(int(report["rejected_functions"]) for report in reports),
        "deopt_points": sum(int(report["deopt_points"]) for report in reports),
        "quickened_superinstructions": sum(
            int(report["quickened_superinstructions"]) for report in reports
        ),
        "candidate_optimizations": candidate_optimizations,
        "rejection_reasons": rejection_reasons,
        "expected_guards": expected_guards,
        "required_helpers": required_helpers,
        "fixtures": [report["path"] for report in reports],
    }
    (OUT_DIR / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "[pass] mid-tier plan smoke classified "
        f"{summary['eligible_functions']} eligible and "
        f"{summary['rejected_functions']} rejected function(s), "
        f"and wrote {rel(OUT_DIR / 'summary.json')}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
