#!/usr/bin/env python3
"""Standard-library builtin/function differential harness."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "stdlib"))
from normalize_php_output import normalize  # noqa: E402


ROOT = Path(__file__).resolve().parents[1]
EXPECTATIONS = {"pass", "fail", "skip", "known_gap"}


@dataclass
class Fixture:
    path: Path
    test_id: str
    area: str
    expect: str = "pass"
    known_gap: str | None = None
    metadata: dict[str, str] = field(default_factory=dict)


def main() -> int:
    args = parse_args()
    report = run(args)
    summary = report["summary"]
    print(
        "[ok] standard-library diff report: "
        f"total={summary['total']} pass={summary['pass']} fail={summary['fail']} "
        f"skip={summary['skip']} known_gap={summary['known_gap']} "
        f"path={report['report_path']}"
    )
    return 1 if summary["fail"] else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixtures", default="tests/fixtures/stdlib/_harness")
    parser.add_argument("--out", default="target/stdlib/diff")
    parser.add_argument("--area", action="append", default=[])
    parser.add_argument("--file", action="append", default=[])
    parser.add_argument(
        "--vm-binary",
        default=os.environ.get("PHRUST_STDLIB_VM", ""),
        help="Path to a prebuilt php_vm_cli binary; defaults to building once.",
    )
    return parser.parse_args()


def run(args: argparse.Namespace) -> dict:
    out_dir = ROOT / args.out
    out_dir.mkdir(parents=True, exist_ok=True)

    fixtures_root = ROOT / args.fixtures
    known_gaps = load_known_gaps(fixtures_root / "known_gaps.toml")
    fixtures = discover_fixtures(fixtures_root, args.area, args.file)
    reference = discover_reference_php()
    vm_binary = discover_vm_binary(args, fixtures, reference)

    results = [
        compare_fixture(fixture, reference, vm_binary, known_gaps, out_dir)
        for fixture in fixtures
    ]
    summary = summarize(results)
    report_path = out_dir / "stdlib-diff-report.json"
    report = {
        "fixtures_root": str(fixtures_root),
        "reference_php": reference,
        "vm_binary": vm_binary,
        "summary": summary,
        "results": results,
        "report_path": str(report_path),
    }
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def discover_vm_binary(
    args: argparse.Namespace,
    fixtures: list[Fixture],
    reference: dict[str, str | bool],
) -> dict[str, str | bool]:
    needs_vm = any(fixture.expect == "pass" for fixture in fixtures) and bool(reference["available"])
    if not needs_vm:
        return {"status": "not-needed", "path": "", "available": False}

    if args.vm_binary:
        path = Path(args.vm_binary)
        return {
            "status": "configured",
            "path": str(path),
            "available": path.exists() and os.access(path, os.X_OK),
        }

    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    binary_name = "php-vm.exe" if sys.platform == "win32" else "php-vm"
    binary = target_dir / "debug" / binary_name
    if not binary.exists() or not os.access(binary, os.X_OK):
        build = run_command(["cargo", "build", "-q", "-p", "php_vm_cli", "--bin", "php-vm"])
        if build.returncode != 0:
            return {
                "status": "build-failed",
                "path": str(binary),
                "available": False,
                "message": build.stderr or build.stdout,
            }
    return {
        "status": "built-once",
        "path": str(binary),
        "available": binary.exists() and os.access(binary, os.X_OK),
    }


def discover_reference_php() -> dict[str, str | bool]:
    env_php = os.environ.get("REFERENCE_PHP")
    if env_php:
        path = Path(env_php)
        return {
            "status": "configured",
            "path": env_php,
            "available": path.exists() and os.access(path, os.X_OK),
        }
    pinned = ROOT / "third_party/php-src/sapi/cli/php"
    if pinned.exists() and os.access(pinned, os.X_OK):
        return {
            "status": "pinned-default",
            "path": str(pinned),
            "available": True,
        }
    return {
        "status": "missing",
        "path": "",
        "available": False,
        "message": (
            "Set REFERENCE_PHP=/absolute/path/to/php or run "
            "`nix develop -c just build-ref-php` to build "
            "third_party/php-src/sapi/cli/php from php-8.5.7."
        ),
    }


def load_known_gaps(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    return dict(data.get("known_gaps", {}))


def discover_fixtures(root: Path, areas: list[str], files: list[str]) -> list[Fixture]:
    selected = [ROOT / item for item in files]
    if not selected:
        selected = sorted(path for path in root.rglob("*.php") if path.is_file())
    fixtures = [load_fixture(path) for path in selected]
    if areas:
        fixtures = [fixture for fixture in fixtures if fixture.area in areas]
    return fixtures


def load_fixture(path: Path) -> Fixture:
    metadata: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines()[:10]:
        text = line.strip()
        if not text.startswith("// stdlib-diff:"):
            continue
        for item in text[len("// stdlib-diff:") :].strip().split():
            if "=" not in item:
                continue
            key, value = item.split("=", 1)
            metadata[key] = value.strip('"')
    test_id = metadata.get("id")
    area = metadata.get("area")
    if not test_id or not area:
        raise HarnessError(f"{path}: fixture must declare id= and area=")
    expect = metadata.get("expect", "pass")
    if expect not in EXPECTATIONS:
        raise HarnessError(f"{path}: unsupported expect={expect}")
    return Fixture(
        path=path,
        test_id=test_id,
        area=area,
        expect=expect,
        known_gap=metadata.get("known_gap"),
        metadata=metadata,
    )


def compare_fixture(
    fixture: Fixture,
    reference: dict[str, str | bool],
    vm_binary: dict[str, str | bool],
    known_gaps: dict[str, str],
    out_dir: Path,
) -> dict:
    if fixture.expect == "skip":
        return result(fixture, "skip", "fixture marked skip")
    if fixture.expect == "known_gap":
        if not fixture.known_gap or fixture.known_gap not in known_gaps:
            return result(fixture, "fail", "known_gap fixture lacks explicit known-gap ID")
        return result(fixture, "known_gap", known_gaps[fixture.known_gap])
    if not reference["available"]:
        return result(fixture, "skip", str(reference.get("message", "REFERENCE_PHP unavailable")))
    if not vm_binary["available"]:
        return result(
            fixture,
            "fail",
            str(vm_binary.get("message", "php_vm_cli binary unavailable")),
        )

    ref = run_command([str(reference["path"]), str(fixture.path)])
    rust = run_command([str(vm_binary["path"]), "run", str(fixture.path)])
    ref_norm = normalized_run(ref)
    rust_norm = normalized_run(rust)

    detail = {
        "reference": ref_norm,
        "rust": rust_norm,
    }
    detail_path = out_dir / f"{fixture.test_id}.json"
    detail_path.write_text(json.dumps(detail, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if ref_norm == rust_norm:
        return result(fixture, "pass", "matched reference", detail_path)
    return result(fixture, "fail", "normalized output differs", detail_path)


def run_command(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def normalized_run(run: subprocess.CompletedProcess[str]) -> dict[str, str | int]:
    return {
        "exit": run.returncode,
        "stdout": normalize(run.stdout),
        "stderr": normalize(run.stderr),
    }


def result(fixture: Fixture, status: str, message: str, detail_path: Path | None = None) -> dict:
    entry = {
        "id": fixture.test_id,
        "area": fixture.area,
        "path": str(fixture.path),
        "expect": fixture.expect,
        "status": status,
        "message": message,
    }
    if fixture.known_gap:
        entry["known_gap"] = fixture.known_gap
    if detail_path:
        entry["detail_path"] = str(detail_path)
    return entry


def summarize(results: list[dict]) -> dict[str, int]:
    summary = {"total": len(results), "pass": 0, "fail": 0, "skip": 0, "known_gap": 0}
    for item in results:
        summary[item["status"]] += 1
    return summary


class HarnessError(RuntimeError):
    pass


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except HarnessError as error:
        print(f"[error] {error}", file=sys.stderr)
        raise SystemExit(2)
