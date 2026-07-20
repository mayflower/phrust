#!/usr/bin/env python3
"""Shared policy and contract validation for native architecture replacements."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PREFIX = "docs/performance/native-replacement-contracts/"
SENSITIVE_PREFIXES = (
    "crates/php_jit/src/",
    "crates/php_vm/src/vm/",
    "crates/php_runtime/src/",
    "crates/php_executor/src/",
    "crates/php_server/src/",
)
SOURCE_SUFFIXES = {".rs", ".c", ".cc", ".cpp", ".h", ".hpp"}
PLACEHOLDER = re.compile(
    r"\b(?:TODO|TBD|FIXME|FILL[ _-]?ME|REPLACE[ _-]?ME|EXAMPLE[ _-]?ONLY)\b",
    re.IGNORECASE,
)
SUSPICIOUS_ADDITIONS: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "fallback",
        re.compile(
            r"(?<![A-Za-z0-9])(?:fallback|fall_back|generic[_ -]?slow[_ -]?path|"
            r"generic[_ -]?recovery)(?![A-Za-z0-9])",
            re.IGNORECASE,
        ),
    ),
    (
        "compatibility layer",
        re.compile(
            r"(?<![A-Za-z0-9])(?:compat(?:ibility)?|legacy)"
            r"(?:[_ -]?(?:adapter|bridge|dispatch|helper|layer|path|route|wrapper))?"
            r"(?![A-Za-z0-9])",
            re.IGNORECASE,
        ),
    ),
    (
        "wrapper or bridge",
        re.compile(
            r"(?<![A-Za-z0-9])(?:adapter|bridge|dual[_ -]?(?:path|route)|"
            r"shadow[_ -]?(?:path|route)|wrapper)(?![A-Za-z0-9])",
            re.IGNORECASE,
        ),
    ),
    (
        "defensive production path",
        re.compile(
            r"(?<![A-Za-z0-9])(?:safe|safety)[_ -]?(?:path|route|wrapper)"
            r"(?![A-Za-z0-9])",
            re.IGNORECASE,
        ),
    ),
    (
        "interpreter re-entry",
        re.compile(
            r"(?<![A-Za-z0-9])(?:interpreter|resume_to_interpreter|"
            r"execute_ir_function|execute_instruction|execute_bytecode_function)"
            r"(?![A-Za-z0-9])",
            re.IGNORECASE,
        ),
    ),
)
ALLOWLIST_ENGINE_ESCAPE = re.compile(
    r"fallback|compat|legacy|wrapper|adapter|bridge|interpreter|"
    r"(?:safe|safety)[_ -]?(?:path|route|wrapper)",
    re.IGNORECASE,
)
ALLOWLIST_WILDCARDS = {".*", ".+", "^.*$", "^.+$", "(?s).*", "(?s).+"}


@dataclass(frozen=True)
class AddedLine:
    path: str
    line: str


@dataclass(frozen=True)
class DiffAllow:
    pattern: re.Pattern[str]
    paths: tuple[str, ...]
    reason: str

    def matches(self, record: AddedLine) -> bool:
        return any(record.path.startswith(prefix) for prefix in self.paths) and bool(
            self.pattern.search(record.line)
        )


@dataclass(frozen=True)
class Contract:
    path: Path
    document: dict[str, Any]
    allowlist: tuple[DiffAllow, ...]


class GuardError(RuntimeError):
    """Raised for a deterministic policy failure."""


def is_sensitive_source(path: str) -> bool:
    candidate = Path(path)
    if candidate.suffix not in SOURCE_SUFFIXES or not path.startswith(SENSITIVE_PREFIXES):
        return False
    if candidate.name in {"test.rs", "tests.rs"}:
        return False
    return not any(
        marker in path
        for marker in ("/tests/", "/testdata/", "/fixtures/", "/benches/", "/examples/")
    )


def is_comment_line(line: str) -> bool:
    return line.lstrip().startswith(("//", "/*", "*", "#"))


def is_contract_path(path: str) -> bool:
    return (
        path.startswith(CONTRACT_PREFIX)
        and path.endswith(".json")
        and not path.endswith(".example.json")
    )


def require_text(document: dict[str, Any], key: str, failures: list[str]) -> str:
    value = document.get(key)
    if not isinstance(value, str) or len(value.strip()) < 12:
        failures.append(f"{key} must be a descriptive string of at least 12 characters")
        return ""
    if PLACEHOLDER.search(value):
        failures.append(f"{key} contains a placeholder")
    return value.strip()


def string_list(
    value: Any,
    label: str,
    failures: list[str],
    *,
    minimum: int = 0,
) -> list[str]:
    if not isinstance(value, list) or not all(
        isinstance(item, str) and item.strip() for item in value
    ):
        failures.append(f"{label} must be a list of non-empty strings")
        return []
    items = [item.strip() for item in value]
    if len(items) < minimum:
        failures.append(f"{label} must contain at least {minimum} item(s)")
    if any(PLACEHOLDER.search(item) for item in items):
        failures.append(f"{label} contains a placeholder")
    return items


def validate_relative_path(value: str, label: str, failures: list[str]) -> None:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts:
        failures.append(f"{label} must be a repository-relative path: {value!r}")


def parse_allowlist(value: Any, failures: list[str]) -> tuple[DiffAllow, ...]:
    if value is None:
        return ()
    if not isinstance(value, list):
        failures.append("diff_allowlist must be a list")
        return ()
    parsed: list[DiffAllow] = []
    for index, item in enumerate(value):
        label = f"diff_allowlist[{index}]"
        if not isinstance(item, dict):
            failures.append(f"{label} must be an object")
            continue
        pattern = item.get("pattern")
        reason = item.get("reason")
        if not isinstance(pattern, str) or not pattern:
            failures.append(f"{label}.pattern must be a non-empty regex")
            continue
        if not isinstance(reason, str) or len(reason.strip()) < 20:
            failures.append(f"{label}.reason must explain the exception")
            continue
        paths = string_list(item.get("paths"), f"{label}.paths", failures, minimum=1)
        for path in paths:
            validate_relative_path(path, f"{label}.paths", failures)
        if pattern.strip() in ALLOWLIST_WILDCARDS or ALLOWLIST_ENGINE_ESCAPE.search(pattern):
            failures.append(
                f"{label}.pattern may not broadly allow an engine fallback or compatibility term"
            )
            continue
        if any(not path.startswith(SENSITIVE_PREFIXES) for path in paths):
            failures.append(f"{label}.paths must stay inside native execution source roots")
            continue
        try:
            compiled = re.compile(pattern, re.IGNORECASE)
        except re.error as error:
            failures.append(f"{label}.pattern is invalid: {error}")
            continue
        parsed.append(DiffAllow(compiled, tuple(paths), reason.strip()))
    return tuple(parsed)


def validate_contract_document(document: Any) -> tuple[list[str], tuple[DiffAllow, ...]]:
    failures: list[str] = []
    if not isinstance(document, dict):
        return ["contract root must be a JSON object"], ()
    if document.get("schema_version") != 1:
        failures.append("schema_version must be 1")
    if document.get("mode") != "production-architecture-replacement":
        failures.append("mode must be 'production-architecture-replacement'")

    require_text(document, "title", failures)
    require_text(document, "target_architecture", failures)
    scope = string_list(document.get("production_scope"), "production_scope", failures, minimum=1)
    for prefix in scope:
        validate_relative_path(prefix, "production_scope", failures)
        if not prefix.startswith(SENSITIVE_PREFIXES):
            failures.append(
                "production_scope must stay inside a native execution source root: "
                f"{prefix!r}"
            )

    removal = document.get("remove")
    if not isinstance(removal, dict):
        failures.append("remove must be an object")
        removal = {}
    symbols = string_list(removal.get("symbols", []), "remove.symbols", failures)
    paths = string_list(removal.get("paths", []), "remove.paths", failures)
    edges = removal.get("call_edges", [])
    if not isinstance(edges, list):
        failures.append("remove.call_edges must be a list")
        edges = []
    for index, edge in enumerate(edges):
        if not isinstance(edge, dict):
            failures.append(f"remove.call_edges[{index}] must be an object")
            continue
        caller = edge.get("caller")
        callee = edge.get("callee")
        if not isinstance(caller, str) or not caller.strip():
            failures.append(f"remove.call_edges[{index}].caller must be non-empty")
        if not isinstance(callee, str) or not callee.strip():
            failures.append(f"remove.call_edges[{index}].callee must be non-empty")
        elif callee.strip() not in symbols:
            failures.append(
                f"remove.call_edges[{index}].callee must also appear in remove.symbols"
            )
    for path in paths:
        validate_relative_path(path, "remove.paths", failures)
    if not symbols and not paths:
        failures.append("remove must name at least one legacy symbol or path")

    slow_paths = document.get("allowed_php_semantic_slow_paths")
    if not isinstance(slow_paths, list) or not slow_paths:
        failures.append("allowed_php_semantic_slow_paths must be a non-empty list")
    else:
        for index, item in enumerate(slow_paths):
            label = f"allowed_php_semantic_slow_paths[{index}]"
            if not isinstance(item, dict):
                failures.append(f"{label} must be an object")
                continue
            require_text(item, "name", failures)
            require_text(item, "reason", failures)

    validation = string_list(
        document.get("required_validation"), "required_validation", failures, minimum=2
    )
    if validation and not any("native_replacement_guard.py" in command for command in validation):
        failures.append("required_validation must include native_replacement_guard.py")
    if validation and not any(
        token in command
        for command in validation
        for token in ("wordpress-root", "verify-performance", "runtime-semantics", "phpt")
    ):
        failures.append("required_validation must include a correctness or application gate")

    metrics = document.get("expected_metric_movement")
    if not isinstance(metrics, list) or len(metrics) < 2:
        failures.append("expected_metric_movement must contain at least two metrics")
    else:
        for index, item in enumerate(metrics):
            label = f"expected_metric_movement[{index}]"
            if not isinstance(item, dict):
                failures.append(f"{label} must be an object")
                continue
            if not isinstance(item.get("metric"), str) or not item["metric"].strip():
                failures.append(f"{label}.metric must be non-empty")
            if item.get("direction") not in {"decrease", "increase", "zero"}:
                failures.append(f"{label}.direction must be decrease, increase, or zero")
            reason = item.get("reason")
            if not isinstance(reason, str) or len(reason.strip()) < 12:
                failures.append(f"{label}.reason must explain the expected movement")

    acceptance = document.get("acceptance")
    expected = {
        "old_route_production_reachable": False,
        "new_engine_fallback_categories": 0,
        "external_php_behavior_preserved": True,
    }
    if not isinstance(acceptance, dict):
        failures.append("acceptance must be an object")
    else:
        for key, value in expected.items():
            if acceptance.get(key) != value:
                failures.append(f"acceptance.{key} must be {value!r}")

    allowlist = parse_allowlist(document.get("diff_allowlist"), failures)
    return failures, allowlist


def load_contract(path: Path) -> Contract:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise GuardError(f"cannot read contract {path.relative_to(ROOT)}: {error}") from error
    failures, allowlist = validate_contract_document(document)
    if failures:
        details = "\n".join(f"  - {failure}" for failure in failures)
        raise GuardError(f"invalid contract {path.relative_to(ROOT)}:\n{details}")
    return Contract(path=path, document=document, allowlist=allowlist)


def valid_contract() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "mode": "production-architecture-replacement",
        "title": "Remove the legacy runtime call bridge",
        "target_architecture": (
            "Generated Cranelift code calls a typed production entry directly and "
            "never re-enters the retired runtime bridge."
        ),
        "production_scope": ["crates/php_jit/src/", "crates/php_vm/src/vm/"],
        "remove": {
            "symbols": ["legacy_runtime_bridge"],
            "paths": [],
            "call_edges": [
                {"caller": "compiled_callsite", "callee": "legacy_runtime_bridge"}
            ],
        },
        "allowed_php_semantic_slow_paths": [
            {
                "name": "runtime-created dynamic callable resolution",
                "reason": (
                    "PHP permits runtime-created callables whose target is not known "
                    "at publication."
                ),
            }
        ],
        "diff_allowlist": [
            {
                "pattern": r"dynamic_callable_slow",
                "paths": ["crates/php_vm/src/vm/"],
                "reason": (
                    "This is the explicit runtime-unknown callable resolver, not an "
                    "engine fallback."
                ),
            }
        ],
        "required_validation": [
            "python3 scripts/verify/native_replacement_guard.py --require-contract --diff-policy",
            "nix develop -c just wordpress-root-tranche-gate target/performance/baseline.json",
        ],
        "expected_metric_movement": [
            {
                "metric": "warm_c1_p50_ms",
                "direction": "decrease",
                "reason": "The shared call bridge leaves the warm path.",
            },
            {
                "metric": "runtime_helper_calls",
                "direction": "decrease",
                "reason": "Stable calls no longer cross the generic helper boundary.",
            },
        ],
        "acceptance": {
            "old_route_production_reachable": False,
            "new_engine_fallback_categories": 0,
            "external_php_behavior_preserved": True,
        },
    }
