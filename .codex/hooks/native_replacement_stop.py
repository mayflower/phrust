#!/usr/bin/env python3
"""Refuse to end an active architecture-replacement turn before its gate passes."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import native_replacement_prompt as prompt_policy


MAX_FEEDBACK_CHARS = 7000


def read_event() -> dict[str, Any]:
    try:
        document = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"invalid Codex hook input: {error}") from error
    if not isinstance(document, dict):
        raise SystemExit("invalid Codex hook input: expected an object")
    return document


def load_state(path: Path) -> dict[str, Any] | None:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None
    except (OSError, json.JSONDecodeError):
        return {"active": True, "invalid": True}
    return document if isinstance(document, dict) else {"active": True, "invalid": True}


def guard_command(root: Path) -> list[str]:
    return [
        sys.executable,
        str(root / "scripts/verify/native_replacement_guard.py"),
        "--require-contract",
        "--diff-policy",
    ]


def feedback(output: str) -> str:
    trimmed = output.strip()
    if len(trimmed) > MAX_FEEDBACK_CHARS:
        trimmed = trimmed[-MAX_FEEDBACK_CHARS:]
    return (
        "The Phrust native replacement contract still fails. Continue the same "
        "turn. Do not bypass the gate, weaken the contract, add an allowlist for "
        "an engine fallback, or wrap/rename the legacy route. Remove the reported "
        "production route and rerun the exact guard.\n\n"
        + (trimmed or "The replacement guard exited without diagnostics.")
    )


def stop_output(already_continued: bool, reason: str) -> dict[str, Any]:
    if already_continued:
        return {
            "continue": False,
            "stopReason": reason,
            "systemMessage": "Native replacement validation remains red.",
        }
    return {"decision": "block", "reason": reason}


def enforce(event: dict[str, Any]) -> dict[str, Any] | None:
    root = prompt_policy.repository_root(str(event.get("cwd") or ""))
    session_id = str(event.get("session_id") or "")
    if root is None or not session_id:
        return None
    state_path = prompt_policy.state_path(root, session_id)
    state = load_state(state_path)
    if not state or not state.get("active"):
        return None

    guard = subprocess.run(
        guard_command(root),
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
        timeout=150,
    )
    if guard.returncode == 0:
        state_path.unlink(missing_ok=True)
        return None

    reason = feedback(guard.stdout + "\n" + guard.stderr)
    return stop_output(bool(event.get("stop_hook_active")), reason)


def self_test() -> int:
    first = stop_output(False, "failure")
    second = stop_output(True, "failure")
    failures = 0
    if first != {"decision": "block", "reason": "failure"}:
        print(f"[FAIL] first Stop response is invalid: {first}")
        failures += 1
    if second.get("continue") is not False or "stopReason" not in second:
        print(f"[FAIL] repeated Stop response is invalid: {second}")
        failures += 1
    if "--require-contract" not in guard_command(Path("/repo")):
        print("[FAIL] Stop hook does not require a replacement contract")
        failures += 1
    if failures:
        return 1
    print("[ok] native replacement Stop hook")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()
    try:
        output = enforce(read_event())
    except (OSError, subprocess.SubprocessError) as error:
        output = stop_output(False, feedback(f"replacement hook setup failed: {error}"))
    if output is not None:
        print(json.dumps(output))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
