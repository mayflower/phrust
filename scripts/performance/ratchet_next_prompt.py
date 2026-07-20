#!/usr/bin/env python3
"""Generate the next complete performance architecture tranche from evidence."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from ratchet_schema import ROOT, load_json, rel


CATEGORIES = {
    "startup",
    "compile-transpile",
    "include-cache",
    "vm-execution",
    "server-responsiveness",
    "counter-instruction-regression",
    "correctness-blocker",
    "measurement-gap",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ratchet", action="append", type=Path, default=[])
    parser.add_argument("--compare", type=Path)
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "target/performance/ratchet/next-performance-prompt.md",
    )
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def load_optional(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    path = path if path.is_absolute() else ROOT / path
    if not path.is_file():
        return None
    return load_json(path)


def scenario_candidates(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for report in reports:
        for item in report.get("scenarios", []):
            if isinstance(item, dict):
                rows.append(item)
    return rows


def is_new_correctness_regression(item: dict[str, Any]) -> bool:
    """Return true only when evidence identifies a new branch regression.

    A known or unclassified pre-existing failure must not redirect a native
    architecture tranche into unrelated compatibility work. Producers can mark
    a regression directly, provide a pass-like baseline, or report a pass-to-
    fail status transition.
    """

    if item.get("correctness") != "fail":
        return False
    if item.get("correctness_regression") is True or item.get("regression") is True:
        return True
    baseline = item.get("baseline_correctness")
    if baseline is True or (
        isinstance(baseline, str) and baseline.lower() in {"pass", "ok", "green"}
    ):
        return True
    transition = str(item.get("correctness_transition", "")).lower().replace(" ", "")
    return transition in {"pass->fail", "ok->fail", "green->fail"}


def classify(
    reports: list[dict[str, Any]], compare: dict[str, Any] | None
) -> tuple[str, dict[str, Any], str]:
    scenarios = scenario_candidates(reports)
    regressions = [item for item in scenarios if is_new_correctness_regression(item)]
    if regressions:
        return (
            "correctness-blocker",
            regressions[0],
            "a correctness regression introduced by the current change outranks speed work",
        )
    if not scenarios:
        return "measurement-gap", {}, "no ratchet reports were available"
    if compare is not None:
        hard = compare.get("hard_regressions")
        if isinstance(hard, list) and hard:
            row = hard[0]
            metric = str(row.get("metric", ""))
            if metric.startswith("counter.") or "instruction" in metric:
                return (
                    "counter-instruction-regression",
                    row,
                    "deterministic counter regression",
                )
    scored: list[tuple[float, str, dict[str, Any], str]] = []
    for item in scenarios:
        metrics = item.get("metrics") if isinstance(item.get("metrics"), dict) else {}
        group = str(item.get("group", ""))
        external = float(
            metrics.get("external_wall_ms.p50", metrics.get("request_total_ms.p50", 0.0))
        )
        startup = float(metrics.get("startup_external_ms.p50", 0.0))
        compile_ms = float(metrics.get("compile_total_ms.p50", 0.0))
        execute = float(metrics.get("execute_ms.p50", 0.0))
        ttfb = float(metrics.get("ttfb_ms.p95", metrics.get("ttfb_ms.p50", 0.0)))
        counters = (
            item.get("counter_highlights")
            if isinstance(item.get("counter_highlights"), dict)
            else {}
        )
        instruction = float(
            metrics.get(
                "counter.instructions_executed",
                metrics.get("instructions_executed", counters.get("instructions_executed", 0.0)),
            )
        )
        if group == "server" and ttfb > 0:
            scored.append(
                (ttfb, "server-responsiveness", item, "server TTFB or tail latency dominates")
            )
        if external > 0 and startup / external >= 0.35:
            scored.append((startup, "startup", item, "startup is a large share of external wall time"))
        if compile_ms >= execute and compile_ms > 0:
            category = "include-cache" if any("cache" in key for key in metrics) else "compile-transpile"
            scored.append((compile_ms, category, item, "compile/transpile phase dominates"))
        if execute > compile_ms and execute > 0:
            scored.append((execute, "vm-execution", item, "execution phase dominates"))
        if instruction > 0:
            scored.append((instruction / 1000.0, "vm-execution", item, "instruction counters are high"))
    if not scored:
        return "measurement-gap", scenarios[0], "available reports lack timing or counter metrics"
    scored.sort(key=lambda row: row[0], reverse=True)
    _, category, item, reason = scored[0]
    return category, item, reason


def prompt(
    category: str,
    evidence: dict[str, Any],
    reason: str,
    inputs: list[Path],
    compare: Path | None,
) -> str:
    metrics = evidence.get("metrics") if isinstance(evidence.get("metrics"), dict) else {}
    counters = (
        evidence.get("counter_highlights")
        if isinstance(evidence.get("counter_highlights"), dict)
        else {}
    )
    scenario = evidence.get("scenario_id") or evidence.get("id") or "unknown"
    metric_lines = []
    for key in (
        "external_wall_ms.p50",
        "startup_external_ms.p50",
        "compile_total_ms.p50",
        "execute_ms.p50",
        "ttfb_ms.p95",
        "request_total_ms.p95",
        "counter.instructions_executed",
    ):
        if key in metrics:
            metric_lines.append(f"- {key}: {metrics[key]}")
    counter_lines = [f"- {key}: {value}" for key, value in list(counters.items())[:8]]
    artifact_lines = [f"- {rel(path if path.is_absolute() else ROOT / path)}" for path in inputs]
    if compare is not None:
        artifact_lines.append(f"- {rel(compare if compare.is_absolute() else ROOT / compare)}")
    if not metric_lines:
        metric_lines.append("- No decisive metric was present; improve measurement first.")
    if not counter_lines:
        counter_lines.append("- No counter highlights were present.")

    replacement = category not in {"measurement-gap", "correctness-blocker"}
    mode = (
        "MODE: [native-replacement]\n\n"
        "This is a complete production architecture tranche, not a compatibility migration."
        if replacement
        else "MODE: evidence or correctness repair before architecture replacement"
    )
    implementation_steps = (
        """1. Reproduce the baseline and preserve the raw artifacts under `target/performance/`.
