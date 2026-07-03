#!/usr/bin/env python3
"""Compare Rust semantic frontend acceptance with the PHP reference."""

from __future__ import annotations

import argparse
import json
import os
import sys
import tomllib
from pathlib import Path

import reference_php_frontend_json
import run_semantic_fixtures


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_KNOWN_GAPS = ROOT / "fixtures" / "semantic" / "known_gaps.toml"
MATCH_ACCEPTED = "MatchAccepted"
MATCH_REJECTED = "MatchRejected"
RUST_ACCEPTS_REFERENCE_REJECTS = "RustAcceptsReferenceRejects"
RUST_REJECTS_REFERENCE_ACCEPTS = "RustRejectsReferenceAccepts"
REFERENCE_UNAVAILABLE = "ReferenceUnavailable"
KNOWN_GAP = "KnownGap"
SKIPPED = "Skipped"


def relative_fixture(path: Path) -> str:
    return path.resolve().relative_to(ROOT).as_posix()


def load_known_gaps(path: Path) -> dict[str, dict[str, object]]:
    if not path.exists():
        return {}
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    gaps: dict[str, dict[str, object]] = {}
    for gap in data.get("gap", []):
        if not isinstance(gap, dict):
            continue
        fixture = gap.get("fixture")
        if isinstance(fixture, str):
            gaps[fixture] = gap
    return gaps


def reference_result(php: Path, fixture: Path) -> dict[str, object]:
    return reference_php_frontend_json.lint_file(php, fixture)


def rust_diagnostic_ids(rust: dict[str, object]) -> list[str]:
    diagnostics = rust.get("semantic_diagnostics", [])
    if not isinstance(diagnostics, list):
        return []
    ids: list[str] = []
    for diagnostic in diagnostics:
        if isinstance(diagnostic, dict) and isinstance(diagnostic.get("id"), str):
            ids.append(diagnostic["id"])
    return ids


def status_for(reference_ok: bool, rust_ok: bool) -> str:
    if reference_ok and rust_ok:
        return MATCH_ACCEPTED
    if not reference_ok and not rust_ok:
        return MATCH_REJECTED
    if not reference_ok and rust_ok:
        return RUST_ACCEPTS_REFERENCE_REJECTS
    return RUST_REJECTS_REFERENCE_ACCEPTS


def skipped_rows(fixtures: list[Path], reason: str, status: str) -> list[dict[str, object]]:
    rows = []
    for fixture in fixtures:
        rust = run_semantic_fixtures.rust_frontend_result(fixture)
        rows.append(
            {
                "fixture": relative_fixture(fixture),
                "status": status,
                "reference_ok": None,
                "rust_ok": bool(rust.get("ok")),
                "harness_error": bool(rust.get("harness_error")),
                "rust_diagnostic_ids": rust_diagnostic_ids(rust),
                "known_gap": None,
                "notes": [reason],
            }
        )
    return rows


def summarize(rows: list[dict[str, object]], max_mismatches: int) -> dict[str, object]:
    unexpected = [
        row
        for row in rows
        if row["status"]
        in {RUST_ACCEPTS_REFERENCE_REJECTS, RUST_REJECTS_REFERENCE_ACCEPTS}
    ]
    return {
        "fixtures": len(rows),
        "matches": sum(
            1 for row in rows if row["status"] in {MATCH_ACCEPTED, MATCH_REJECTED}
        ),
        "mismatches": len(unexpected),
        "known_gaps": sum(1 for row in rows if row["status"] == KNOWN_GAP),
        "skips": sum(
            1 for row in rows if row["status"] in {REFERENCE_UNAVAILABLE, SKIPPED}
        ),
        "first_mismatches": [row["fixture"] for row in unexpected[:max_mismatches]],
    }


