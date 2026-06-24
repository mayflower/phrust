#!/usr/bin/env python3
"""Generate the standard-library compatibility preflight inventory."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def workspace_crates() -> list[str]:
    cargo = read_text(ROOT / "Cargo.toml")
    crates: list[str] = []
    in_members = False
    for line in cargo.splitlines():
        stripped = line.strip()
        if stripped == "members = [":
            in_members = True
            continue
        if in_members and stripped == "]":
            break
        if in_members and stripped.startswith('"'):
            crates.append(stripped.strip('",'))
    return crates


def just_targets() -> list[str]:
    result = subprocess.run(
        ["just", "--list"],
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    targets: list[str] = []
    for line in result.stdout.splitlines():
        if line.startswith("    "):
            targets.append(line.strip().split()[0])
    return targets


def discover_reference_php() -> dict[str, str | bool]:
    env_php = os.environ.get("REFERENCE_PHP")
    pinned = ROOT / "third_party/php-src/sapi/cli/php"
    if env_php:
        path = Path(env_php)
        return {
            "status": "configured",
            "path": env_php,
            "exists": path.exists(),
            "executable": os.access(path, os.X_OK),
        }
    if pinned.exists():
        return {
            "status": "pinned-default",
            "path": str(pinned.relative_to(ROOT)),
            "exists": True,
            "executable": os.access(pinned, os.X_OK),
        }
    return {
        "status": "missing",
        "path": "",
        "exists": False,
        "executable": False,
        "message": (
            "Set REFERENCE_PHP=/absolute/path/to/php or run "
            "`nix develop -c just build-ref-php` to build "
            "third_party/php-src/sapi/cli/php from php-8.5.7."
        ),
    }


def file_status(paths: list[str]) -> dict[str, bool]:
    return {path: (ROOT / path).exists() for path in paths}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="target/stdlib/preflight.json")
    args = parser.parse_args()

    targets = just_targets()
    required_verification_targets = [
        "verify-foundation",
        "verify-lexer",
        "verify-frontend",
        "verify-runtime",
        "verify-stdlib",
        "verify-performance",
    ]
    stdlib_targets = [
        "verify-stdlib",
        "stdlib-docs",
        "stdlib-coverage",
        "diff-stdlib",
        "diff-streams",
        "diff-json-pcre-date",
        "diff-spl-reflection",
        "composer-smoke",
    ]
    docs = [
        "docs/stdlib-preflight.md",
        "docs/stdlib-standard-library.md",
        "docs/stdlib-extension-coverage.md",
        "docs/stdlib-composer-compatibility.md",
        "docs/stdlib-security-capabilities.md",
        "docs/stdlib-known-gaps.md",
    ]
    known_gap_docs = [
        "docs/runtime-known-gaps.md",
        "docs/runtime-semantics-known-gaps.md",
        "docs/stdlib-known-gaps.md",
    ]
    report = {
        "area": "standard-library",
        "php": {
            "series": "8.5",
            "version": "8.5.7",
            "tag": "php-8.5.7",
            "repository": "https://github.com/php/php-src.git",
        },
        "workspace_crates": workspace_crates(),
        "just_targets": targets,
        "required_verification_targets": {
            target: target in targets for target in required_verification_targets
        },
        "stdlib_targets": {target: target in targets for target in stdlib_targets},
        "reference_php": discover_reference_php(),
        "docs": file_status(docs),
        "known_gap_docs": file_status(known_gap_docs),
        "tools": {
            "just": shutil.which("just") or "",
            "jq": shutil.which("jq") or "",
            "python3": shutil.which("python3") or "",
            "diff": shutil.which("diff") or "",
        },
    }

    out = ROOT / args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"[ok] wrote {out.relative_to(ROOT)}")
    if report["reference_php"]["status"] == "missing":
        print(f"[warn] {report['reference_php']['message']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
