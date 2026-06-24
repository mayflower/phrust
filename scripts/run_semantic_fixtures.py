#!/usr/bin/env python3
"""Run Semantic frontend semantic fixtures through the Rust frontend skeleton."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURE_ROOT = ROOT / "fixtures" / "semantic"
DEFAULT_SNAPSHOT_DIR = DEFAULT_FIXTURE_ROOT / "snapshots"
FRONTEND_TIMEOUT_SECONDS = float(os.environ.get("PHRUST_FRONTEND_TIMEOUT_SECONDS", "30"))
FRONTEND_BUILD_TIMEOUT_SECONDS = float(
    os.environ.get("PHRUST_FRONTEND_BUILD_TIMEOUT_SECONDS", "120")
)
FRONTEND_BINARY = ROOT / "target" / "debug" / "php-frontend"
FRONTEND_FORCE_BUILD = os.environ.get("PHRUST_FRONTEND_FORCE_BUILD") == "1"
_frontend_build_error: dict[str, object] | None = None
_frontend_ready = False


def iter_fixtures(root: Path) -> list[Path]:
    if not root.exists():
        return []
    return sorted(path for path in root.rglob("*.php") if path.is_file())


def relative_fixture(path: Path) -> str:
    return path.resolve().relative_to(ROOT).as_posix()


def ensure_frontend_binary() -> dict[str, object] | None:
    global _frontend_build_error, _frontend_ready
    if _frontend_ready:
        return _frontend_build_error
    if FRONTEND_BINARY.exists() and not FRONTEND_FORCE_BUILD:
        _frontend_ready = True
        return None
    try:
        process = subprocess.run(
            ["cargo", "build", "--quiet", "-p", "php_frontend_cli"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=FRONTEND_BUILD_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        _frontend_build_error = {
            "ok": False,
            "harness_error": True,
            "stderr": (
                f"frontend build timed out after {FRONTEND_BUILD_TIMEOUT_SECONDS:g}s"
            ),
            "stdout": error.stdout or "",
        }
        _frontend_ready = True
        return _frontend_build_error
    if process.returncode != 0:
        _frontend_build_error = {
            "ok": False,
            "harness_error": True,
            "stderr": process.stderr,
            "stdout": process.stdout,
        }
    elif not FRONTEND_BINARY.exists():
        _frontend_build_error = {
            "ok": False,
            "harness_error": True,
            "stderr": f"frontend binary was not built: {FRONTEND_BINARY}",
            "stdout": process.stdout,
        }
    _frontend_ready = True
    return _frontend_build_error


def rust_frontend_result(fixture: Path) -> dict[str, object]:
    build_error = ensure_frontend_binary()
    if build_error is not None:
        build_error["file"] = str(fixture)
        build_error["parser_diagnostics"] = 1
        build_error["semantic_diagnostics"] = []
        return build_error
    try:
        process = subprocess.run(
            [
                str(FRONTEND_BINARY),
                "analyze",
                str(fixture),
                "--format",
                "json",
            ],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=FRONTEND_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        return {
            "file": str(fixture),
            "ok": False,
            "parser_diagnostics": 1,
            "semantic_diagnostics": [],
            "stdout": error.stdout or "",
            "stderr": f"frontend timed out after {FRONTEND_TIMEOUT_SECONDS:g}s",
            "harness_error": True,
            "timeout": True,
        }
    if process.returncode != 0:
        return {
            "file": str(fixture),
            "ok": False,
            "parser_diagnostics": 1,
            "semantic_diagnostics": [],
            "stderr": process.stderr,
            "harness_error": True,
        }
    parsed = json.loads(process.stdout)
    parsed["file"] = str(fixture)
    return parsed


def snapshot_name(fixture: Path) -> str:
    rel = relative_fixture(fixture)
    return rel.removesuffix(".php").replace("/", "__") + ".snap"


def format_snapshot(fixture: Path, result: dict[str, object]) -> str:
    module = result.get("module", {})
    if not isinstance(module, dict):
        module = {}
    diagnostics = result.get("semantic_diagnostics", [])
    if not isinstance(diagnostics, list):
        diagnostics = []
    symbols = module.get("symbols", [])
    if not isinstance(symbols, list):
        symbols = []
    return "\n".join(
        [
            f"source: {relative_fixture(fixture)}",
            f"parse_ok: {str(result.get('parser_diagnostics') == 0).lower()}",
            f"semantic_ok: {str(bool(result.get('ok'))).lower()}",
            "diagnostics:",
            *format_diagnostics(diagnostics),
            "symbols:",
            *format_symbols(symbols),
            "hir_summary:",
            f"  statements: {len(module.get('statements', []))}",
            f"  expressions: {len(module.get('expressions', []))}",
            f"  types: {len(module.get('types', []))}",
            f"  const_exprs: {len(module.get('const_exprs', []))}",
            f"  class_likes: {len(module.get('class_likes', []))}",
            f"  methods: {len(module.get('methods', []))}",
            f"  properties: {len(module.get('properties', []))}",
            "",
        ]
    )


def format_diagnostics(diagnostics: list[object]) -> list[str]:
    if not diagnostics:
        return ["  []"]
    lines: list[str] = []
    for diagnostic in diagnostics:
        if not isinstance(diagnostic, dict):
            continue
        span = diagnostic.get("span")
        if isinstance(span, dict):
            span_text = f"{span.get('start')}..{span.get('end')}"
        else:
            span_text = "null"
        lines.extend(
            [
                f"  - id: {diagnostic.get('id')}",
                f"    severity: {diagnostic.get('severity')}",
                f"    phase: {diagnostic.get('phase')}",
                f"    span: {span_text}",
            ]
        )
    return lines or ["  []"]


def format_symbols(symbols: list[object]) -> list[str]:
    if not symbols:
        return ["  []"]
    lines: list[str] = []
    for symbol in symbols:
        if not isinstance(symbol, dict):
            continue
        lines.append(
            "  - "
            f"{symbol.get('kind')} "
            f"{symbol.get('name') or symbol.get('fqn') or symbol.get('canonical_name')}"
        )
    return lines or ["  []"]


def write_snapshots(fixtures: list[Path], snapshot_dir: Path) -> int:
    snapshot_dir.mkdir(parents=True, exist_ok=True)
    written = 0
    for fixture in fixtures:
        result = rust_frontend_result(fixture)
        snapshot_path = snapshot_dir / snapshot_name(fixture)
        snapshot_path.write_text(format_snapshot(fixture, result), encoding="utf-8")
        written += 1
    return written


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture-root", default=str(DEFAULT_FIXTURE_ROOT))
    parser.add_argument("--json", action="store_true", help="emit JSON array")
    parser.add_argument(
        "--write-snapshots",
        action="store_true",
        help="write deterministic semantic snapshots",
    )
    parser.add_argument("--snapshot-dir", default=str(DEFAULT_SNAPSHOT_DIR))
    args = parser.parse_args()

    fixtures = iter_fixtures(Path(args.fixture_root))

    if args.write_snapshots:
        written = write_snapshots(fixtures, Path(args.snapshot_dir))
        print(f"[info] wrote {written} semantic snapshot(s) to {args.snapshot_dir}")
        return 0

    if args.json:
        results = [rust_frontend_result(fixture) for fixture in fixtures]
        print(json.dumps(results, indent=2, sort_keys=True))
        return 1 if any(result.get("harness_error") for result in results) else 0
    else:
        has_harness_error = False
        for fixture in fixtures:
            result = rust_frontend_result(fixture)
            has_harness_error = has_harness_error or bool(result.get("harness_error"))
            status = "ok" if result.get("ok") else "not ok"
            print(
                f"[{status}] {result['file']}: "
                f"parser_diagnostics={result.get('parser_diagnostics', 'unknown')} "
                f"semantic_diagnostics={len(result.get('semantic_diagnostics', []))}",
                flush=True,
            )
        print(f"[info] checked {len(fixtures)} semantic fixture(s)")
        return 1 if has_harness_error else 0


if __name__ == "__main__":
    raise SystemExit(main())