2. Map the shared cost block and write a deletion manifest: legacy symbols, paths, callers, and production call edges.
3. Create one concrete contract under `docs/performance/native-replacement-contracts/` with the target architecture, allowed PHP-semantic slow paths, validation commands, and expected wall-time plus structural movement.
4. Implement the smallest **complete vertical replacement**. Delete or make the named old production route unreachable in this same change.
5. Run the native replacement guard, then the contract's PHP correctness and application gates.
6. Run ratchet current and compare with instrumentation-free timing separated from diagnostics.
7. Keep the tranche only when clean wall time and the shared structural counters improve together."""
        if replacement
        else """1. Reproduce and classify the missing evidence or new correctness regression.
2. Repair only the measurement or behavior needed to make the next architecture decision reliable.
3. Do not add a production wrapper, fallback, or compatibility route as part of this repair.
4. Regenerate the ratchet evidence and then select the architecture tranche."""
    )
    replacement_validation = (
        "python3 scripts/verify/native_replacement_guard.py --require-contract --diff-policy\n"
        if replacement
        else ""
    )
    replacement_acceptance = (
        """- Every removal target in the changed contract is absent or production-unreachable.
- No adapter, wrapper, bridge, dual route, shadow implementation, renamed legacy helper, or feature-gated old route recreates it.
- No new engine-fallback category is introduced; only explicitly contracted PHP-semantic slow paths remain.
- Clean wall time improves together with at least one shared structural metric such as helper boundaries, value traffic, call traffic, allocations, or RSS.
"""
        if replacement
        else ""
    )

    return f"""# Codex Performance Task: {category}

{mode}

## Problem evidence

