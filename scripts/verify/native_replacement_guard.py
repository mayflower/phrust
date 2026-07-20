#!/usr/bin/env python3
"""Reject wrapper-based Cranelift changes and verify replacement contracts.

``--diff-policy`` rejects newly added compatibility, fallback, adapter, and
interpreter-reentry machinery in production execution sources unless a changed
contract contains a narrow semantic allowlist. ``--require-contract`` also
proves that named legacy targets existed at the base and are absent from the
final production tree.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Sequence

from native_replacement_git import (
    added_lines,
    changed_paths,
    path_exists_at_ref,
    repository_source_files,
    resolve_base,
    symbol_exists_at_ref,
)
from native_replacement_policy import (
    CONTRACT_PREFIX,
    ROOT,
    SENSITIVE_PREFIXES,
    SUSPICIOUS_ADDITIONS,
    AddedLine,
    Contract,
    DiffAllow,
    GuardError,
    is_comment_line,
    is_contract_path,
    is_sensitive_source,
    load_contract,
    valid_contract,
    validate_contract_document,
)


def enforce_contract(contract: Contract, changed: set[str], base: str) -> list[str]:
    failures: list[str] = []
    scope = tuple(contract.document["production_scope"])
    removal = contract.document["remove"]
    if not any(path.startswith(scope) and is_sensitive_source(path) for path in changed):
        failures.append(
            f"{contract.path.relative_to(ROOT)} changes no production source in its declared scope"
        )

    for relative in removal.get("paths", []):
        if not path_exists_at_ref(relative, base):
            failures.append(f"legacy path did not exist at base {base}: {relative}")
        if (ROOT / relative).exists():
            failures.append(f"legacy path still exists: {relative}")

    symbols = removal.get("symbols", [])
    hits: dict[str, list[str]] = {symbol: [] for symbol in symbols}
    for symbol in symbols:
        if not symbol_exists_at_ref(symbol, base):
            failures.append(
                f"legacy symbol {symbol!r} did not exist in native production source at base {base}"
            )
    for path in repository_source_files(SENSITIVE_PREFIXES):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        relative = path.relative_to(ROOT).as_posix()
        for symbol in symbols:
            for line_number, line in enumerate(lines, start=1):
                if symbol in line and not is_comment_line(line):
                    hits[symbol].append(f"{relative}:{line_number}: {line.strip()}")
                    if len(hits[symbol]) >= 5:
                        break
    for symbol, locations in hits.items():
        if locations:
            failures.append(
                f"legacy symbol {symbol!r} remains in production source:\n"
                + "\n".join(f"    {location}" for location in locations)
            )
    return failures


def all_allowlist(contracts: Sequence[Contract]) -> tuple[DiffAllow, ...]:
    return tuple(item for contract in contracts for item in contract.allowlist)


def scan_diff_policy(
    records: Sequence[AddedLine], allowlist: Sequence[DiffAllow]
) -> list[str]:
    failures: list[str] = []
    for record in records:
        if (
            not is_sensitive_source(record.path)
            or not record.line.strip()
            or is_comment_line(record.line)
        ):
            continue
        for label, pattern in SUSPICIOUS_ADDITIONS:
            if not pattern.search(record.line):
                continue
            if any(rule.matches(record) for rule in allowlist):
                break
            failures.append(
                f"{record.path}: added {label} without a contract allowlist: "
                f"{record.line.strip()}"
            )
            break
    return failures


def contract_paths_from_diff(changed: set[str]) -> list[Path]:
    return sorted(
        ROOT / path
        for path in changed
        if is_contract_path(path) and (ROOT / path).is_file()
    )


def run_guard(args: argparse.Namespace) -> int:
    base = resolve_base(args.base, args.head)
    changed = changed_paths(base, args.head)
    paths = [path if path.is_absolute() else ROOT / path for path in args.contract]
    if not paths:
        paths = contract_paths_from_diff(changed)

    contracts: list[Contract] = []
    failures: list[str] = []
    for path in paths:
        try:
            contracts.append(load_contract(path))
        except GuardError as error:
            failures.append(str(error))

    if args.require_contract and not contracts:
        failures.append(
            "architecture-replacement mode requires a changed contract under "
            f"{CONTRACT_PREFIX}"
        )
    for contract in contracts:
        failures.extend(enforce_contract(contract, changed, base))

    if args.require_contract and contracts:
        uncovered = sorted(
            path
            for path in changed
            if is_sensitive_source(path)
            and not any(
                path.startswith(tuple(contract.document["production_scope"]))
                for contract in contracts
            )
        )
        if uncovered:
            failures.append(
                "changed native production sources are outside every replacement "
                "contract scope: " + ", ".join(uncovered)
            )

    if args.diff_policy or (not args.require_contract and not args.contract):
        failures.extend(
            scan_diff_policy(added_lines(base, args.head, changed), all_allowlist(contracts))
        )

    if failures:
        print("[fail] native replacement guard:", file=sys.stderr)
        for failure in failures:
            for line in failure.splitlines():
                print(f"  - {line}", file=sys.stderr)
        print(
            "  fix: remove the legacy production route rather than wrapping it; "
            "document only a narrow, genuine PHP-semantic slow path.",
            file=sys.stderr,
        )
        return 1
    print(
        "[ok] native replacement guard: "
        f"base={base} changed={len(changed)} contracts={len(contracts)}"
    )
    return 0


def self_test() -> int:
    failures = 0
    valid_failures, allowlist = validate_contract_document(valid_contract())
    if valid_failures or len(allowlist) != 1:
        print(f"[FAIL] valid contract rejected: {valid_failures}")
        failures += 1
    else:
        print("[ok] valid replacement contract")

    invalid = valid_contract()
    invalid["remove"] = {"symbols": [], "paths": [], "call_edges": []}
    invalid["acceptance"] = {"old_route_production_reachable": True}
    invalid_failures, _ = validate_contract_document(invalid)
    if len(invalid_failures) < 3:
        print(f"[FAIL] invalid contract was under-rejected: {invalid_failures}")
        failures += 1
    else:
        print("[ok] invalid replacement contract rejected")

    records = [
        AddedLine("crates/php_vm/src/vm/calls.rs", "fn compatibility_wrapper() {}"),
        AddedLine("crates/php_vm/src/vm/calls.rs", "fn direct_call() {}"),
    ]
    policy_failures = scan_diff_policy(records, ())
    if not any("compatibility" in failure for failure in policy_failures):
        print(f"[FAIL] wrapper addition escaped policy: {policy_failures}")
        failures += 1
    else:
        print("[ok] wrapper addition rejected")

    allowed = DiffAllow(
        re.compile(r"dynamic_callable_slow", re.IGNORECASE),
        ("crates/php_vm/src/vm/",),
        "explicit PHP dynamic callable resolver",
    )
    semantic = [
        AddedLine(
            "crates/php_vm/src/vm/calls.rs",
            "fn dynamic_callable_slow_fallback() {}",
        )
    ]
    if scan_diff_policy(semantic, (allowed,)):
        print("[FAIL] allowlisted semantic slow path rejected")
        failures += 1
    else:
        print("[ok] narrow semantic slow-path allowlist")

    broad = valid_contract()
    broad["diff_allowlist"][0]["pattern"] = "fallback"
    broad_failures, _ = validate_contract_document(broad)
    if not any("broadly allow" in failure for failure in broad_failures):
        print(f"[FAIL] broad fallback allowlist escaped validation: {broad_failures}")
        failures += 1
    else:
        print("[ok] broad fallback allowlist rejected")

    comment = AddedLine(
        "crates/php_vm/src/vm/calls.rs",
        "// The optimized route has no generic fallback.",
    )
    if scan_diff_policy([comment], ()):
        print("[FAIL] explanatory source comment was treated as production code")
        failures += 1
    else:
        print("[ok] source comments do not trigger the diff policy")

    if is_sensitive_source("crates/php_jit/src/cranelift_lowering/tests.rs"):
        print("[FAIL] Rust test module was treated as production source")
        failures += 1
    else:
        print("[ok] Rust test modules are excluded")

    template = ROOT / CONTRACT_PREFIX / "template.example.json"
    if template.is_file():
        try:
            document = json.loads(template.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            print(f"[FAIL] contract template is unreadable: {error}")
            failures += 1
        else:
            template_failures, _ = validate_contract_document(document)
            if template_failures:
                print(f"[FAIL] contract template is invalid: {template_failures}")
                failures += 1
            else:
                print("[ok] checked-in replacement contract template")
    return 1 if failures else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--contract", action="append", type=Path, default=[])
    parser.add_argument("--require-contract", action="store_true")
    parser.add_argument("--diff-policy", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()
    try:
        return run_guard(args)
    except (GuardError, OSError, subprocess.CalledProcessError) as error:
        print(f"[fail] native replacement guard setup: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
