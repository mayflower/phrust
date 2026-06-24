#!/usr/bin/env python3
"""Differential harness for performance Cranelift JIT modes."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_FIXTURES = ROOT / "tests/fixtures/performance/cranelift"
DEFAULT_OUT = ROOT / "target/performance/cranelift/diff.json"
DEFAULT_DIFF_DIR = ROOT / "target/performance/cranelift/diff"
DEFAULT_REFERENCE = ROOT / "third_party/php-src/sapi/cli/php"


@dataclass(frozen=True)
class Sample:
    mode: str
    command: list[str]
    returncode: int
    stdout: str
    stderr: str
    diagnostics: list[dict[str, Any]]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_ENGINE)))
    parser.add_argument("--fixtures-dir", type=Path, default=DEFAULT_FIXTURES)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--diff-dir", type=Path, default=DEFAULT_DIFF_DIR)
    parser.add_argument("--timeout", type=float, default=float(os.getenv("PHRUST_CRANELIFT_DIFF_TIMEOUT", "10.0")))
    parser.add_argument("--reference-php", type=Path, default=None)
    return parser.parse_args()


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def fixture_paths(fixtures_dir: Path) -> list[Path]:
    if not fixtures_dir.is_dir():
        raise SystemExit(f"missing Cranelift fixture directory: {fixtures_dir}")
    fixtures = sorted(path for path in fixtures_dir.rglob("*.php") if path.is_file())
    if not fixtures:
        raise SystemExit(f"no Cranelift fixtures found under {fixtures_dir}")
    return fixtures


def reference_php_path(explicit: Path | None) -> tuple[Path | None, str | None]:
    if explicit is not None:
        if explicit.is_file() and os.access(explicit, os.X_OK):
            return explicit, None
        raise SystemExit(f"reference PHP is not executable: {explicit}")
    env_path = os.getenv("REFERENCE_PHP")
    if env_path:
        path = Path(env_path)
        if path.is_file() and os.access(path, os.X_OK):
            return path, None
        raise SystemExit(f"REFERENCE_PHP is not executable: {path}")
    if DEFAULT_REFERENCE.is_file() and os.access(DEFAULT_REFERENCE, os.X_OK):
        return DEFAULT_REFERENCE, None
    return None, "REFERENCE_PHP not set and third_party/php-src/sapi/cli/php is unavailable"


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
            "PHRUST_RANDOM_SEED": "performance-cranelift-jit-diff",
            "RUST_TEST_SEED": "performance-cranelift-jit-diff",
        }
    )
    return env


def normalize_text(value: str) -> str:
    return value.replace("\r\n", "\n").replace("\r", "\n")


def extract_json_diagnostics(stderr: str) -> list[dict[str, Any]]:
    diagnostics: list[dict[str, Any]] = []
    for line in stderr.splitlines():
        stripped = line.strip()
        if not stripped.startswith("{"):
            continue
        try:
            decoded = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        if isinstance(decoded, dict):
            diagnostics.append(decoded)
    return diagnostics


def command_for(engine: Path, fixture: Path, mode: str) -> list[str]:
    if mode == "reference":
        return [str(engine), rel(fixture)]
    command = [str(engine), "run", f"--jit={mode}"]
    if mode == "cranelift":
        command.append("--jit-eager")
    command.append(rel(fixture))
    return command


def run_sample(
    *,
    engine: Path,
    fixture: Path,
    mode: str,
    tmp_dir: Path,
    timeout: float,
) -> Sample:
    tmp_dir.mkdir(parents=True, exist_ok=True)
    command = command_for(engine, fixture, mode)
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
    return Sample(
        mode=mode,
        command=[rel(Path(command[0])), *command[1:]],
        returncode=completed.returncode,
        stdout=normalize_text(completed.stdout),
        stderr=stderr,
        diagnostics=extract_json_diagnostics(stderr),
    )


def unified_diff(name: str, before: str, after: str) -> str:
    return "".join(
        difflib.unified_diff(
            before.splitlines(keepends=True),
            after.splitlines(keepends=True),
            fromfile=f"{name}:jit-off",
            tofile=f"{name}:jit-cranelift",
        )
    )


def compare_samples(fixture: Path, off: Sample, cranelift: Sample, diff_dir: Path) -> dict[str, Any] | None:
    sections: list[str] = []
    diff_parts: list[str] = []
    if off.returncode != cranelift.returncode:
        sections.append("exit_code")
        diff_parts.append(f"exit_code jit-off={off.returncode} jit-cranelift={cranelift.returncode}\n")
    if off.stdout != cranelift.stdout:
        sections.append("stdout")
        diff_parts.append(unified_diff("stdout", off.stdout, cranelift.stdout))
    if off.stderr != cranelift.stderr:
        sections.append("stderr")
        diff_parts.append(unified_diff("stderr", off.stderr, cranelift.stderr))
    if off.diagnostics != cranelift.diagnostics:
        sections.append("diagnostics")
        diff_parts.append(
            unified_diff(
                "diagnostics",
                json.dumps(off.diagnostics, sort_keys=True, indent=2) + "\n",
                json.dumps(cranelift.diagnostics, sort_keys=True, indent=2) + "\n",
            )
        )
    if not sections:
        return None

    diff_dir.mkdir(parents=True, exist_ok=True)
    diff_path = diff_dir / f"{fixture.relative_to(ROOT).as_posix().replace('/', '__')}.diff"
    diff_path.write_text("\n".join(diff_parts), encoding="utf-8")
    return {
        "fixture": rel(fixture),
        "sections": sections,
        "diff": rel(diff_path),
    }


def sample_json(sample: Sample) -> dict[str, Any]:
    return {
        "mode": sample.mode,
        "command": sample.command,
        "exit_code": sample.returncode,
        "stdout_sha256": hashlib.sha256(sample.stdout.encode()).hexdigest(),
        "stderr_sha256": hashlib.sha256(sample.stderr.encode()).hexdigest(),
        "diagnostics": sample.diagnostics,
    }


def main() -> int:
    args = parse_args()
    engine = args.engine
    if not engine.is_file() or not os.access(engine, os.X_OK):
        raise SystemExit(f"Rust VM is not executable: {engine}")
    fixtures = fixture_paths(args.fixtures_dir)
    reference, reference_skip = reference_php_path(args.reference_php)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.diff_dir.mkdir(parents=True, exist_ok=True)
    for stale in args.diff_dir.glob("*.diff"):
        stale.unlink()

    rows: list[dict[str, Any]] = []
    differences: list[dict[str, Any]] = []
    for fixture in fixtures:
        fixture_key = fixture.relative_to(ROOT).as_posix().replace("/", "__")
        off = run_sample(
            engine=engine,
            fixture=fixture,
            mode="off",
            tmp_dir=ROOT / "target/performance/cranelift/tmp" / fixture_key / "off",
            timeout=args.timeout,
        )
        cranelift = run_sample(
            engine=engine,
            fixture=fixture,
            mode="cranelift",
            tmp_dir=ROOT / "target/performance/cranelift/tmp" / fixture_key / "cranelift",
            timeout=args.timeout,
        )
        difference = compare_samples(fixture, off, cranelift, args.diff_dir)
        if difference is not None:
            differences.append(difference)

        reference_sample = None
        if reference is not None:
            reference_sample = run_sample(
                engine=reference,
                fixture=fixture,
                mode="reference",
                tmp_dir=ROOT / "target/performance/cranelift/tmp" / fixture_key / "reference",
                timeout=args.timeout,
            )

        rows.append(
            {
                "fixture": rel(fixture),
                "jit_off": sample_json(off),
                "jit_cranelift": sample_json(cranelift),
                "reference_php": sample_json(reference_sample) if reference_sample else None,
            }
        )

    report = {
        "gate": "jit-cranelift-diff",
        "status": "fail" if differences else "pass",
        "engine": rel(engine),
        "fixture_count": len(fixtures),
        "reference_php": rel(reference) if reference else None,
        "reference_skip": reference_skip,
        "differences": differences,
        "rows": rows,
    }
    args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if differences:
        for difference in differences:
            print(
                f"[fail] {difference['fixture']} diverged in {','.join(difference['sections'])}; "
                f"see {difference['diff']}",
                file=sys.stderr,
            )
        print(f"[fail] Cranelift JIT diff wrote {rel(args.out)}", file=sys.stderr)
        return 1

    print(f"[pass] Cranelift JIT diff compared {len(fixtures)} fixture(s); wrote {rel(args.out)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
