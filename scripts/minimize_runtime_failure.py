#!/usr/bin/env python3
"""Minimize a runtime-semantics differential failure to a smaller PHP fixture."""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

import runtime_semantics_diff


def main() -> int:
    args = parse_args()
    if not os.environ.get("REFERENCE_PHP"):
        print("[error] REFERENCE_PHP must be set for failure minimization", file=sys.stderr)
        return 2

    source_path = Path(args.input)
    if not source_path.is_file():
        print(f"[error] input is not a file: {source_path}", file=sys.stderr)
        return 2

    original = source_path.read_text(encoding="utf-8", errors="replace")
    rust_vm = Path(args.rust_vm)
    base_diff = diff_signature(source_path, rust_vm, original)
    if not base_diff:
        print("[error] input does not currently differ between REFERENCE_PHP and Rust VM", file=sys.stderr)
        return 1

    minimized = minimize_lines(source_path, rust_vm, original, base_diff, args.keep_same_signature)
    output_path = Path(args.out)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(minimized, encoding="utf-8")
    kept = len(minimized.splitlines())
    total = len(original.splitlines())
    print(f"[ok] minimized {source_path} from {total} line(s) to {kept} line(s): {output_path}")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Greedily minimize a PHP fixture while preserving a differential "
            "failure between REFERENCE_PHP and php_vm_cli."
        )
    )
    parser.add_argument("input", help="PHP fixture that currently differs")
    parser.add_argument("--out", default="target/runtime-semantics/minimized.php", help="minimized output file")
    parser.add_argument("--rust-vm", default=os.environ.get("PHP_VM_CLI", "target/debug/php-vm"))
    parser.add_argument(
        "--keep-same-signature",
        action="store_true",
        help="preserve the original normalized difference signature exactly",
    )
    return parser.parse_args()


def minimize_lines(source_path: Path, rust_vm: Path, source: str, base_diff: tuple[str, ...], keep_same: bool) -> str:
    lines = source.splitlines(keepends=True)
    if not lines:
        return source

    changed = True
    while changed:
        changed = False
        chunk = max(1, len(lines) // 2)
        while chunk >= 1:
            index = 0
            removed_at_this_size = False
            while index < len(lines):
                candidate_lines = lines[:index] + lines[index + chunk :]
                candidate = "".join(candidate_lines)
                if candidate.strip() and preserves_diff(source_path, rust_vm, candidate, base_diff, keep_same):
                    lines = candidate_lines
                    changed = True
                    removed_at_this_size = True
                    continue
                index += chunk
            if removed_at_this_size:
                chunk = max(1, min(chunk, len(lines) // 2))
            else:
                chunk //= 2
    return "".join(lines)


def preserves_diff(
    source_path: Path,
    rust_vm: Path,
    candidate: str,
    base_diff: tuple[str, ...],
    keep_same: bool,
) -> bool:
    signature = diff_signature(source_path, rust_vm, candidate)
    if not signature:
        return False
    return signature == base_diff if keep_same else True


def diff_signature(source_path: Path, rust_vm: Path, source: str) -> tuple[str, ...]:
    temp_path = Path("target/runtime-semantics/minimize-candidate.php")
    temp_path.parent.mkdir(parents=True, exist_ok=True)
    temp_path.write_text(source, encoding="utf-8")
    fixture = runtime_semantics_diff.Fixture(path=temp_path, category="ad_hoc", php_ref_required=True)
    reference = runtime_semantics_diff.run_reference(fixture)
    rust = runtime_semantics_diff.run_rust(fixture, rust_vm)
    if reference["status"] != "completed" or rust["status"] != "completed":
        return ()
    return tuple(runtime_semantics_diff.normalized_differences(reference, rust))


if __name__ == "__main__":
    raise SystemExit(main())
