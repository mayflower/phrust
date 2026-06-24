#!/usr/bin/env python3
"""Deterministic runtime-semantics fuzz smoke for refs, COW arrays, and foreach."""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REFERENCE = ROOT / "third_party/php-src/sapi/cli/php"
DEFAULT_VM = ROOT / "target/debug/php-vm"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--seed",
        type=int,
        default=int(os.getenv("PHRUST_RUNTIME_SEMANTICS_FUZZ_SEED", "20260622")),
    )
    parser.add_argument(
        "--cases",
        type=int,
        default=int(os.getenv("PHRUST_RUNTIME_SEMANTICS_FUZZ_CASES", "24")),
    )
    parser.add_argument("--out", type=Path, default=ROOT / "target/runtime-semantics/fuzz-smoke")
    parser.add_argument("--reference-php", type=Path, default=Path(os.getenv("REFERENCE_PHP", DEFAULT_REFERENCE)))
    parser.add_argument("--rust-vm", type=Path, default=Path(os.getenv("PHRUST_PHP_VM", DEFAULT_VM)))
    parser.add_argument("--save-regressions", action="store_true")
    return parser.parse_args()


def php_program(body: str) -> str:
    return (
        "<?php\n"
        "// runtime-semantics: category=regressions expect=pass "
        "regression_category=fuzz reference_behavior=generated regression_case=optional-a\n"
        f"{body}\n"
    )


def generate_case(rng: random.Random, index: int) -> tuple[str, str]:
    a = rng.randint(-8, 20)
    b = rng.randint(-8, 20)
    c = rng.randint(-8, 20)
    variants = [
        (
            "local-reference-write-through",
            f'$x = {a}; $y =& $x; $y = {b}; echo $x, ":", $y, "\\n";',
        ),
        (
            "array-cow-append",
            f'$a = [{a}, {b}]; $b = $a; $b[] = {c}; echo $a[0], ":", $a[1], ":", $b[2], "\\n";',
        ),
        (
            "array-element-reference",
            f'$a = ["k" => {a}]; $r =& $a["k"]; $r = {b}; echo $a["k"], ":", $r, "\\n";',
        ),
        (
            "unset-append-order",
            f'$a = [0 => {a}, 1 => {b}, 2 => {c}]; unset($a[1]); $a[] = {a + b}; '
            'foreach ($a as $k => $v) { echo $k, "=", $v, ";"; } echo "\\n";',
        ),
        (
            "foreach-by-value-snapshot",
            f'$a = [{a}, {b}]; foreach ($a as $v) {{ echo $v, "|"; $a[] = {c}; }} echo $a[2], "\\n";',
        ),
        (
            "foreach-by-reference",
            f'$a = [{a}, {b}]; foreach ($a as &$v) {{ $v = $v + 1; }} unset($v); '
            'echo $a[0], ":", $a[1], "\\n";',
        ),
    ]
    name, body = variants[index % len(variants)]
    return f"{index:03d}-{name}", php_program(body)


def run_command(command: list[str], timeout: float) -> dict[str, object]:
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        return {
            "exit": completed.returncode,
            "stdout": completed.stdout,
            "stderr": normalize(completed.stderr),
            "timeout": False,
        }
    except subprocess.TimeoutExpired as error:
        return {
            "exit": 124,
            "stdout": error.stdout or "",
            "stderr": normalize(error.stderr or ""),
            "timeout": True,
        }


def normalize(text: str) -> str:
    text = text.replace(str(ROOT), "$ROOT")
    return re.sub(r"/private/var/folders/[^:\n ]+", "$TMP", text)


def save_failure(out: Path, name: str, source: str, reference: dict[str, object], rust: dict[str, object]) -> None:
    failures = out / "failures"
    failures.mkdir(parents=True, exist_ok=True)
    (failures / f"{name}.php").write_text(source, encoding="utf-8")
    (failures / f"{name}.reference.txt").write_text(json.dumps(reference, indent=2), encoding="utf-8")
    (failures / f"{name}.rust.txt").write_text(json.dumps(rust, indent=2), encoding="utf-8")


def main() -> int:
    args = parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    if not args.reference_php.exists():
        print(f"[skip] reference PHP not found: {args.reference_php}")
        return 0
    if not args.rust_vm.exists():
        print(f"[skip] php-vm binary not found: {args.rust_vm}")
        return 0

    rng = random.Random(args.seed)
    cases_dir = args.out / "cases"
    cases_dir.mkdir(parents=True, exist_ok=True)
    results: list[dict[str, object]] = []
    failures = 0

    for index in range(args.cases):
        name, source = generate_case(rng, index)
        case_path = cases_dir / f"{name}.php"
        case_path.write_text(source, encoding="utf-8")
        reference = run_command([str(args.reference_php), str(case_path)], timeout=2.0)
        rust = run_command([str(args.rust_vm), "run", str(case_path)], timeout=2.0)
        ok = (
            reference["exit"] == rust["exit"]
            and reference["stdout"] == rust["stdout"]
            and reference["stderr"] == rust["stderr"]
        )
        results.append({"name": name, "ok": ok, "reference": reference, "rust": rust})
        if not ok:
            failures += 1
            save_failure(args.out, name, source, reference, rust)
            if args.save_regressions:
                target = ROOT / "fixtures/runtime_semantics/regressions" / f"fuzz-{name}.php"
                shutil.copyfile(case_path, target)

    report = {
        "seed": args.seed,
        "total": len(results),
        "pass": len(results) - failures,
        "fail": failures,
        "results": results,
    }
    (args.out / "runtime-semantics-fuzz-smoke-report.json").write_text(
        json.dumps(report, indent=2), encoding="utf-8"
    )
    if failures:
        print(f"[fail] runtime-semantics fuzz smoke: total={len(results)} fail={failures} path={args.out}")
        return 1
    print(f"[ok] runtime-semantics fuzz smoke: total={len(results)} pass={len(results)} path={args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