- Scenario: `{scenario}`
- Category reason: {reason}
{chr(10).join(metric_lines)}

## Relevant counters

{chr(10).join(counter_lines)}

## Artifacts

{chr(10).join(artifact_lines) if artifact_lines else "- No artifact inputs were available."}

## Architecture decision

The ratchet selects the shared cost class `{category}`. It does not authorize a
one-helper micro-optimization or preserving the current internal route. For a
replacement tranche, externally observable PHP behavior is the compatibility
boundary; compatibility with the retired internal implementation is not a goal.

## Required implementation constraints

- Preserve stdout, stderr, exit status, diagnostics, PHP-visible behavior, side-effect order, and request semantics.
- Do not globally disable a PHP semantic path to make one scenario faster.
- Do not finish with old and new production routes coexisting.
- Do not satisfy the task with a wrapper, adapter, bridge, dual dispatch, shadow implementation, renamed old helper, or preparation-only refactor.
- Do not claim a speedup without clean before/after artifacts; diagnostic timing is ranking evidence only.
- Keep raw measurements under `target/performance/` and do not commit them.
- A known unrelated pre-existing correctness failure does not redirect this tranche. A correctness regression introduced by the current change does block completion.

## Steps

{implementation_steps}

## Validation commands

```bash
{replacement_validation}nix develop -c just perf-ratchet-baseline
nix develop -c just perf-ratchet-current
nix develop -c just perf-ratchet-compare
nix develop -c just perf-ratchet-next-prompt
```

## Acceptance criteria

{replacement_acceptance}- No correctness regression is introduced by the current change.
- The comparator reports no hard regressions.
- The regenerated next prompt no longer selects the same shallow bottleneck unless a deeper part of the same shared cost block remains.
"""


def run_self_test() -> int:
    for category in CATEGORIES:
        text = prompt(
            category,
            {"id": "self", "metrics": {"external_wall_ms.p50": 1.0}},
            "self-test",
            [],
            None,
        )
        assert f"Codex Performance Task: {category}" in text
        assert "Implement the smallest fix." not in text

    category, _, _ = classify([], None)
    assert category == "measurement-gap"

    known_failure = {
        "scenarios": [
            {
                "id": "known",
                "correctness": "fail",
                "correctness_regression": False,
                "metrics": {"execute_ms.p50": 10.0},
            }
        ]
    }
    category, _, _ = classify([known_failure], None)
    assert category == "vm-execution"

    new_failure = {
        "scenarios": [
            {
                "id": "new",
                "correctness": "fail",
                "correctness_regression": True,
                "metrics": {"execute_ms.p50": 10.0},
            }
        ]
    }
    category, _, _ = classify([new_failure], None)
    assert category == "correctness-blocker"

    architecture = prompt(
        "vm-execution",
        {"id": "self", "metrics": {"execute_ms.p50": 10.0}},
        "self-test",
        [],
        None,
    )
    assert "[native-replacement]" in architecture
    assert "smallest **complete vertical replacement**" in architecture
    assert "--require-contract --diff-policy" in architecture

    measurement = prompt("measurement-gap", {"id": "self"}, "self-test", [], None)
    assert "MODE: [native-replacement]" not in measurement

    print("[pass] ratchet_next_prompt self-test")
    return 0


def main() -> int:
    args = parse_args()
    if args.self_test:
        return run_self_test()
    reports = [
        load_json(path if path.is_absolute() else ROOT / path)
        for path in args.ratchet
        if (path if path.is_absolute() else ROOT / path).is_file()
    ]
    compare = load_optional(args.compare)
    category, evidence, reason = classify(reports, compare)
    if category not in CATEGORIES:
        raise SystemExit(f"internal error: invalid category {category}")
    out = args.out if args.out.is_absolute() else ROOT / args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(prompt(category, evidence, reason, args.ratchet, args.compare), encoding="utf-8")
    print(f"[pass] wrote next performance prompt {rel(out)} ({category})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
