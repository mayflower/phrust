#!/usr/bin/env python3
"""Create a reduced-fixture scaffold from a real WordPress first failure."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from common import REPO_ROOT, json_dump, owner_suggestion  # noqa: E402


def main() -> int:
    args = parse_args()
    failure_path = find_failure_path(args.failure)
    failure = json.loads(failure_path.read_text(encoding="utf-8"))
    out_dir = Path(args.out) if args.out else failure_path.parent / "reduction"
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    slug = failure_slug(failure)
    php_path = out_dir / f"{slug}.php"
    readme_path = out_dir / "README.md"
    php_path.write_text(scaffold_php(failure), encoding="utf-8")
    readme_path.write_text(scaffold_readme(failure, php_path), encoding="utf-8")
    manifest = {"failure": str(failure_path), "php": str(php_path), "readme": str(readme_path)}
    json_dump(manifest, out_dir / "reduction-manifest.json")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--failure", default="", help="path to first-failure.json; defaults to newest target/wordpress-real run")
    parser.add_argument("--out", default="", help="output directory; defaults to <run>/reduction")
    return parser.parse_args()


def find_failure_path(value: str) -> Path:
    if value:
        path = Path(value).expanduser()
        if not path.is_absolute():
            path = REPO_ROOT / path
        if not path.is_file():
            raise SystemExit(f"first-failure.json not found: {path}")
        return path
    candidates = sorted(
        (REPO_ROOT / "target" / "wordpress-real").glob("*/first-failure.json"),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    if not candidates:
        raise SystemExit("no first-failure.json found under target/wordpress-real")
    return candidates[0]


def failure_slug(failure: dict[str, Any]) -> str:
    diagnostic = first_diagnostic_id(failure) or "environment"
    source = Path(str(failure.get("source_path") or "failure.php")).stem
    base = f"{diagnostic}-{source}".lower()
    return re.sub(r"[^a-z0-9]+", "-", base).strip("-") or "wordpress-failure"


def first_diagnostic_id(failure: dict[str, Any]) -> str | None:
    ids = failure.get("diagnostic_ids")
    if isinstance(ids, list):
        for item in ids:
            if isinstance(item, str) and item:
                return item
    return None


def scaffold_php(failure: dict[str, Any]) -> str:
    diagnostic = first_diagnostic_id(failure) or "environment"
    source_path = failure.get("source_path") or "unknown"
    line = failure.get("line") or "unknown"
    return f"""<?php
// Reduced fixture scaffold from real WordPress smoke.
// Failing source: {source_path}:{line}
// First diagnostic: {diagnostic}
// Replace this scaffold with the smallest generic PHP program that reproduces
// the behavior. Do not require a WordPress checkout from the final fixture.

echo "TODO reduce {diagnostic}\\n";
"""


def scaffold_readme(failure: dict[str, Any], php_path: Path) -> str:
    first_class = str(failure.get("first_failure_class") or "environment")
    diagnostic = first_diagnostic_id(failure)
    request = failure.get("request") if isinstance(failure.get("request"), dict) else {}
    owner = failure.get("owner_suggestion") or owner_suggestion(first_class, diagnostic)
    body = request.get("body") if isinstance(request, dict) else None
    body_class = "empty" if not body else "form" if request.get("headers", {}).get("Content-Type") == "application/x-www-form-urlencoded" else "present"
    return f"""# WordPress Failure Reduction Scaffold

Generated from `first-failure.json`.

- Fixture scaffold: `{php_path.name}`
- Failing source path: `{failure.get("source_path") or "unknown"}`
- Failing source line: `{failure.get("line") or "unknown"}`
- First diagnostic ID: `{diagnostic or "none"}`
- Failure class: `{first_class}`
- Candidate owner layer: `{failure.get("candidate_owner_layer") or "unknown"}`
- Suggested destination: `{owner}`

## Request Context

- Method: `{request.get("method", "n/a") if isinstance(request, dict) else "n/a"}`
- URI: `{request.get("path", "n/a") if isinstance(request, dict) else "n/a"}`
- Headers: `{json.dumps(request.get("headers", {}), sort_keys=True) if isinstance(request, dict) else "{}"}`
- Cookies: `not captured`
- Body classification: `{body_class}`

## Runtime Context

- Document root: `{failure.get("inputs", {}).get("docroot", "see smoke report") if isinstance(failure.get("inputs"), dict) else "see smoke report"}`
- Include path: `document root plus server-configured include roots`
- CWD: `document root for web phases`
- DB enabled: `{bool(failure.get("inputs", {}).get("db_enabled", False)) if isinstance(failure.get("inputs"), dict) else "see smoke report"}`

## Reduction Notes

1. Copy only the PHP shape needed to reproduce the diagnostic into `{php_path.name}`.
2. Remove WordPress names, files, database contents, credentials, and generated logs.
3. Move the final reduced fixture to the suggested destination above.
4. Add oracle expectations or PHPT metadata in that destination's existing style.
"""


if __name__ == "__main__":
    raise SystemExit(main())
