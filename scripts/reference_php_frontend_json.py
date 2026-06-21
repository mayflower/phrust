#!/usr/bin/env python3
"""Emit normalized PHP compile-frontend JSON for one file."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PHP_VERSION = "8.5.7"
PHP_TIMEOUT_SECONDS = float(os.environ.get("REFERENCE_PHP_TIMEOUT_SECONDS", "30"))


def find_reference_php() -> tuple[Path | None, str | None]:
    configured = os.environ.get("REFERENCE_PHP")
    if configured:
        path = Path(configured)
        if path.exists() and os.access(path, os.X_OK):
            return path, None
        return None, f"REFERENCE_PHP is set but not executable: {path}"

    local = ROOT / "third_party" / "php-src" / "sapi" / "cli" / "php"
    if local.exists() and os.access(local, os.X_OK):
        return local, None

    system = shutil.which("php")
    if system:
        return Path(system), "using php from PATH; this may not be PHP 8.5.7"

    return None, "no PHP binary found; set REFERENCE_PHP or build the local reference"


def php_version(php: Path) -> str | None:
    try:
        process = subprocess.run(
            [str(php), "-r", "echo PHP_VERSION;"],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=PHP_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        return None
    if process.returncode != 0:
        return None
    return process.stdout.strip()


def lint_file(php: Path, file: Path) -> dict[str, object]:
    try:
        process = subprocess.run(
            [str(php), "-l", str(file)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=PHP_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        return {
            "file": str(file),
            "ok": False,
            "exit_code": None,
            "stdout": error.stdout or "",
            "stderr": (
                f"reference PHP timed out after {PHP_TIMEOUT_SECONDS:g}s "
                f"while linting {file}"
            ),
            "php_version": php_version(php) or "unknown",
            "mode": "lint_compile_frontend",
            "oracle": "php-lint",
            "classification": "timeout",
            "timeout": True,
        }
    classification = "accepted" if process.returncode == 0 else "rejected"
    return {
        "file": str(file),
        "ok": process.returncode == 0,
        "exit_code": process.returncode,
        "stdout": process.stdout,
        "stderr": process.stderr,
        "php_version": php_version(php) or "unknown",
        "mode": "lint_compile_frontend",
        "oracle": "php-lint",
        "classification": classification,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--file", required=True)
    args = parser.parse_args()

    php, warning = find_reference_php()
    if php is None:
        if os.environ.get("REFERENCE_PHP"):
            print(f"[fail] {warning}", file=sys.stderr)
            return 1
        print(json.dumps({"skipped": True, "reason": warning}, sort_keys=True))
        return 0
    if warning:
        print(f"[warn] {warning}", file=sys.stderr)

    print(json.dumps(lint_file(php, Path(args.file)), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
