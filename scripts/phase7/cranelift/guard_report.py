#!/usr/bin/env python3
"""Generate a deterministic Cranelift side-exit and guard report."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_OUT = ROOT / "target/phase7/cranelift/guard-report.json"
DEFAULT_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/side-exit/helper-status-overflow.php"
DEFAULT_BLACKLIST_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/side-exit/unstable-type-switch.php"
DEFAULT_PACKED_VALID_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/arrays/packed-fetch-valid.php"
DEFAULT_PACKED_BOUNDS_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/arrays/packed-fetch-out-of-bounds.php"
DEFAULT_PACKED_LAYOUT_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/arrays/packed-fetch-mixed-array.php"
DEFAULT_PACKED_STRING_KEY_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/arrays/packed-fetch-string-key.php"
DEFAULT_PROPERTY_VALID_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/property-load/simple-dto-property-read.php"
DEFAULT_PROPERTY_WRONG_CLASS_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/property-load/wrong-class-side-exit.php"
DEFAULT_PROPERTY_HOOK_MAGIC_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/property-load/hook-magic-fallback.php"
DEFAULT_PROPERTY_UNINITIALIZED_FIXTURE = ROOT / "tests/fixtures/phase7/cranelift/property-load/uninitialized-error-path.php"


@dataclass(frozen=True)
class Run:
    command: list[str]
    returncode: int
    stdout: str
    stderr: str
    jit_stats: dict[str, Any] | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_ENGINE)))
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    parser.add_argument("--blacklist-fixture", type=Path, default=DEFAULT_BLACKLIST_FIXTURE)
    parser.add_argument("--packed-valid-fixture", type=Path, default=DEFAULT_PACKED_VALID_FIXTURE)
    parser.add_argument("--packed-bounds-fixture", type=Path, default=DEFAULT_PACKED_BOUNDS_FIXTURE)
    parser.add_argument("--packed-layout-fixture", type=Path, default=DEFAULT_PACKED_LAYOUT_FIXTURE)
    parser.add_argument("--packed-string-key-fixture", type=Path, default=DEFAULT_PACKED_STRING_KEY_FIXTURE)
    parser.add_argument("--property-valid-fixture", type=Path, default=DEFAULT_PROPERTY_VALID_FIXTURE)
    parser.add_argument("--property-wrong-class-fixture", type=Path, default=DEFAULT_PROPERTY_WRONG_CLASS_FIXTURE)
    parser.add_argument("--property-hook-magic-fixture", type=Path, default=DEFAULT_PROPERTY_HOOK_MAGIC_FIXTURE)
    parser.add_argument("--property-uninitialized-fixture", type=Path, default=DEFAULT_PROPERTY_UNINITIALIZED_FIXTURE)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--timeout", type=float, default=float(os.getenv("PHRUST_CRANELIFT_GUARD_TIMEOUT", "10.0")))
    return parser.parse_args()


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def normalized_env(tmp_dir: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.update(
        {
            "TZ": "UTC",
            "LC_ALL": "C",
            "LANG": "C",
            "TMPDIR": str(tmp_dir),
            "TMP": str(tmp_dir),
            "TEMP": str(tmp_dir),
            "PHRUST_RANDOM_SEED": "phase7-cranelift-guard-report",
            "RUST_TEST_SEED": "phase7-cranelift-guard-report",
        }
    )
    return env


def normalize_text(value: str) -> str:
    return value.replace("\r\n", "\n").replace("\r", "\n")


def extract_jit_stats(stderr: str) -> dict[str, Any] | None:
    for line in stderr.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{"):
            continue
        try:
            decoded = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        jit = decoded.get("jit") if isinstance(decoded, dict) else None
        if isinstance(jit, dict):
            return jit
    return None


def stderr_without_jit_stats(stderr: str) -> str:
    lines = []
    for line in stderr.splitlines():
        stripped = line.strip()
        if stripped.startswith("{"):
            try:
                decoded = json.loads(stripped)
            except json.JSONDecodeError:
                pass
            else:
                if isinstance(decoded, dict) and isinstance(decoded.get("jit"), dict):
                    continue
        lines.append(line)
    return "\n".join(lines) + ("\n" if lines else "")


def run_engine(engine: Path, fixture: Path, mode: str, tmp_dir: Path, timeout: float) -> Run:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    command = [
        str(engine),
        "run",
        f"--jit={mode}",
        "--jit-eager",
        "--jit-stats=json",
        rel(fixture),
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=normalized_env(tmp_dir),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    stderr = normalize_text(completed.stderr)
    return Run(
        command=[rel(Path(command[0])), *command[1:]],
        returncode=completed.returncode,
        stdout=normalize_text(completed.stdout),
        stderr=stderr,
        jit_stats=extract_jit_stats(stderr),
    )


def require(condition: bool, failures: list[str], message: str) -> None:
    if not condition:
        failures.append(message)


def require_same_visible_behavior(name: str, off: Run, cranelift: Run, failures: list[str]) -> None:
    require(off.returncode == cranelift.returncode, failures, f"{name}: jit off/on exit codes differ")
    require(off.stdout == cranelift.stdout, failures, f"{name}: jit off/on stdout differs")
    require(
        stderr_without_jit_stats(off.stderr) == stderr_without_jit_stats(cranelift.stderr),
        failures,
        f"{name}: jit off/on stderr differs after removing jit stats",
    )


def check_packed_fetch(
    *,
    name: str,
    off: Run,
    cranelift: Run,
    failures: list[str],
    expected_fast_hits: int,
    expected_bounds_exits: int,
    expected_layout_exits: int,
    expected_compiled: bool,
) -> None:
    require_same_visible_behavior(name, off, cranelift, failures)
    stats = cranelift.jit_stats
    require(stats is not None, failures, f"{name}: missing Cranelift jit stats JSON")
    if stats is None:
        return
    require(stats.get("mode") == "cranelift", failures, f"{name}: stats mode is not cranelift")
    if expected_compiled:
        require(stats.get("compiled", 0) > 0, failures, f"{name}: expected a compiled native region")
    else:
        require(stats.get("compiled", 0) == 0, failures, f"{name}: should not compile a native packed-fetch region")
    require(
        stats.get("packed_fetch_fast_hits", 0) == expected_fast_hits,
        failures,
        f"{name}: expected packed_fetch_fast_hits={expected_fast_hits}, got {stats.get('packed_fetch_fast_hits')}",
    )
    require(
        stats.get("packed_fetch_bounds_exits", 0) == expected_bounds_exits,
        failures,
        f"{name}: expected packed_fetch_bounds_exits={expected_bounds_exits}, got {stats.get('packed_fetch_bounds_exits')}",
    )
    require(
        stats.get("packed_fetch_layout_exits", 0) == expected_layout_exits,
        failures,
        f"{name}: expected packed_fetch_layout_exits={expected_layout_exits}, got {stats.get('packed_fetch_layout_exits')}",
    )
    if expected_bounds_exits > 0 or expected_layout_exits > 0:
        raw_side_exit_reasons = stats.get("side_exit_reasons")
        helper_status_count = (
            raw_side_exit_reasons.get("helper_status", 0)
            if isinstance(raw_side_exit_reasons, dict)
            else 0
        )
        overflow_count = (
            raw_side_exit_reasons.get("overflow", 0)
            if isinstance(raw_side_exit_reasons, dict)
            else 0
        )
        require(helper_status_count > 0, failures, f"{name}: expected helper_status side-exit reason")
        require(overflow_count == 0, failures, f"{name}: packed fetch should not record overflow side-exit reason")


def check_property_load(
    *,
    name: str,
    off: Run,
    cranelift: Run,
    failures: list[str],
    expected_fast_hits: int,
    expected_guard_exits: int,
    expected_slow_calls: int,
    expected_uninitialized_exits: int,
    expected_compiled: bool,
) -> None:
    require_same_visible_behavior(name, off, cranelift, failures)
    stats = cranelift.jit_stats
    require(stats is not None, failures, f"{name}: missing Cranelift jit stats JSON")
    if stats is None:
        return
    require(stats.get("mode") == "cranelift", failures, f"{name}: stats mode is not cranelift")
    if expected_compiled:
        require(stats.get("compiled", 0) > 0, failures, f"{name}: expected a compiled native property-load region")
    else:
        require(stats.get("compiled", 0) == 0, failures, f"{name}: should not compile a native property-load region")
    require(
        stats.get("property_load_fast_hits", 0) == expected_fast_hits,
        failures,
        f"{name}: expected property_load_fast_hits={expected_fast_hits}, got {stats.get('property_load_fast_hits')}",
    )
    require(
        stats.get("property_load_guard_exits", 0) == expected_guard_exits,
        failures,
        f"{name}: expected property_load_guard_exits={expected_guard_exits}, got {stats.get('property_load_guard_exits')}",
    )
    require(
        stats.get("property_load_slow_calls", 0) == expected_slow_calls,
        failures,
        f"{name}: expected property_load_slow_calls={expected_slow_calls}, got {stats.get('property_load_slow_calls')}",
    )
    require(
        stats.get("property_load_uninitialized_exits", 0) == expected_uninitialized_exits,
        failures,
        f"{name}: expected property_load_uninitialized_exits={expected_uninitialized_exits}, got {stats.get('property_load_uninitialized_exits')}",
    )
    if expected_guard_exits > 0:
        raw_side_exit_reasons = stats.get("side_exit_reasons")
        guard_failed_count = (
            raw_side_exit_reasons.get("guard_failed", 0)
            if isinstance(raw_side_exit_reasons, dict)
            else 0
        )
        require(guard_failed_count > 0, failures, f"{name}: expected guard_failed side-exit reason")


def main() -> int:
    args = parse_args()
    if not args.engine.is_file() or not os.access(args.engine, os.X_OK):
        raise SystemExit(f"Rust VM is not executable: {args.engine}")
    if not args.fixture.is_file():
        raise SystemExit(f"missing side-exit fixture: {args.fixture}")
    if not args.blacklist_fixture.is_file():
        raise SystemExit(f"missing blacklist fixture: {args.blacklist_fixture}")
    for name, fixture in (
        ("packed valid fixture", args.packed_valid_fixture),
        ("packed bounds fixture", args.packed_bounds_fixture),
        ("packed layout fixture", args.packed_layout_fixture),
        ("packed string-key fixture", args.packed_string_key_fixture),
        ("property valid fixture", args.property_valid_fixture),
        ("property wrong-class fixture", args.property_wrong_class_fixture),
        ("property hook/magic fixture", args.property_hook_magic_fixture),
        ("property uninitialized fixture", args.property_uninitialized_fixture),
    ):
        if not fixture.is_file():
            raise SystemExit(f"missing {name}: {fixture}")

    off = run_engine(
        args.engine,
        args.fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/off",
        args.timeout,
    )
    cranelift = run_engine(
        args.engine,
        args.fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/cranelift",
        args.timeout,
    )
    blacklist = run_engine(
        args.engine,
        args.blacklist_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/blacklist",
        args.timeout,
    )
    blacklist_off = run_engine(
        args.engine,
        args.blacklist_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/blacklist-off",
        args.timeout,
    )
    packed_valid_off = run_engine(
        args.engine,
        args.packed_valid_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-valid-off",
        args.timeout,
    )
    packed_valid = run_engine(
        args.engine,
        args.packed_valid_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-valid",
        args.timeout,
    )
    packed_bounds_off = run_engine(
        args.engine,
        args.packed_bounds_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-bounds-off",
        args.timeout,
    )
    packed_bounds = run_engine(
        args.engine,
        args.packed_bounds_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-bounds",
        args.timeout,
    )
    packed_layout_off = run_engine(
        args.engine,
        args.packed_layout_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-layout-off",
        args.timeout,
    )
    packed_layout = run_engine(
        args.engine,
        args.packed_layout_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-layout",
        args.timeout,
    )
    packed_string_key_off = run_engine(
        args.engine,
        args.packed_string_key_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-string-key-off",
        args.timeout,
    )
    packed_string_key = run_engine(
        args.engine,
        args.packed_string_key_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/packed-string-key",
        args.timeout,
    )
    property_valid_off = run_engine(
        args.engine,
        args.property_valid_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-valid-off",
        args.timeout,
    )
    property_valid = run_engine(
        args.engine,
        args.property_valid_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-valid",
        args.timeout,
    )
    property_wrong_class_off = run_engine(
        args.engine,
        args.property_wrong_class_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-wrong-class-off",
        args.timeout,
    )
    property_wrong_class = run_engine(
        args.engine,
        args.property_wrong_class_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-wrong-class",
        args.timeout,
    )
    property_hook_magic_off = run_engine(
        args.engine,
        args.property_hook_magic_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-hook-magic-off",
        args.timeout,
    )
    property_hook_magic = run_engine(
        args.engine,
        args.property_hook_magic_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-hook-magic",
        args.timeout,
    )
    property_uninitialized_off = run_engine(
        args.engine,
        args.property_uninitialized_fixture,
        "off",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-uninitialized-off",
        args.timeout,
    )
    property_uninitialized = run_engine(
        args.engine,
        args.property_uninitialized_fixture,
        "cranelift",
        ROOT / "target/phase7/cranelift/guard-report-tmp/property-uninitialized",
        args.timeout,
    )

    failures: list[str] = []
    require_same_visible_behavior("side-exit fixture", off, cranelift, failures)

    stats = cranelift.jit_stats
    require(stats is not None, failures, "missing Cranelift jit stats JSON")
    side_exit_reasons: dict[str, Any] = {}
    blacklist_reasons: dict[str, Any] = {}
    if stats is not None:
        raw_reasons = stats.get("side_exit_reasons")
        if isinstance(raw_reasons, dict):
            side_exit_reasons = raw_reasons
        require(stats.get("mode") == "cranelift", failures, "stats mode is not cranelift")
        require(stats.get("compiled", 0) > 0, failures, "expected a compiled native region")
        require(stats.get("executed", 0) == 0, failures, "side-exit fixture must not count as executed")
        require(stats.get("bailouts", 0) > 0, failures, "side-exit fixture must record a bailout")
        require(stats.get("side_exits", 0) > 0, failures, "side-exit fixture must record side exits")
        require(
            side_exit_reasons.get("overflow", 0) > 0,
            failures,
            "side-exit reason overflow is missing",
        )
        require(stats.get("overflow_exits", 0) > 0, failures, "side-exit fixture must record overflow exits")
        require(stats.get("slow_path_calls", 0) > 0, failures, "side-exit fixture must record slow-path calls")

    require_same_visible_behavior("blacklist fixture", blacklist_off, blacklist, failures)
    blacklist_stats = blacklist.jit_stats
    require(blacklist_stats is not None, failures, "missing blacklist jit stats JSON")
    if blacklist_stats is not None:
        raw_reasons = blacklist_stats.get("blacklist_reasons")
        if isinstance(raw_reasons, dict):
            blacklist_reasons = raw_reasons
        raw_side_exit_reasons = blacklist_stats.get("side_exit_reasons")
        type_mismatch_count = (
            raw_side_exit_reasons.get("type_mismatch", 0)
            if isinstance(raw_side_exit_reasons, dict)
            else 0
        )
        require(blacklist_stats.get("mode") == "cranelift", failures, "blacklist stats mode is not cranelift")
        require(blacklist_stats.get("blacklist") == "on", failures, "blacklist mode should default to on")
        require(blacklist_stats.get("executed", 0) == 1, failures, "blacklist fixture should execute native once")
        require(type_mismatch_count >= 2, failures, "blacklist fixture should record type_mismatch side exits")
        require(
            blacklist_stats.get("blacklisted_regions", 0) > 0,
            failures,
            "blacklist fixture should record a blacklisted region",
        )
        require(
            blacklist_reasons.get("too_many_side_exits", 0) > 0,
            failures,
            "blacklist reason too_many_side_exits is missing",
        )
    check_packed_fetch(
        name="packed valid fixture",
        off=packed_valid_off,
        cranelift=packed_valid,
        failures=failures,
        expected_fast_hits=1,
        expected_bounds_exits=0,
        expected_layout_exits=0,
        expected_compiled=True,
    )
    check_packed_fetch(
        name="packed bounds fixture",
        off=packed_bounds_off,
        cranelift=packed_bounds,
        failures=failures,
        expected_fast_hits=0,
        expected_bounds_exits=1,
        expected_layout_exits=0,
        expected_compiled=True,
    )
    check_packed_fetch(
        name="packed layout fixture",
        off=packed_layout_off,
        cranelift=packed_layout,
        failures=failures,
        expected_fast_hits=0,
        expected_bounds_exits=0,
        expected_layout_exits=1,
        expected_compiled=True,
    )
    check_packed_fetch(
        name="packed string-key fixture",
        off=packed_string_key_off,
        cranelift=packed_string_key,
        failures=failures,
        expected_fast_hits=0,
        expected_bounds_exits=0,
        expected_layout_exits=0,
        expected_compiled=False,
    )
    check_property_load(
        name="property valid fixture",
        off=property_valid_off,
        cranelift=property_valid,
        failures=failures,
        expected_fast_hits=1,
        expected_guard_exits=0,
        expected_slow_calls=0,
        expected_uninitialized_exits=0,
        expected_compiled=True,
    )
    check_property_load(
        name="property wrong-class fixture",
        off=property_wrong_class_off,
        cranelift=property_wrong_class,
        failures=failures,
        expected_fast_hits=1,
        expected_guard_exits=1,
        expected_slow_calls=1,
        expected_uninitialized_exits=0,
        expected_compiled=True,
    )
    check_property_load(
        name="property hook/magic fixture",
        off=property_hook_magic_off,
        cranelift=property_hook_magic,
        failures=failures,
        expected_fast_hits=0,
        expected_guard_exits=0,
        expected_slow_calls=0,
        expected_uninitialized_exits=0,
        expected_compiled=False,
    )
    check_property_load(
        name="property uninitialized fixture",
        off=property_uninitialized_off,
        cranelift=property_uninitialized,
        failures=failures,
        expected_fast_hits=0,
        expected_guard_exits=1,
        expected_slow_calls=1,
        expected_uninitialized_exits=1,
        expected_compiled=True,
    )

    report = {
        "gate": "cranelift-guard-report",
        "status": "fail" if failures else "pass",
        "fixture": rel(args.fixture),
        "blacklist_fixture": rel(args.blacklist_fixture),
        "jit_off": {
            "command": off.command,
            "exit_code": off.returncode,
        },
        "jit_cranelift": {
            "command": cranelift.command,
            "exit_code": cranelift.returncode,
            "stats": stats,
        },
        "side_exit_reasons": side_exit_reasons,
        "blacklist_jit_cranelift": {
            "command": blacklist.command,
            "exit_code": blacklist.returncode,
            "stats": blacklist_stats,
        },
        "blacklist_reasons": blacklist_reasons,
        "packed_fetch": {
            "valid": {
                "fixture": rel(args.packed_valid_fixture),
                "jit_cranelift": {
                    "command": packed_valid.command,
                    "exit_code": packed_valid.returncode,
                    "stats": packed_valid.jit_stats,
                },
            },
            "bounds": {
                "fixture": rel(args.packed_bounds_fixture),
                "jit_cranelift": {
                    "command": packed_bounds.command,
                    "exit_code": packed_bounds.returncode,
                    "stats": packed_bounds.jit_stats,
                },
            },
            "layout": {
                "fixture": rel(args.packed_layout_fixture),
                "jit_cranelift": {
                    "command": packed_layout.command,
                    "exit_code": packed_layout.returncode,
                    "stats": packed_layout.jit_stats,
                },
            },
            "string_key": {
                "fixture": rel(args.packed_string_key_fixture),
                "jit_cranelift": {
                    "command": packed_string_key.command,
                    "exit_code": packed_string_key.returncode,
                    "stats": packed_string_key.jit_stats,
                },
            },
        },
        "property_load": {
            "fallback_policy": {
                "class_guard": "wrong receiver class side-exits with guard_failed and re-enters the generic property fetch",
                "hook_magic": "classes with property hooks or public __get are compile-time fallbacks, not native property-load regions",
                "uninitialized": "typed uninitialized slots side-exit with guard_failed before the interpreter raises the canonical error",
            },
            "valid": {
                "fixture": rel(args.property_valid_fixture),
                "jit_cranelift": {
                    "command": property_valid.command,
                    "exit_code": property_valid.returncode,
                    "stats": property_valid.jit_stats,
                },
            },
            "wrong_class": {
                "fixture": rel(args.property_wrong_class_fixture),
                "jit_cranelift": {
                    "command": property_wrong_class.command,
                    "exit_code": property_wrong_class.returncode,
                    "stats": property_wrong_class.jit_stats,
                },
            },
            "hook_magic": {
                "fixture": rel(args.property_hook_magic_fixture),
                "jit_cranelift": {
                    "command": property_hook_magic.command,
                    "exit_code": property_hook_magic.returncode,
                    "stats": property_hook_magic.jit_stats,
                },
            },
            "uninitialized": {
                "fixture": rel(args.property_uninitialized_fixture),
                "jit_cranelift": {
                    "command": property_uninitialized.command,
                    "exit_code": property_uninitialized.returncode,
                    "stats": property_uninitialized.jit_stats,
                },
            },
        },
        "failures": failures,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        for failure in failures:
            print(f"[fail] {failure}", file=sys.stderr)
        print(f"[fail] Cranelift guard report wrote {rel(args.out)}", file=sys.stderr)
        return 1

    print(f"[pass] Cranelift guard report validated side exits and packed-fetch guards; wrote {rel(args.out)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
