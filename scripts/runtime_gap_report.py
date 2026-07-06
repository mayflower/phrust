#!/usr/bin/env python3
"""Build and validate the runtime compatibility gap closure report."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "known_gaps" / "runtime.jsonl"
KNOWN_GAPS_DOC = ROOT / "docs" / "runtime" / "known-gaps.md"
SUMMARY_DOC = ROOT / "docs" / "runtime" / "gap-closure-plan.md"
TARGET_DIR = ROOT / "target" / "runtime-gap-report"
REPORT_JSON = TARGET_DIR / "runtime-gap-report.json"
REPORT_MD = TARGET_DIR / "runtime-gap-report.md"

REQUIRED_CATEGORIES = (
    "references and Copy-on-Write",
    "arrays and array-key conversion",
    "foreach mutation/reference behavior",
    "warning channel and exact warning continuation",
    "Throwable/Error hierarchy and stack traces",
    "weak/strict type coercion",
    "include scope and cross-file declarations",
    "superglobals and $GLOBALS",
    "standard-library/extension routing",
)

PRACTICAL_HIGH = {
    "reference",
    "cow",
    "array",
    "foreach",
    "include",
    "autoload",
    "globals",
    "superglobal",
    "stdlib",
    "builtin",
    "callable",
    "type",
    "throwable",
    "exception",
}
ARCHITECTURE_HIGH = {
    "reference",
    "cow",
    "globals",
    "include",
    "autoload",
    "closure",
    "class",
    "object",
    "property",
    "foreach",
    "jit",
    "abi",
}


@dataclass(frozen=True)
class ManifestRow:
    lineno: int
    data: dict[str, Any]


def fail(message: str) -> None:
    print(f"runtime gap report error: {message}", file=sys.stderr)
    raise SystemExit(1)


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def load_manifest() -> list[ManifestRow]:
    if not MANIFEST.is_file():
        fail(f"missing manifest: {rel(MANIFEST)}")
    rows: list[ManifestRow] = []
    seen: dict[str, int] = {}
    with MANIFEST.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
            except json.JSONDecodeError as exc:
                fail(f"{rel(MANIFEST)}:{lineno}: invalid JSON: {exc}")
            if not isinstance(data, dict):
                fail(f"{rel(MANIFEST)}:{lineno}: row must be a JSON object")
            gap_id = data.get("id")
            if not isinstance(gap_id, str) or not gap_id:
                fail(f"{rel(MANIFEST)}:{lineno}: missing string id")
            if gap_id in seen:
                fail(f"duplicate runtime gap ID {gap_id!r} on lines {seen[gap_id]} and {lineno}")
            seen[gap_id] = lineno
            rows.append(ManifestRow(lineno=lineno, data=data))
    if not rows:
        fail(f"empty manifest: {rel(MANIFEST)}")
    return rows


def parse_markdown_table_ids(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    ids: set[str] = set()
    in_table = False
    for line in text.splitlines():
        if not line.startswith("|"):
            if in_table:
                break
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if not cells:
            continue
        if cells[0] == "ID":
            in_table = True
            continue
        if not in_table or set(cells[0]) <= {"-", ":"}:
            continue
        ids.add(cells[0].strip("`"))
    return ids


def expand_fixtures(row: dict[str, Any], lineno: int) -> list[str]:
    fixtures = row.get("fixtures", [])
    if not isinstance(fixtures, list):
        fail(f"{rel(MANIFEST)}:{lineno}: fixtures must be a list")
    concrete: set[str] = set()
    for fixture in fixtures:
        if not isinstance(fixture, str) or not fixture:
            fail(f"{rel(MANIFEST)}:{lineno}: fixture paths must be non-empty strings")
        if any(marker in fixture for marker in ("*", "?", "[")):
            fail(f"{rel(MANIFEST)}:{lineno}: wildcard belongs in fixture_patterns: {fixture}")
        fixture_path = ROOT / fixture
        if not fixture_path.is_file():
            fail(f"{rel(MANIFEST)}:{lineno}: fixture does not exist: {fixture}")
        concrete.add(fixture)

    patterns = row.get("fixture_patterns", []) or []
    if not isinstance(patterns, list):
        fail(f"{rel(MANIFEST)}:{lineno}: fixture_patterns must be a list")
    for pattern in patterns:
        if not isinstance(pattern, str) or not pattern:
            fail(f"{rel(MANIFEST)}:{lineno}: fixture_patterns entries must be strings")
        matches = sorted(path for path in ROOT.glob(pattern) if path.is_file())
        if not matches:
            fail(f"{rel(MANIFEST)}:{lineno}: fixture pattern matches no files: {pattern}")
        concrete.update(rel(path) for path in matches)
    return sorted(concrete)


def runtime_semantics_metadata(path: Path) -> dict[str, str]:
    metadata: dict[str, str] = {}
    if not path.is_file() or path.suffix != ".php":
        return metadata
    if "fixtures/runtime_semantics" not in rel(path):
        return metadata
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines()[:8]:
        text = line.strip()
        if not text.startswith("// runtime-semantics:") and not text.startswith(
            "# runtime-semantics:"
        ):
            continue
        _, payload = text.split(":", 1)
        for item in payload.split():
            if "=" not in item:
                continue
            key, value = item.split("=", 1)
            metadata[key] = value.strip('"')
    return metadata


def classify(row: dict[str, Any]) -> str:
    haystack = f"{row['id']} {row['feature']} {row['reference_behavior']} {row['current_behavior']}".lower()
    if any(token in haystack for token in ("strict_types", "weak", "coercion", "type mismatch")):
        return "weak/strict type coercion"
    if "foreach" in haystack:
        return "foreach mutation/reference behavior"
    if any(token in haystack for token in ("array", "key", "variadic", "unpack")):
        return "arrays and array-key conversion"
    if any(token in haystack for token in ("include", "require", "eval", "autoload", "cross-file")):
        return "include scope and cross-file declarations"
    if any(token in haystack for token in ("superglobal", "$globals", "globals", "sapi")):
        return "superglobals and $GLOBALS"
    if any(token in haystack for token in ("stdlib", "builtin", "extension", "stream", "wrapper", "abi", "jit", "opcache")):
        return "standard-library/extension routing"
    if any(
        token in haystack
        for token in (
            "throwable",
            "exception",
            "error hierarchy",
            "stack",
            "typeerror",
            "argumentcounterror",
            "unhandledmatcherror",
            "finally",
            "catch",
        )
    ):
        return "Throwable/Error hierarchy and stack traces"
    if any(token in haystack for token in ("warning", "diagnostic", "text", "wording", "fatal output")):
        return "warning channel and exact warning continuation"
    if any(token in haystack for token in ("reference", "by-reference", "copy-on-write", "cow", "alias")):
        return "references and Copy-on-Write"
    return "runtime object/control-flow compatibility"


def risk_level(row: dict[str, Any], keywords: set[str]) -> str:
    haystack = f"{row['id']} {row['feature']} {row['reference_behavior']} {row['current_behavior']}".lower()
    hits = sum(1 for keyword in keywords if keyword in haystack)
    if hits >= 2:
        return "high"
    if hits == 1:
        return "medium"
    return "low"


def score(practical: str, architecture: str, status: str) -> int:
    value = {"high": 3, "medium": 2, "low": 1}
    status_weight = {"known_gap": 3, "planned": 2, "deferred": 1, "implemented": 0}
    return value[practical] * 10 + value[architecture] * 5 + status_weight.get(status, 0)


def build_report(rows: list[ManifestRow]) -> dict[str, Any]:
    manifest_ids = {row.data["id"] for row in rows}
    doc_ids = parse_markdown_table_ids(KNOWN_GAPS_DOC)
    missing_in_doc = sorted(manifest_ids - doc_ids)
    extra_in_doc = sorted(doc_ids - manifest_ids)
    if missing_in_doc:
        fail(f"{rel(KNOWN_GAPS_DOC)} is missing manifest IDs: {missing_in_doc}")
    if extra_in_doc:
        fail(f"{rel(KNOWN_GAPS_DOC)} has IDs absent from manifest: {extra_in_doc}")

    declared_known_gap_ids: set[str] = set()
    for path in (ROOT / "fixtures" / "runtime_semantics").rglob("*.php"):
        metadata = runtime_semantics_metadata(path)
        if metadata.get("expect") == "known_gap" or "known_gaps" in path.parts:
            gap_id = metadata.get("known_gap")
            if not gap_id:
                fail(f"{rel(path)} declares a known gap without known_gap=<ID>")
            declared_known_gap_ids.add(gap_id)
    unknown_fixture_ids = sorted(declared_known_gap_ids - manifest_ids)
    if unknown_fixture_ids:
        fail(f"runtime-semantics known-gap fixtures use IDs absent from manifest: {unknown_fixture_ids}")

    entries: list[dict[str, Any]] = []
    categories = {category: {"total": 0, "open": 0, "implemented": 0} for category in REQUIRED_CATEGORIES}
    categories["runtime object/control-flow compatibility"] = {"total": 0, "open": 0, "implemented": 0}

    for manifest_row in rows:
        row = manifest_row.data
        gap_id = row["id"]
        fixtures = expand_fixtures(row, manifest_row.lineno)
        if not fixtures:
            fail(f"{rel(MANIFEST)}:{manifest_row.lineno}: {gap_id} has no concrete fixture evidence")
        status = row["status"]
        if status == "implemented" and not fixtures:
            fail(f"{rel(MANIFEST)}:{manifest_row.lineno}: implemented gap has no proof fixture")
        category = classify(row)
        if category not in categories:
            categories[category] = {"total": 0, "open": 0, "implemented": 0}
        practical = risk_level(row, PRACTICAL_HIGH)
        architecture = risk_level(row, ARCHITECTURE_HIGH)
        entry = {
            "id": gap_id,
            "feature": row["feature"],
            "status": status,
            "layer": row["layer"],
            "owner_area": row["owner_area"],
            "category": category,
            "practical_blocker_risk": practical,
            "vm_architecture_risk": architecture,
            "priority_score": score(practical, architecture, status),
            "reference_behavior": row["reference_behavior"],
            "current_behavior": row["current_behavior"],
            "fixture_evidence": fixtures,
        }
        entries.append(entry)
        categories[category]["total"] += 1
        if status == "implemented":
            categories[category]["implemented"] += 1
        else:
            categories[category]["open"] += 1

    missing_required = sorted(
        category for category in REQUIRED_CATEGORIES if categories[category]["total"] == 0
    )
    if missing_required:
        fail(f"required runtime gap categories are absent from report: {missing_required}")

    open_entries = [entry for entry in entries if entry["status"] != "implemented"]
    return {
        "source_manifest": rel(MANIFEST),
        "summary_doc": rel(SUMMARY_DOC),
        "summary": {
            "total": len(entries),
            "open": len(open_entries),
            "implemented": len(entries) - len(open_entries),
            "categories": categories,
        },
        "entries": sorted(entries, key=lambda item: (-item["priority_score"], item["id"])),
    }


def render_summary_doc(report: dict[str, Any]) -> str:
    lines = [
        "# Runtime Gap Closure Plan",
        "",
        "This document is generated by `scripts/runtime_gap_report.py` from",
        "`docs/known_gaps/runtime.jsonl`. Regenerate it with",
        "`just runtime-gap-report` and validate it with `just runtime-known-gaps`.",
        "",
        "The report treats every manifest row as executable debt: each row must have",
        "concrete fixture evidence, and runtime-semantics known-gap fixtures must use",
        "IDs present in the manifest.",
        "",
        "## Summary",
        "",
        f"- Total runtime gap rows: {report['summary']['total']}",
        f"- Open rows: {report['summary']['open']}",
        f"- Implemented rows retained for historical coverage: {report['summary']['implemented']}",
        f"- Machine-readable report: `target/runtime-gap-report/runtime-gap-report.json`",
        "",
        "## Required Risk Classes",
        "",
        "| Class | Total | Open | Implemented |",
        "| --- | ---: | ---: | ---: |",
    ]
    for category in REQUIRED_CATEGORIES:
        counts = report["summary"]["categories"][category]
        lines.append(
            f"| {category} | {counts['total']} | {counts['open']} | {counts['implemented']} |"
        )
    other = report["summary"]["categories"].get("runtime object/control-flow compatibility")
    if other and other["total"]:
        lines.append(
            f"| runtime object/control-flow compatibility | {other['total']} | {other['open']} | {other['implemented']} |"
        )
    lines.extend(["", "## Highest Priority Open Gaps", ""])
    lines.extend(
        [
            "| ID | Class | Practical blocker risk | VM architecture risk | Evidence |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    open_entries = [entry for entry in report["entries"] if entry["status"] != "implemented"]
    for entry in open_entries[:20]:
        evidence = ", ".join(f"`{fixture}`" for fixture in entry["fixture_evidence"][:3])
        remaining = len(entry["fixture_evidence"]) - 3
        if remaining > 0:
            evidence += f", and {remaining} more"
        lines.append(
            f"| `{entry['id']}` | {entry['category']} | "
            f"{entry['practical_blocker_risk']} | {entry['vm_architecture_risk']} | {evidence} |"
        )
    lines.extend(
        [
            "",
            "## Closure Rules",
            "",
            "- Add or update a fixture before adding a runtime gap row.",
            "- Use `fixture_patterns` only when the glob matches at least one committed fixture.",
            "- Keep runtime-semantics known-gap fixture IDs synchronized with `docs/known_gaps/runtime.jsonl`.",
            "- Move a row to `implemented` only when positive proof fixtures cover the behavior.",
            "- Do not commit generated files under `target/`.",
            "",
        ]
    )
    return "\n".join(lines)


def render_target_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Runtime Gap Report",
        "",
        f"Source: `{report['source_manifest']}`",
        "",
        "| ID | Status | Class | Practical | Architecture | Fixtures |",
        "| --- | --- | --- | --- | --- | ---: |",
    ]
    for entry in report["entries"]:
        lines.append(
            f"| `{entry['id']}` | {entry['status']} | {entry['category']} | "
            f"{entry['practical_blocker_risk']} | {entry['vm_architecture_risk']} | "
            f"{len(entry['fixture_evidence'])} |"
        )
    lines.append("")
    return "\n".join(lines)


def write_outputs(report: dict[str, Any], check: bool) -> None:
    summary_doc = render_summary_doc(report)
    if check:
        if not SUMMARY_DOC.is_file():
            fail(f"missing generated summary doc: {rel(SUMMARY_DOC)}")
        current = SUMMARY_DOC.read_text(encoding="utf-8")
        if current != summary_doc:
            fail(f"{rel(SUMMARY_DOC)} is stale; run `just runtime-gap-report`")

    TARGET_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_JSON.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    REPORT_MD.write_text(render_target_markdown(report), encoding="utf-8")
    if not check:
        SUMMARY_DOC.write_text(summary_doc, encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate the checked-in summary doc without rewriting it",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = build_report(load_manifest())
    write_outputs(report, args.check)
    print(
        "[ok] runtime gap report: "
        f"total={report['summary']['total']} "
        f"open={report['summary']['open']} "
        f"implemented={report['summary']['implemented']} "
        f"json={rel(REPORT_JSON)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