def print_text_report(summary: dict[str, object], rows: list[dict[str, object]]) -> None:
    for row in rows:
        status = row["status"]
        fixture = row["fixture"]
        reference_ok = row["reference_ok"]
        rust_ok = row["rust_ok"]
        print(
            f"[{status}] {fixture}: reference_ok={reference_ok} rust_ok={rust_ok}"
        )
    print(
        "[info] compared {fixtures} semantic fixture(s); matches={matches} "
        "mismatches={mismatches} known_gaps={known_gaps} skips={skips}".format(
            **summary
        )
    )
    if summary["first_mismatches"]:
        print("[fail] first semantic acceptance mismatch(es):")
        for fixture in summary["first_mismatches"]:
            print(f"  {fixture}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-root", default=str(run_semantic_fixtures.DEFAULT_FIXTURE_ROOT))
    parser.add_argument("--json", action="store_true", help="emit JSON summary")
    parser.add_argument("--strict", action="store_true", help="fail on skips or known gaps")
    parser.add_argument("--known-gaps", default=str(DEFAULT_KNOWN_GAPS))
    parser.add_argument("--max-mismatches", type=int, default=10)
    args = parser.parse_args()

    fixture_root = Path(args.fixture_root).resolve()
    fixtures = run_semantic_fixtures.iter_fixtures(fixture_root)
    known_gaps = load_known_gaps(Path(args.known_gaps))

    php, warning = reference_php_frontend_json.find_reference_php()
    rows: list[dict[str, object]]
    if php is None:
        reason = warning or "no PHP reference binary available"
        if os.environ.get("REFERENCE_PHP"):
            status = SKIPPED
        else:
            status = REFERENCE_UNAVAILABLE
        rows = skipped_rows(fixtures, reason, status)
    else:
        version = reference_php_frontend_json.php_version(php)
        if version != reference_php_frontend_json.EXPECTED_PHP_VERSION:
            reason = (
                "semantic acceptance comparison requires "
                f"PHP {reference_php_frontend_json.EXPECTED_PHP_VERSION}; "
                f"{php} reports {version or 'unknown'}"
            )
            rows = skipped_rows(fixtures, reason, SKIPPED)
        else:
            if warning and not args.json:
                print(f"[warn] {warning}")
            rows = []
            for fixture in fixtures:
                reference = reference_result(php, fixture)
                rust = run_semantic_fixtures.rust_frontend_result(fixture)
                rel = relative_fixture(fixture)
                if reference.get("timeout"):
                    row_status = SKIPPED
                    notes = [str(reference.get("stderr", "reference timed out"))]
                    reference_ok = None
                else:
                    reference_ok = bool(reference["ok"])
                    row_status = status_for(reference_ok, bool(rust.get("ok")))
                    notes = []
                    if row_status in {
                        RUST_ACCEPTS_REFERENCE_REJECTS,
                        RUST_REJECTS_REFERENCE_ACCEPTS,
                    }:
                        # Keep the raw reference evidence on mismatch rows so
                        # transient reference-side failures are attributable.
                        notes.append(
                            "reference exit_code="
                            f"{reference.get('exit_code')} "
                            f"stdout={str(reference.get('stdout', ''))[:200]!r} "
                            f"stderr={str(reference.get('stderr', ''))[:200]!r}"
                        )
                    if row_status in {
                        RUST_ACCEPTS_REFERENCE_REJECTS,
                        RUST_REJECTS_REFERENCE_ACCEPTS,
                    } and rel in known_gaps:
                        row_status = KNOWN_GAP
                        notes.append(str(known_gaps[rel].get("reason", "")))
                if rust.get("harness_error"):
                    notes.append(str(rust.get("stderr", "Rust frontend harness error")))
                rows.append(
                    {
                        "fixture": rel,
                        "status": row_status,
                        "reference_ok": reference_ok,
                        "rust_ok": bool(rust.get("ok")),
                        "harness_error": bool(rust.get("harness_error")),
                        "rust_diagnostic_ids": rust_diagnostic_ids(rust),
                        "known_gap": known_gaps.get(rel) if row_status == KNOWN_GAP else None,
                        "notes": notes,
                    }
                )

    summary = summarize(rows, args.max_mismatches)
    report = {"summary": summary, "rows": rows}

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_text_report(summary, rows)

    has_harness_error = any(bool(row.get("harness_error")) for row in rows)
    if summary["mismatches"] or has_harness_error:
        return 1
    if args.strict and (summary["known_gaps"] or summary["skips"]):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
