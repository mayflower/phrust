#!/usr/bin/env python3
"""Compare performance flag combinations against a baseline run."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ENGINE = ROOT / "target/debug/php-vm"
DEFAULT_OUT_DIR = ROOT / "target/performance/perf-flag-matrix"
DEFAULT_PERF_FIXTURES = ROOT / "tests/fixtures/performance/regressions"
DEFAULT_SELECTED_FIXTURES = (
    ROOT / "fixtures/runtime/valid/hello.php",
    ROOT / "fixtures/runtime/valid/functions/factorial.php",
    ROOT / "fixtures/runtime/valid/arrays/indexed.php",
    ROOT / "fixtures/runtime/valid/exceptions/catch-finally.php",
    ROOT / "fixtures/runtime/valid/fibers/fiber.php",
    ROOT / "tests/fixtures/stdlib/_harness/stdlib/array_basics.php",
    ROOT / "tests/fixtures/stdlib/_harness/json-pcre-date/json_basics.php",
    ROOT / "tests/fixtures/stdlib/_harness/spl-reflection/reflection_function.php",
)


@dataclass(frozen=True)
class Combo:
    label: str
    args: tuple[str, ...]


@dataclass(frozen=True)
class RunResult:
    returncode: int
    stdout: str
    stderr: str


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def normalize(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--engine",
        type=Path,
        default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_ENGINE)),
    )
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--performance-fixtures", type=Path, default=DEFAULT_PERF_FIXTURES)
    parser.add_argument(
        "--extra-fixture",
        action="append",
        type=Path,
        default=[],
        help="Additional fixture to compare; defaults include selected runtime and stdlib fixtures.",
    )
    parser.add_argument(
        "--no-default-extra-fixtures",
        action="store_true",
        help="Only compare performance regression fixtures and explicit --extra-fixture values.",
    )
    parser.add_argument(
        "--include-jit",
        action="store_true",
        default=os.getenv("PHRUST_PERF_MATRIX_JIT") == "1",
        help="Also compare --jit=cranelift. Intended for feature/platform supported runs.",
    )
    parser.add_argument("--timeout", type=float, default=10.0)
    parser.add_argument("--min-combinations", type=int, default=5)
    return parser.parse_args()


def fixture_paths(perf_dir: Path, extra: list[Path], include_defaults: bool) -> list[Path]:
    if not perf_dir.is_dir():
        raise SystemExit(f"missing performance fixture directory: {perf_dir}")
    fixtures = sorted(path for path in perf_dir.glob("*.php") if path.is_file())
    if include_defaults:
        fixtures.extend(DEFAULT_SELECTED_FIXTURES)
    fixtures.extend(extra)

    resolved: list[Path] = []
    seen: set[Path] = set()
    for fixture in fixtures:
        path = fixture if fixture.is_absolute() else ROOT / fixture
        path = path.resolve()
        if path in seen:
            continue
        if not path.is_file():
            raise SystemExit(f"missing fixture: {path}")
        seen.add(path)
        resolved.append(path)
    if not resolved:
        raise SystemExit("no fixtures selected for performance flag matrix")
    return resolved


def combos(cache_root: Path, include_jit: bool) -> tuple[Combo, list[Combo]]:
    baseline = Combo(
        "baseline",
        (
            "--opt-level=0",
            "--quickening=off",
            "--inline-caches=off",
            "--bytecode-cache=off",
            "--jit=off",
            "--tiering=off",
        ),
    )
    variants = [
        Combo("opt1", ("--opt-level=1", "--quickening=off", "--inline-caches=off", "--bytecode-cache=off", "--jit=off", "--tiering=off")),
        Combo("opt2", ("--opt-level=2", "--quickening=off", "--inline-caches=off", "--bytecode-cache=off", "--jit=off", "--tiering=off")),
        Combo("quickening-on", ("--opt-level=0", "--quickening=on", "--inline-caches=off", "--bytecode-cache=off", "--jit=off")),
        Combo("inline-caches-on", ("--opt-level=0", "--quickening=off", "--inline-caches=on", "--bytecode-cache=off", "--jit=off")),
        Combo(
            "bytecode-cache-read-write",
            (
                "--opt-level=0",
                "--quickening=off",
                "--inline-caches=off",
                "--bytecode-cache=read-write",
                "--bytecode-cache-dir",
                str(cache_root / "bytecode-cache-read-write"),
                "--jit=off",
                "--tiering=off",
            ),
        ),
        Combo(
            "all-non-jit-cranelift",
            (
                "--opt-level=2",
                "--quickening=on",
                "--inline-caches=on",
                "--bytecode-cache=read-write",
                "--bytecode-cache-dir",
                str(cache_root / "all-non-jit-cranelift"),
                "--jit=off",
            ),
        ),
    ]
    if include_jit:
        variants.append(
            Combo(
                "jit-cranelift",
                (
                    "--opt-level=2",
                    "--quickening=on",
                    "--inline-caches=on",
                    "--bytecode-cache=off",
                    "--jit=cranelift",
                ),
            )
        )
    return baseline, variants


def run_combo(
    engine: Path,
    fixture: Path,
    combo: Combo,
    out_dir: Path,
    timeout: float,
) -> RunResult:
    tmp_dir = out_dir / "tmp" / combo.label / fixture.stem
    tmp_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env.update(
        {
            "TZ": "UTC",
            "LC_ALL": "C",
            "LANG": "C",
            "TMPDIR": str(tmp_dir),
            "TMP": str(tmp_dir),
            "TEMP": str(tmp_dir),
            "PHRUST_RANDOM_SEED": "performance-perf-flag-matrix",
            "RUST_TEST_SEED": "performance-perf-flag-matrix",
        }
    )
    completed = subprocess.run(
        [str(engine), "run", *combo.args, rel(fixture)],
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    result = RunResult(
        returncode=completed.returncode,
        stdout=normalize(completed.stdout),
        stderr=normalize(completed.stderr),
    )
    artifact_base = out_dir / "runs" / rel(fixture).replace("/", "__")
    artifact_base.mkdir(parents=True, exist_ok=True)
    (artifact_base / f"{combo.label}.stdout").write_text(result.stdout, encoding="utf-8")
    (artifact_base / f"{combo.label}.stderr").write_text(result.stderr, encoding="utf-8")
    (artifact_base / f"{combo.label}.status").write_text(
        f"{result.returncode}\n",
        encoding="utf-8",
    )
    return result


def unified_diff(name: str, expected: str, actual: str) -> str:
    diff = difflib.unified_diff(
        expected.splitlines(keepends=True),
        actual.splitlines(keepends=True),
        fromfile=f"baseline/{name}",
        tofile=f"variant/{name}",
    )
    text = "".join(diff)
    return text if text else "(no textual diff)\n"


def compare_or_fail(fixture: Path, combo: Combo, baseline: RunResult, actual: RunResult) -> None:
    failures: list[str] = []
    if actual.returncode != baseline.returncode:
        failures.append(
            f"exit code baseline={baseline.returncode} variant={actual.returncode}"
        )
    if actual.stdout != baseline.stdout:
        failures.append("stdout differs:\n" + unified_diff("stdout", baseline.stdout, actual.stdout))
    if actual.stderr != baseline.stderr:
        failures.append("stderr differs:\n" + unified_diff("stderr", baseline.stderr, actual.stderr))
    if failures:
        message = "\n".join(failures)
        raise SystemExit(
            f"[fail] performance flag matrix changed behavior for {rel(fixture)} "
            f"under {combo.label}\n{message}"
        )


def main() -> int:
    args = parse_args()
    engine = args.engine if args.engine.is_absolute() else ROOT / args.engine
    if not engine.is_file() or not os.access(engine, os.X_OK):
        raise SystemExit(f"Rust VM is not executable: {engine}")

    fixtures = fixture_paths(
        args.performance_fixtures,
        args.extra_fixture,
        not args.no_default_extra_fixtures,
    )
    if args.out_dir.exists():
        shutil.rmtree(args.out_dir)
    args.out_dir.mkdir(parents=True)

    baseline_combo, variant_combos = combos(args.out_dir / "cache", args.include_jit)
    if len(variant_combos) < args.min_combinations:
        raise SystemExit(
            f"expected at least {args.min_combinations} combinations, "
            f"configured {len(variant_combos)}"
        )

    compared = 0
    fixture_summaries: list[dict[str, object]] = []
    for fixture in fixtures:
        baseline = run_combo(engine, fixture, baseline_combo, args.out_dir, args.timeout)
        labels: list[str] = []
        for combo in variant_combos:
            actual = run_combo(engine, fixture, combo, args.out_dir, args.timeout)
            compare_or_fail(fixture, combo, baseline, actual)
            labels.append(combo.label)
            compared += 1
        fixture_summaries.append({"fixture": rel(fixture), "variants": labels})

    summary = {
        "engine": rel(engine),
        "baseline": baseline_combo.label,
        "variant_count": len(variant_combos),
        "fixture_count": len(fixtures),
        "comparison_count": compared,
        "include_jit": args.include_jit,
        "fixtures": fixture_summaries,
    }
    (args.out_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        "[pass] performance flag matrix compared "
        f"{len(fixtures)} fixture(s), {len(variant_combos)} variant(s), "
        f"{compared} comparison(s)"
    )
    if not args.include_jit:
        print("[skip] performance flag matrix JIT variant: feature/platform not requested")
    return 0


if __name__ == "__main__":
    sys.exit(main())
