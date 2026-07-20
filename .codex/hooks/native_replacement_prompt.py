#!/usr/bin/env python3
"""Activate strict replacement mode for architecture-cutover prompts.

Codex sends one JSON event on stdin. For matching prompts this hook persists a
small per-session marker under the Git directory and injects additional
developer context. The Stop hook consumes that marker and refuses to finish
until the repository replacement guard passes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


OPT_OUT = re.compile(r"\[native-replacement:(?:off|disable)\]", re.IGNORECASE)
EXPLICIT = re.compile(
    r"\[(?:native|architecture)-replacement(?::on)?\]|"
    r"production[ _-]architecture[ _-]replacement|"
    r"cranelift[ _-]only[ _-]cutover",
    re.IGNORECASE,
)
NATIVE = re.compile(
    r"\b(?:cranelift|jit|native|warm[ _-]?path|hot[ _-]?path|runtime[ _-]?abi|"
    r"execution[ _-]?engine|ausf(?:ü|ue)hrungsengine)\b",
    re.IGNORECASE,
)
REPLACEMENT = re.compile(
    r"\b(?:remove|delete|eliminate|replace|retire|cut[ _-]?over|shut[ _-]?off|"
    r"make\s+.+?unreachable|entfern(?:e|en|t)|ersetz(?:e|en|t)|"
    r"eliminier(?:e|en|t)|abschalt(?:en|et)|abl(?:ö|oe)s(?:en|e|t)|"
    r"vollst(?:ä|ae)ndig)\b",
    re.IGNORECASE,
)
LEGACY_ROUTE = re.compile(
    r"\b(?:fallback|wrapper|adapter|bridge|safe[ _-]?path|safepath|legacy|"
    r"old[ _-]?(?:route|path|api)|alte[nr]?[ _-]?(?:route|pfad|strecke)|"
    r"interpreter|generic[ _-]?(?:binder|runtime|helper|route)|"
    r"dual[ _-]?(?:dispatch|route|path)|shadow[ _-]?(?:route|path))\b",
    re.IGNORECASE,
)
ARCHITECTURE = re.compile(
    r"\b(?:architecture|architektur|execution[ _-]?(?:route|path)|"
    r"ausf(?:ü|ue)hrungsstrecke|production[ _-]?(?:route|path))\b",
    re.IGNORECASE,
)


def read_event() -> dict[str, Any]:
    try:
        document = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"invalid Codex hook input: {error}") from error
    if not isinstance(document, dict):
        raise SystemExit("invalid Codex hook input: expected an object")
    return document


def repository_root(cwd: str | None) -> Path | None:
    start = Path(cwd or os.getcwd())
    result = subprocess.run(
        ["git", "-C", str(start), "rev-parse", "--show-toplevel"],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0 or not result.stdout.strip():
        return None
    return Path(result.stdout.strip()).resolve()


def state_directory(root: Path) -> Path:
    result = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--git-path", "codex-native-replacement"],
        text=True,
        capture_output=True,
        check=True,
    )
    path = Path(result.stdout.strip())
    return path if path.is_absolute() else root / path


def state_path(root: Path, session_id: str) -> Path:
    digest = hashlib.sha256(session_id.encode("utf-8")).hexdigest()
    return state_directory(root) / f"{digest}.json"


def prompt_requests_replacement(prompt: str) -> bool:
    if OPT_OUT.search(prompt):
        return False
    if EXPLICIT.search(prompt):
        return True
    return bool(
        REPLACEMENT.search(prompt)
        and LEGACY_ROUTE.search(prompt)
        and (NATIVE.search(prompt) or ARCHITECTURE.search(prompt))
    )


def write_state(path: Path, document: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=path.name, dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(document, handle, indent=2, sort_keys=True)
            handle.write("\n")
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def additional_context() -> str:
    return """PHRUST NATIVE REPLACEMENT MODE IS ACTIVE FOR THIS TURN.

This is a production architecture replacement, not a compatibility migration.
Before changing code, create one concrete JSON contract under
`docs/performance/native-replacement-contracts/` naming the legacy symbols,
paths, and call edges that this turn will remove. The final production tree may
not contain an adapter, wrapper, bridge, dual route, shadow implementation,
renamed legacy helper, feature-gated old path, or generic engine fallback that
recreates the removed route. Internal compatibility with that route is not a
requirement; externally observable PHP 8.5 behavior is.

Implement the smallest COMPLETE vertical replacement. The turn is incomplete
while old and new production routes coexist or while the change only prepares a
later cutover. Genuine PHP-semantic slow paths must be explicitly named and
reasoned in the contract and must not re-enter the retired engine route.

Before finishing, run:
`python3 scripts/verify/native_replacement_guard.py --require-contract --diff-policy`
Then run every correctness, application, and performance command listed by the
contract. Do not claim completion until the guard passes and the named old route
is absent from the final production source."""


def activate(event: dict[str, Any]) -> dict[str, Any] | None:
    prompt = str(event.get("prompt") or "")
    session_id = str(event.get("session_id") or "")
    root = repository_root(str(event.get("cwd") or ""))
    if not session_id or root is None:
        return None

    path = state_path(root, session_id)
    if not prompt_requests_replacement(prompt):
        path.unlink(missing_ok=True)
        return None

    write_state(
        path,
        {
            "schema_version": 1,
            "active": True,
            "session_id": session_id,
            "turn_id": event.get("turn_id"),
            "repository_root": str(root),
            "prompt_sha256": hashlib.sha256(prompt.encode("utf-8")).hexdigest(),
            "activated_at": datetime.now(timezone.utc).isoformat(),
        },
    )
    return {
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": additional_context(),
        }
    }


def self_test() -> int:
    positives = (
        "[native-replacement] remove the old runtime bridge",
        "Replace the Cranelift fallback route completely",
        "Die alte native Wrapper-Strecke vollständig entfernen",
        "Cut over the execution architecture and eliminate the generic binder",
    )
    negatives = (
        "Fix a parser diagnostic",
        "Document the existing native fallback counter",
        "[native-replacement:off] replace the Cranelift fallback route",
    )
    failures = 0
    for prompt in positives:
        if not prompt_requests_replacement(prompt):
            print(f"[FAIL] replacement prompt not detected: {prompt}")
            failures += 1
    for prompt in negatives:
        if prompt_requests_replacement(prompt):
            print(f"[FAIL] ordinary prompt misclassified: {prompt}")
            failures += 1
    if "smallest COMPLETE vertical replacement" not in additional_context():
        print("[FAIL] replacement context lost the completion rule")
        failures += 1
    if failures:
        return 1
    print("[ok] native replacement prompt hook")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()
    output = activate(read_event())
    if output is not None:
        print(json.dumps(output))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
