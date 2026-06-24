#!/usr/bin/env python3
"""Opt-in local Composer project smoke for runtime semantics."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_VM = ROOT / "target/debug/php-vm"
DIAGNOSTIC_RE = re.compile(r"E_PHP_[A-Z0-9_]+")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, default=ROOT / "target/runtime-semantics/composer-smoke")
    parser.add_argument("--rust-vm", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_VM)))
    parser.add_argument("--limit", type=int, default=int(os.getenv("PHRUST_COMPOSER_SMOKE_LIMIT", "20")))
    return parser.parse_args()


def normalize(text: str, project: Path) -> str:
    return text.replace(str(project), "$PROJECT").replace(str(ROOT), "$ROOT")


def candidate_files(project: Path, limit: int) -> list[Path]:
    smoke_dir = project / "smoke"
    roots = [smoke_dir] if smoke_dir.is_dir() else [project]
    found: list[Path] = []
    for root in roots:
        for path in sorted(root.rglob("*.php")):
            rel = path.relative_to(project)
            if "vendor" in rel.parts and not str(rel).startswith("vendor/autoload.php"):
                continue
            found.append(path)
            if len(found) >= limit:
                return found
    return found


def run_fixture(vm: Path, project: Path, path: Path) -> dict[str, object]:
    completed = subprocess.run(
        [str(vm), "run", str(path)],
        cwd=project,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=5.0,
        check=False,
    )
    stderr = normalize(completed.stderr, project)
    stdout = normalize(completed.stdout, project)
    return {
        "path": str(path.relative_to(project)),
        "exit": completed.returncode,
        "stdout": stdout,
        "stderr": stderr,
        "diagnostics": sorted(set(DIAGNOSTIC_RE.findall(stderr))),
    }


def write_report(out: Path, report: dict[str, object]) -> None:
    out.mkdir(parents=True, exist_ok=True)
    (out / "runtime-composer-smoke-report.json").write_text(
        json.dumps(report, indent=2), encoding="utf-8"
    )


def main() -> int:
    args = parse_args()
    fixture_dir = os.getenv("PHPRUST_COMPOSER_FIXTURE_DIR")
    if not fixture_dir:
        write_report(args.out, {"status": "skip", "reason": "PHPRUST_COMPOSER_FIXTURE_DIR is not set"})
        print("[skip] set PHPRUST_COMPOSER_FIXTURE_DIR to run the local Composer smoke.")
        return 0

    project = Path(fixture_dir).resolve()
    if not project.is_dir():
        write_report(args.out, {"status": "fail", "reason": f"not a directory: {project}"})
        print(f"[fail] PHPRUST_COMPOSER_FIXTURE_DIR is not a directory: {project}")
        return 2
    if not args.rust_vm.exists():
        write_report(args.out, {"status": "skip", "reason": f"php-vm binary not found: {args.rust_vm}"})
        print(f"[skip] php-vm binary not found: {args.rust_vm}")
        return 0

    files = candidate_files(project, args.limit)
    if not files:
        write_report(args.out, {"status": "skip", "reason": f"no PHP smoke files found under {project}"})
        print(f"[skip] no PHP smoke files found under {project}")
        return 0

    results = [run_fixture(args.rust_vm, project, path) for path in files]
    diagnostics = sorted({diag for result in results for diag in result["diagnostics"]})
    passes = sum(1 for result in results if result["exit"] == 0)
    report = {
        "status": "ok",
        "project": str(project),
        "total": len(results),
        "pass": passes,
        "gap_or_failure": len(results) - passes,
        "diagnostics": diagnostics,
        "results": results,
    }
    write_report(args.out, report)
    print(
        f"[ok] runtime composer smoke: total={len(results)} pass={passes} "
        f"gap_or_failure={len(results) - passes} path={args.out}"
    )
    if diagnostics:
        print("[gap] " + ", ".join(diagnostics))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
