#!/usr/bin/env python3
"""Git inspection helpers for native replacement policy checks."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import Iterable, Sequence

from native_replacement_policy import (
    AddedLine,
    GuardError,
    ROOT,
    SENSITIVE_PREFIXES,
    is_sensitive_source,
)


def git(*arguments: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        env={**os.environ, "LC_ALL": "C", "TZ": "UTC"},
        text=True,
        capture_output=True,
        check=check,
    )


def resolve_base(explicit: str | None, head: str | None) -> str:
    if explicit:
        probe = git("rev-parse", "--verify", f"{explicit}^{{commit}}", check=False)
        if probe.returncode != 0:
            raise GuardError(f"cannot resolve base ref {explicit!r}: {probe.stderr.strip()}")
        return explicit
    configured = os.environ.get("PHRUST_NATIVE_REPLACEMENT_BASE")
    if configured:
        return resolve_base(configured, head)
    current = head or "HEAD"
    for candidate in ("origin/main", "main"):
        probe = git("merge-base", current, candidate, check=False)
        if probe.returncode == 0 and probe.stdout.strip():
            return probe.stdout.strip()
    raise GuardError(
        "cannot resolve a comparison base; pass --base or set "
        "PHRUST_NATIVE_REPLACEMENT_BASE"
    )


def diff_arguments(base: str, head: str | None) -> list[str]:
    return [base, head] if head else [base]


def changed_paths(base: str, head: str | None) -> set[str]:
    result = git("diff", "--name-status", "--find-renames", *diff_arguments(base, head))
    paths: set[str] = set()
    for raw in result.stdout.splitlines():
        fields = raw.split("\t")
        if not fields:
            continue
        if fields[0].startswith(("R", "C")) and len(fields) >= 3:
            paths.add(fields[2])
        elif len(fields) >= 2:
            paths.add(fields[1])
    if head is None:
        untracked = git("ls-files", "--others", "--exclude-standard")
        paths.update(line for line in untracked.stdout.splitlines() if line)
    return paths


def added_lines(base: str, head: str | None, paths: Iterable[str]) -> list[AddedLine]:
    records: list[AddedLine] = []
    tracked = sorted(path for path in paths if (ROOT / path).exists())
    if tracked:
        result = git(
            "diff",
            "--no-ext-diff",
            "--unified=0",
            "--no-color",
            *diff_arguments(base, head),
            "--",
            *tracked,
        )
        current_path = ""
        for line in result.stdout.splitlines():
            if line.startswith("+++ b/"):
                current_path = line[6:]
            elif line.startswith("+") and not line.startswith("+++") and current_path:
                records.append(AddedLine(current_path, line[1:]))
    if head is None:
        untracked = set(
            line
            for line in git("ls-files", "--others", "--exclude-standard").stdout.splitlines()
            if line
        )
        for path in sorted(untracked.intersection(paths)):
            candidate = ROOT / path
            if not candidate.is_file():
                continue
            try:
                text = candidate.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            records.extend(AddedLine(path, line) for line in text.splitlines())
    return records


def symbol_exists_at_ref(symbol: str, ref: str) -> bool:
    result = git(
        "grep",
        "-F",
        "-n",
        "-e",
        symbol,
        ref,
        "--",
        *SENSITIVE_PREFIXES,
        check=False,
    )
    return result.returncode == 0


def path_exists_at_ref(relative: str, ref: str) -> bool:
    return git("cat-file", "-e", f"{ref}:{relative}", check=False).returncode == 0


def repository_source_files(scope: Sequence[str]) -> list[Path]:
    tracked = git("ls-files", "--cached", "--others", "--exclude-standard")
    files: list[Path] = []
    for relative in tracked.stdout.splitlines():
        if not relative or not relative.startswith(tuple(scope)) or not is_sensitive_source(relative):
            continue
        path = ROOT / relative
        if path.is_file():
            files.append(path)
    return files
