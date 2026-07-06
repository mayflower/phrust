#!/usr/bin/env python3
"""Validate checked known-gap manifests and their doc references."""

from __future__ import annotations

import datetime as dt
import json
import re
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_DIR = ROOT / "docs" / "known_gaps"
PHPT_ACCEPTED_NON_GREEN = (
    ROOT / "tests" / "phpt" / "manifests" / "runner-smoke-known-non-green.jsonl"
)

REQUIRED_FIELDS = {
    "id",
    "feature",
    "status",
    "layer",
    "fixtures",
    "reference_behavior",
    "current_behavior",
    "owner_area",
}
STATUSES = {"planned", "known_gap", "implemented", "deferred"}
DOC_LINKS = {
    ROOT / "docs" / "runtime" / "known-gaps.md": "docs/known_gaps/runtime.jsonl",
    ROOT / "docs" / "performance" / "known-gaps.md": "docs/known_gaps/performance.jsonl",
    ROOT / "docs" / "phpt" / "known-gaps.md": "docs/known_gaps/phpt-runner-smoke.jsonl",
}


def fail(message: str) -> None:
    print(f"known-gap manifest error: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_jsonl(path: Path, *, allow_empty: bool = False) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.is_file():
        fail(f"missing manifest: {path.relative_to(ROOT)}")
    if path.stat().st_size == 0:
        if allow_empty:
            return rows
        fail(f"empty manifest: {path.relative_to(ROOT)}")
    with path.open(encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as exc:
                fail(f"{path.relative_to(ROOT)}:{lineno}: invalid JSON: {exc}")
            if not isinstance(row, dict):
                fail(f"{path.relative_to(ROOT)}:{lineno}: row must be an object")
            rows.append(row)
    return rows


def relative(path: Path) -> str:
    return str(path.relative_to(ROOT))


def validate_row(path: Path, lineno: int, row: dict[str, Any]) -> None:
    missing = REQUIRED_FIELDS - row.keys()
    if missing:
        fail(f"{relative(path)}:{lineno}: missing required fields: {sorted(missing)}")

    gap_id = row["id"]
    if not isinstance(gap_id, str) or not gap_id:
        fail(f"{relative(path)}:{lineno}: id must be a non-empty string")

    for key in ("feature", "layer", "reference_behavior", "current_behavior", "owner_area"):
        if not isinstance(row[key], str) or not row[key].strip():
            fail(f"{relative(path)}:{lineno}: {key} must be a non-empty string")

    status = row["status"]
    if status not in STATUSES:
        fail(f"{relative(path)}:{lineno}: unsupported status {status!r}")

    fixtures = row["fixtures"]
    if not isinstance(fixtures, list):
        fail(f"{relative(path)}:{lineno}: fixtures must be a list")
    for fixture in fixtures:
        if not isinstance(fixture, str) or not fixture:
            fail(f"{relative(path)}:{lineno}: fixture paths must be non-empty strings")
        if any(marker in fixture for marker in ("*", "?", "[")):
            fail(f"{relative(path)}:{lineno}: wildcard fixture belongs in fixture_patterns: {fixture}")
        fixture_path = ROOT / fixture
        if not fixture_path.exists():
            fail(f"{relative(path)}:{lineno}: fixture does not exist: {fixture}")

    fixture_patterns = row.get("fixture_patterns", [])
    if fixture_patterns is None:
        fixture_patterns = []
    if not isinstance(fixture_patterns, list):
        fail(f"{relative(path)}:{lineno}: fixture_patterns must be a list")
    for pattern in fixture_patterns:
        if not isinstance(pattern, str) or not pattern:
            fail(f"{relative(path)}:{lineno}: fixture_patterns entries must be strings")

    examples = row.get("examples", [])
    if examples is None:
        examples = []
    if not isinstance(examples, list):
        fail(f"{relative(path)}:{lineno}: examples must be a list")
    for example in examples:
        if not isinstance(example, str) or not example:
            fail(f"{relative(path)}:{lineno}: examples entries must be strings")

    fixture_planned = row.get("fixture_planned", False)
    if not isinstance(fixture_planned, bool):
        fail(f"{relative(path)}:{lineno}: fixture_planned must be a boolean")
    if not fixtures and not fixture_patterns and not examples and not fixture_planned:
        fail(
            f"{relative(path)}:{lineno}: gap needs at least one concrete fixture, "
            "fixture_patterns entry, example, or fixture_planned=true"
        )
    if status == "implemented" and not fixtures:
        fail(f"{relative(path)}:{lineno}: implemented entry requires positive proof fixtures")

    expires_after = row.get("expires_after")
    if expires_after is not None:
        if not isinstance(expires_after, str):
            fail(f"{relative(path)}:{lineno}: expires_after must be YYYY-MM-DD")
        try:
            expiry = dt.date.fromisoformat(expires_after)
        except ValueError:
            fail(f"{relative(path)}:{lineno}: expires_after must be YYYY-MM-DD")
        if expiry < dt.date.today():
            fail(f"{relative(path)}:{lineno}: expired known gap {gap_id} after {expires_after}")

    accepted = row.get("accepted_non_green", [])
    if accepted is None:
        accepted = []
    if not isinstance(accepted, list):
        fail(f"{relative(path)}:{lineno}: accepted_non_green must be a list")
    if status == "implemented" and accepted and not row.get("historical_diagnostic_reason"):
        fail(
            f"{relative(path)}:{lineno}: implemented entry with accepted non-green "
            "requires historical_diagnostic_reason"
        )

    for item in accepted:
        if not isinstance(item, dict):
            fail(f"{relative(path)}:{lineno}: accepted_non_green entries must be objects")
        if not isinstance(item.get("path"), str) or not item["path"]:
            fail(f"{relative(path)}:{lineno}: accepted_non_green entry missing path")
        if item.get("outcome") not in {"FAIL", "BORK", "SKIP"}:
            fail(f"{relative(path)}:{lineno}: accepted_non_green entry has invalid outcome")
        if not isinstance(item.get("reason"), str) or not item["reason"]:
            fail(f"{relative(path)}:{lineno}: accepted_non_green entry missing reason")


def load_manifests() -> list[tuple[Path, int, dict[str, Any]]]:
    rows: list[tuple[Path, int, dict[str, Any]]] = []
    for path in sorted(MANIFEST_DIR.glob("*.jsonl")):
        with path.open(encoding="utf-8") as handle:
            for lineno, line in enumerate(handle, 1):
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError as exc:
                    fail(f"{relative(path)}:{lineno}: invalid JSON: {exc}")
                if not isinstance(row, dict):
                    fail(f"{relative(path)}:{lineno}: row must be an object")
                rows.append((path, lineno, row))
    if not rows:
        fail(f"no manifests found in {relative(MANIFEST_DIR)}")
    return rows


def parse_markdown_table_ids(path: Path, first_column: str) -> set[str]:
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
        if cells[0] == first_column:
            in_table = True
            continue
        if not in_table or set(cells[0]) <= {"-", ":"}:
            continue
        ids.add(cells[0].strip("`"))
    return ids


def validate_doc_links() -> None:
    for doc_path, manifest_ref in DOC_LINKS.items():
        if not doc_path.is_file():
            fail(f"missing known-gap doc: {relative(doc_path)}")
        text = doc_path.read_text(encoding="utf-8")
        if manifest_ref not in text:
            fail(f"{relative(doc_path)} must reference {manifest_ref}")


def validate_doc_coverage(rows_by_manifest: dict[str, dict[str, dict[str, Any]]]) -> None:
    runtime_ids = parse_markdown_table_ids(ROOT / "docs" / "runtime" / "known-gaps.md", "ID")
    runtime_manifest_ids = set(rows_by_manifest["docs/known_gaps/runtime.jsonl"])
    missing_runtime = sorted(gap_id for gap_id in runtime_ids if gap_id not in runtime_manifest_ids)
    if missing_runtime:
        fail(f"runtime docs have IDs missing from manifests: {missing_runtime}")
    undocumented_runtime = sorted(gap_id for gap_id in runtime_manifest_ids if gap_id not in runtime_ids)
    if undocumented_runtime:
        fail(f"runtime manifest has IDs missing from docs: {undocumented_runtime}")

    performance_ids = parse_markdown_table_ids(
        ROOT / "docs" / "performance" / "known-gaps.md", "Gap ID"
    )
    performance_manifest_ids = set(rows_by_manifest["docs/known_gaps/performance.jsonl"])
    missing_performance = sorted(
        gap_id for gap_id in performance_ids if gap_id not in performance_manifest_ids
    )
    if missing_performance:
        fail(f"performance docs have IDs missing from manifests: {missing_performance}")
    undocumented_performance = sorted(
        gap_id for gap_id in performance_manifest_ids if gap_id not in performance_ids
    )
    if undocumented_performance:
        fail(f"performance manifest has IDs missing from docs: {undocumented_performance}")


def validate_phpt_accepted(rows: list[dict[str, Any]]) -> None:
    expected = {
        (row["path"], row["outcome"], row["reason"])
        for row in load_jsonl(PHPT_ACCEPTED_NON_GREEN, allow_empty=True)
    }
    actual: set[tuple[str, str, str]] = set()
    for row in rows:
        for item in row.get("accepted_non_green", []) or []:
            actual.add((item["path"], item["outcome"], item["reason"]))
    missing = sorted(expected - actual)
    if missing:
        fail(f"PHPT accepted non-green entries missing from manifests: {missing}")


def main() -> int:
    manifest_rows = load_manifests()
    seen: dict[str, Path] = {}
    rows_by_id: dict[str, dict[str, Any]] = {}
    rows_by_manifest: dict[str, dict[str, dict[str, Any]]] = {}
    rows: list[dict[str, Any]] = []

    for path, lineno, row in manifest_rows:
        validate_row(path, lineno, row)
        gap_id = row["id"]
        if gap_id in seen:
            fail(f"duplicate known-gap ID {gap_id!r} in {relative(seen[gap_id])} and {relative(path)}")
        seen[gap_id] = path
        rows_by_id[gap_id] = row
        rows_by_manifest.setdefault(relative(path), {})[gap_id] = row
        rows.append(row)

    validate_doc_links()
    validate_doc_coverage(rows_by_manifest)
    validate_phpt_accepted(rows)

    print(f"[ok] validated {len(rows)} known-gap manifest entries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
