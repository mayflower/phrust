#!/usr/bin/env python3
"""Generate and run the selected extension PHPT smoke manifest."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = REPO_ROOT / "fixtures/stdlib/phpt_extension_manifest.toml"
DEFAULT_PHP_SRC = REPO_ROOT / "third_party/php-src"
DEFAULT_OUT = REPO_ROOT / "target/stdlib/phpt-extension-smoke"
REQUIRED_CATEGORIES = {"standard", "spl", "json", "pcre", "date"}
ALLOWED_DISPOSITIONS = {"run", "skip", "known_gap", "expected_fail"}


def main() -> int:
    args = parse_args()
    manifest_path = args.manifest.resolve()
    out_dir = args.out.resolve()
    php_src = args.php_src.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    try:
        entries = read_manifest(manifest_path)
        generated_allowlist, selector_report = write_generated_allowlist(
            entries=entries,
            manifest_path=manifest_path,
            php_src=php_src,
            out_dir=out_dir,
        )
    except Exception as error:  # noqa: BLE001 - script boundary; print clean error.
        print(f"extension PHPT selector error: {error}", file=sys.stderr)
        return 2

    selector_report_path = out_dir / "selector-report.json"
    selector_report_path.write_text(
        json.dumps(selector_report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    empty_fixtures = out_dir / "empty-fixtures"
    empty_fixtures.mkdir(exist_ok=True)
    runner = REPO_ROOT / "target/debug/run-phpt-smoke"
    if not runner.is_file():
        print(
            f"extension PHPT selector error: missing runner {runner}; build php_testkit first",
            file=sys.stderr,
        )
        return 2

    command = [
        str(runner),
        "--fixtures",
        str(empty_fixtures),
        "--out",
        str(out_dir),
        "--rust-vm",
        str(REPO_ROOT / "target/debug/php-vm"),
        "--allowlist",
        str(generated_allowlist),
    ]
    completed = subprocess.run(command, cwd=REPO_ROOT, check=False)
    normalize_runner_report(out_dir, manifest_path, php_src)
    print(f"[ok] extension PHPT selector report: {selector_report_path}")
    return completed.returncode


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--php-src", type=Path, default=DEFAULT_PHP_SRC)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    return parser.parse_args()


def read_manifest(path: Path) -> list[dict[str, str]]:
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    entries = raw.get("test")
    if not isinstance(entries, list) or not entries:
        raise ValueError(f"{path} must contain at least one [[test]] entry")

    normalized: list[dict[str, str]] = []
    categories: set[str] = set()
    for index, entry in enumerate(entries, start=1):
        if not isinstance(entry, dict):
            raise ValueError(f"{path}: [[test]] entry {index} must be a table")
        normalized_entry = normalize_entry(path, index, entry)
        categories.add(normalized_entry["category"])
        normalized.append(normalized_entry)

    missing_categories = sorted(REQUIRED_CATEGORIES - categories)
    if missing_categories:
        raise ValueError(
            f"{path} is missing required extension categories: {', '.join(missing_categories)}"
        )
    if len(normalized) < 10:
        raise ValueError(f"{path} must select at least 10 PHPT files")
    return normalized


def normalize_entry(path: Path, index: int, entry: dict[str, Any]) -> dict[str, str]:
    def required_string(key: str) -> str:
        value = entry.get(key)
        if not isinstance(value, str) or not value:
            raise ValueError(f"{path}: [[test]] entry {index} requires string `{key}`")
        return value

    rel_path = required_string("path")
    if rel_path.startswith("/") or ".." in Path(rel_path).parts or not rel_path.endswith(".phpt"):
        raise ValueError(f"{path}: [[test]] entry {index} has invalid PHPT path `{rel_path}`")
    category = required_string("category")
    disposition = entry.get("disposition", "run")
    if disposition not in ALLOWED_DISPOSITIONS:
        raise ValueError(
            f"{path}: [[test]] entry {index} has unsupported disposition `{disposition}`"
        )
    reason = entry.get("reason", "")
    if disposition != "run" and not isinstance(reason, str):
        raise ValueError(f"{path}: [[test]] entry {index} has non-string reason")
    if disposition != "run" and not reason:
        raise ValueError(f"{path}: [[test]] entry {index} requires reason")
    return {
        "path": rel_path,
        "category": category,
        "disposition": disposition,
        "reason": reason if isinstance(reason, str) else "",
    }


def write_generated_allowlist(
    *,
    entries: list[dict[str, str]],
    manifest_path: Path,
    php_src: Path,
    out_dir: Path,
) -> tuple[Path, dict[str, Any]]:
    allowlist_path = out_dir / "generated-allowlist.toml"
    selected: list[dict[str, str]] = []
    missing_count = 0
    with allowlist_path.open("w", encoding="utf-8") as handle:
        handle.write("# Generated by scripts/stdlib/phpt_extension_selector.py\n\n")
        for entry in entries:
            source_path = php_src / entry["path"]
            disposition = entry["disposition"]
            reason = entry["reason"]
            if not source_path.is_file():
                missing_count += 1
                disposition = "skip"
                reason = (
                    f"selected upstream PHPT is unavailable under php-src root "
                    f"{php_src}"
                )
            write_allowlist_entry(handle, source_path, entry["category"], disposition, reason)
            selected.append(
                {
                    "path": entry["path"],
                    "category": entry["category"],
                    "disposition": disposition,
                    "reason": reason,
                }
            )
    report = {
        "manifest": relative_to_repo(manifest_path),
        "php_src": relative_to_repo(php_src),
        "selected": len(entries),
        "missing": missing_count,
        "categories": sorted({entry["category"] for entry in entries}),
        "generated_allowlist": relative_to_repo(allowlist_path),
        "tests": selected,
    }
    return allowlist_path, report


def write_allowlist_entry(
    handle: Any,
    path: Path,
    category: str,
    disposition: str,
    reason: str,
) -> None:
    handle.write("[[test]]\n")
    handle.write(f'path = "{escape_toml_string(str(path))}"\n')
    handle.write(f'category = "{escape_toml_string(category)}"\n')
    handle.write(f'disposition = "{disposition}"\n')
    if reason:
        handle.write(f'reason = "{escape_toml_string(reason)}"\n')
    handle.write("\n")


def normalize_runner_report(out_dir: Path, manifest_path: Path, php_src: Path) -> None:
    report_path = out_dir / "phpt-smoke-report.json"
    if not report_path.is_file():
        return
    report = json.loads(report_path.read_text(encoding="utf-8"))
    for result in report.get("results", []):
        for key in ("path", "generated_file"):
            value = result.get(key)
            if isinstance(value, str):
                result[key] = normalize_path(value, php_src)
    normalized = {
        "manifest": relative_to_repo(manifest_path),
        "total": report.get("total", 0),
        "pass": report.get("pass", 0),
        "fail": report.get("fail", 0),
        "skipped": report.get("skipped", 0),
        "known_gap": report.get("known_gap", 0),
        "expected_fail": report.get("expected_fail", 0),
        "results": report.get("results", []),
    }
    (out_dir / "normalized-report.json").write_text(
        json.dumps(normalized, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def normalize_path(value: str, php_src: Path) -> str:
    path = Path(value)
    try:
        return f"php-src/{path.relative_to(php_src).as_posix()}"
    except ValueError:
        return relative_to_repo(path)


def relative_to_repo(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return str(path)


def escape_toml_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


if __name__ == "__main__":
    raise SystemExit(main())
