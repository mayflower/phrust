#!/usr/bin/env python3
"""Build and benchmark production-oriented php-vm profiles."""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python 3.11+ in repo policy.
    tomllib = None  # type: ignore[assignment]


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUT_DIR = ROOT / "target/performance/release"
PERF_FIXTURES = ROOT / "tests/fixtures/performance/perf_smoke"
FRAMEWORK_FIXTURES = ROOT / "tests/fixtures/performance/framework_smoke"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "mode",
        choices=("release", "pgo", "bolt"),
        help="Production profile smoke to run.",
    )
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument(
        "--repetitions",
        type=int,
        default=int(os.getenv("PHRUST_RELEASE_BENCH_REPETITIONS", "1")),
    )
    parser.add_argument(
        "--warmups",
        type=int,
        default=int(os.getenv("PHRUST_RELEASE_BENCH_WARMUPS", "0")),
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=float(os.getenv("PHRUST_RELEASE_BENCH_TIMEOUT", "10.0")),
    )
    return parser.parse_args()


def rel(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def cargo_target_dir(env: dict[str, str] | None = None) -> Path:
    source = env if env is not None else os.environ
    value = source.get("CARGO_TARGET_DIR", "target")
    path = Path(value)
    return path if path.is_absolute() else ROOT / path


def profile_dir(profile: str) -> str:
    return "release" if profile == "release" else profile


def binary_path(profile: str, env: dict[str, str] | None = None) -> Path:
    return cargo_target_dir(env) / profile_dir(profile) / "php-vm"


def display_command(command: list[str]) -> list[str]:
    return [rel(Path(part)) if part.startswith(str(ROOT)) else part for part in command]


def run_command(
    command: list[str],
    *,
    env: dict[str, str] | None = None,
    quiet: bool = False,
) -> tuple[int, str, str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE if quiet else None,
        stderr=subprocess.PIPE if quiet else None,
        check=False,
    )
    return completed.returncode, completed.stdout or "", completed.stderr or ""


def merged_env(extra: dict[str, str] | None = None) -> dict[str, str]:
    env = dict(os.environ)
    if extra:
        env.update(extra)
    return env


def append_rustflags(env: dict[str, str], flags: str) -> dict[str, str]:
    current = env.get("RUSTFLAGS", "").strip()
    env["RUSTFLAGS"] = f"{current} {flags}".strip()
    return env


def load_profiles() -> dict[str, Any]:
    if tomllib is None:
        return {}
    data = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    profiles = data.get("profile")
    return profiles if isinstance(profiles, dict) else {}


def build_profile(profile: str, env: dict[str, str] | None = None) -> dict[str, Any]:
    command = ["cargo", "build", "--profile", profile, "-p", "php_vm_cli", "--bin", "php-vm"]
    code, stdout, stderr = run_command(command, env=env)
    return {
        "kind": "build",
        "profile": profile,
        "command": command,
        "status": "pass" if code == 0 else "fail",
        "exit_code": code,
        "stdout_tail": stdout[-4000:],
        "stderr_tail": stderr[-4000:],
    }


def run_benchmarks(
    binary: Path,
    out_dir: Path,
    label: str,
    args: argparse.Namespace,
    env: dict[str, str] | None = None,
) -> list[dict[str, Any]]:
    bench_json = out_dir / f"{label}-benchmark-smoke.json"
    framework_json = out_dir / f"{label}-framework-smoke.json"
    framework_md = out_dir / f"{label}-framework-smoke.md"
    commands = [
        [
            str(ROOT / "scripts/performance/bench_matrix.py"),
            "--engine",
            str(binary),
            "--fixtures-dir",
            str(PERF_FIXTURES),
            "--out",
            str(bench_json),
            "--repetitions",
            str(args.repetitions),
            "--warmups",
            str(args.warmups),
            "--timeout",
            str(args.timeout),
        ],
        [
            str(ROOT / "scripts/performance/framework_micro_smoke.py"),
            "--engine",
            str(binary),
            "--fixtures",
            str(FRAMEWORK_FIXTURES),
            "--out",
            str(framework_json),
            "--markdown-out",
            str(framework_md),
        ],
    ]
    results = []
    for command in commands:
        code, stdout, stderr = run_command(command, env=env)
        results.append(
            {
                "kind": "benchmark",
                "command": command,
                "status": "pass" if code == 0 else "fail",
                "exit_code": code,
                "stdout_tail": stdout[-4000:],
                "stderr_tail": stderr[-4000:],
            }
        )
    return results


def summary_paths(out_dir: Path, mode: str) -> tuple[Path, Path]:
    return out_dir / f"{mode}-summary.json", out_dir / f"{mode}-summary.md"


def read_json(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else None


def report_status(steps: list[dict[str, Any]]) -> str:
    return "pass" if all(step.get("status") == "pass" for step in steps) else "fail"


def write_report(summary: dict[str, Any], out_dir: Path, mode: str) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_out, md_out = summary_paths(out_dir, mode)
    json_out.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    md_out.write_text(render_markdown(summary), encoding="utf-8")


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        f"# {summary['title']}",
        "",
        "This report is local production-profile evidence. Wall-clock timings are",
        "advisory and are not pass/fail budgets.",
        "",
        "## Summary",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | `{summary['status']}` |",
        f"| Mode | `{summary['mode']}` |",
        f"| Binary | `{summary.get('binary', '-')}` |",
        f"| Host | `{summary['environment']['platform']}` |",
        f"| Repetitions | `{summary['settings']['repetitions']}` |",
        f"| Warmups | `{summary['settings']['warmups']}` |",
        "",
    ]
    if summary.get("reason"):
        lines.extend(["## Reason", "", str(summary["reason"]), ""])
    reports = summary.get("reports")
    if isinstance(reports, dict):
        lines.extend(["## Reports", "", "| Report | Path |", "| --- | --- |"])
        for name, path in sorted(reports.items()):
            lines.append(f"| {name} | `{path}` |")
        lines.append("")
    lines.extend(["## Steps", "", "| Step | Status | Command |", "| --- | --- | --- |"])
    for step in summary.get("steps", []):
        command = " ".join(display_command(step.get("command", [])))
        lines.append(f"| {step.get('kind', '-')} | `{step.get('status', '-')}` | `{command}` |")
    lines.append("")
    return "\n".join(lines)


def base_summary(mode: str, args: argparse.Namespace) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "mode": mode,
        "title": {
            "release": "Performance Release Benchmark Smoke",
            "pgo": "Performance PGO Benchmark Smoke",
            "bolt": "Performance BOLT Benchmark Smoke",
        }[mode],
        "environment": {
            "platform": platform.platform(),
            "system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "settings": {
            "repetitions": args.repetitions,
            "warmups": args.warmups,
            "timeout": args.timeout,
        },
        "cargo_profiles": load_profiles(),
        "timing_policy": "wall-clock timings are advisory and are not CI budgets",
        "steps": [],
    }


def write_skip(mode: str, args: argparse.Namespace, reason: str) -> int:
    summary = base_summary(mode, args)
    summary.update({"status": "skip", "reason": reason})
    write_report(summary, args.out_dir, mode)
    json_out, md_out = summary_paths(args.out_dir, mode)
    print(f"[skip] performance {mode} smoke: {reason}; wrote {rel(json_out)} and {rel(md_out)}")
    return 0


def run_release(args: argparse.Namespace) -> int:
    summary = base_summary("release", args)
    build = build_profile("release")
    steps = [build]
    binary = binary_path("release")
    if build["status"] == "pass":
        steps.extend(run_benchmarks(binary, args.out_dir, "release", args))
    summary.update(
        {
            "status": report_status(steps),
            "binary": rel(binary),
            "reports": {
                "benchmark_json": rel(args.out_dir / "release-benchmark-smoke.json"),
                "framework_json": rel(args.out_dir / "release-framework-smoke.json"),
                "framework_markdown": rel(args.out_dir / "release-framework-smoke.md"),
            },
            "steps": steps,
            "benchmark": read_json(args.out_dir / "release-benchmark-smoke.json"),
            "framework_smoke": read_json(args.out_dir / "release-framework-smoke.json"),
        }
    )
    write_report(summary, args.out_dir, "release")
    json_out, md_out = summary_paths(args.out_dir, "release")
    print(
        f"[{summary['status']}] performance release benchmark smoke wrote "
        f"{rel(json_out)} and {rel(md_out)}"
    )
    return 0 if summary["status"] == "pass" else 1


def run_pgo(args: argparse.Namespace) -> int:
    if os.getenv("PHRUST_RUN_PGO") != "1":
        return write_skip("pgo", args, "set PHRUST_RUN_PGO=1 to run optional PGO profile flow")
    llvm_profdata = shutil.which("llvm-profdata")
    if llvm_profdata is None:
        return write_skip("pgo", args, "llvm-profdata is unavailable")

    summary = base_summary("pgo", args)
    pgo_root = args.out_dir / "pgo"
    pgo_data = pgo_root / "data"
    pgo_data.mkdir(parents=True, exist_ok=True)
    gen_env = merged_env({"CARGO_TARGET_DIR": str(pgo_root / "generate-target")})
    append_rustflags(gen_env, f"-Cprofile-generate={pgo_data}")
    gen_env["LLVM_PROFILE_FILE"] = str(pgo_data / "phrust-%p-%m.profraw")
    gen_build = build_profile("release", gen_env)
    steps = [gen_build]
    gen_binary = binary_path("release", gen_env)
    if gen_build["status"] == "pass":
        steps.extend(run_benchmarks(gen_binary, args.out_dir, "pgo-training", args, gen_env))

    profraws = sorted(pgo_data.glob("*.profraw"))
    profdata = pgo_root / "pgo.profdata"
    if report_status(steps) == "pass" and profraws:
        merge_command = [llvm_profdata, "merge", "-output", str(profdata), *map(str, profraws)]
        code, stdout, stderr = run_command(merge_command)
        steps.append(
            {
                "kind": "pgo-merge",
                "command": merge_command,
                "status": "pass" if code == 0 else "fail",
                "exit_code": code,
                "stdout_tail": stdout[-4000:],
                "stderr_tail": stderr[-4000:],
            }
        )
    elif report_status(steps) == "pass":
        steps.append(
            {
                "kind": "pgo-merge",
                "command": [llvm_profdata, "merge", "-output", str(profdata), str(pgo_data)],
                "status": "fail",
                "exit_code": 1,
                "stderr_tail": f"no .profraw files found under {rel(pgo_data)}",
            }
        )

    use_env = merged_env({"CARGO_TARGET_DIR": str(pgo_root / "use-target")})
    append_rustflags(use_env, f"-Cprofile-use={profdata} -Cllvm-args=-pgo-warn-missing-function")
    if report_status(steps) == "pass":
        use_build = build_profile("release", use_env)
        steps.append(use_build)
        use_binary = binary_path("release", use_env)
        if use_build["status"] == "pass":
            steps.extend(run_benchmarks(use_binary, args.out_dir, "pgo", args, use_env))
    else:
        use_binary = binary_path("release", use_env)

    summary.update(
        {
            "status": report_status(steps),
            "binary": rel(use_binary),
            "reports": {
                "training_benchmark_json": rel(args.out_dir / "pgo-training-benchmark-smoke.json"),
                "benchmark_json": rel(args.out_dir / "pgo-benchmark-smoke.json"),
                "framework_json": rel(args.out_dir / "pgo-framework-smoke.json"),
                "profdata": rel(profdata),
            },
            "steps": steps,
        }
    )
    write_report(summary, args.out_dir, "pgo")
    json_out, md_out = summary_paths(args.out_dir, "pgo")
    print(f"[{summary['status']}] performance PGO benchmark smoke wrote {rel(json_out)} and {rel(md_out)}")
    return 0 if summary["status"] == "pass" else 1


def run_bolt(args: argparse.Namespace) -> int:
    if platform.system() != "Linux":
        return write_skip("bolt", args, f"BOLT is Linux-only for this gate; host is {platform.system()}")
    if os.getenv("PHRUST_RUN_BOLT") != "1":
        return write_skip("bolt", args, "set PHRUST_RUN_BOLT=1 to run optional BOLT flow")
    llvm_bolt = shutil.which("llvm-bolt")
    perf2bolt = shutil.which("perf2bolt")
    perf_data_env = os.getenv("PHRUST_BOLT_PERF_DATA")
    if llvm_bolt is None:
        return write_skip("bolt", args, "llvm-bolt is unavailable")
    if perf2bolt is None:
        return write_skip("bolt", args, "perf2bolt is unavailable")
    if not perf_data_env:
        return write_skip("bolt", args, "set PHRUST_BOLT_PERF_DATA to an existing perf.data file")
    perf_data = Path(perf_data_env)
    if not perf_data.is_file():
        return write_skip("bolt", args, f"PHRUST_BOLT_PERF_DATA is not a file: {perf_data}")

    summary = base_summary("bolt", args)
    env = merged_env({"CARGO_TARGET_DIR": str(args.out_dir / "bolt-target")})
    build = build_profile("profiling", env)
    steps = [build]
    source_binary = binary_path("profiling", env)
    fdata = args.out_dir / "bolt.fdata"
    bolt_binary = args.out_dir / "php-vm.bolt"
    if build["status"] == "pass":
        perf2bolt_command = [perf2bolt, str(source_binary), "-p", str(perf_data), "-o", str(fdata)]
        code, stdout, stderr = run_command(perf2bolt_command)
        steps.append(
            {
                "kind": "perf2bolt",
                "command": perf2bolt_command,
                "status": "pass" if code == 0 else "fail",
                "exit_code": code,
                "stdout_tail": stdout[-4000:],
                "stderr_tail": stderr[-4000:],
            }
        )
    if report_status(steps) == "pass":
        bolt_command = [
            llvm_bolt,
            str(source_binary),
            "-data",
            str(fdata),
            "-o",
            str(bolt_binary),
            "-reorder-blocks=ext-tsp",
            "-reorder-functions=hfsort",
            "-split-functions",
            "-split-all-cold",
        ]
        code, stdout, stderr = run_command(bolt_command)
        steps.append(
            {
                "kind": "llvm-bolt",
                "command": bolt_command,
                "status": "pass" if code == 0 else "fail",
                "exit_code": code,
                "stdout_tail": stdout[-4000:],
                "stderr_tail": stderr[-4000:],
            }
        )
    if report_status(steps) == "pass":
        bolt_binary.chmod(0o755)
        steps.extend(run_benchmarks(bolt_binary, args.out_dir, "bolt", args))

    summary.update(
        {
            "status": report_status(steps),
            "binary": rel(bolt_binary),
            "reports": {
                "benchmark_json": rel(args.out_dir / "bolt-benchmark-smoke.json"),
                "framework_json": rel(args.out_dir / "bolt-framework-smoke.json"),
                "bolt_fdata": rel(fdata),
            },
            "steps": steps,
        }
    )
    write_report(summary, args.out_dir, "bolt")
    json_out, md_out = summary_paths(args.out_dir, "bolt")
    print(f"[{summary['status']}] performance BOLT benchmark smoke wrote {rel(json_out)} and {rel(md_out)}")
    return 0 if summary["status"] == "pass" else 1


def main() -> int:
    started = time.perf_counter()
    args = parse_args()
    if args.repetitions < 0 or args.warmups < 0:
        raise SystemExit("repetitions and warmups must be non-negative")
    if args.timeout <= 0:
        raise SystemExit("timeout must be positive")
    args.out_dir.mkdir(parents=True, exist_ok=True)
    if args.mode == "release":
        code = run_release(args)
    elif args.mode == "pgo":
        code = run_pgo(args)
    else:
        code = run_bolt(args)
    elapsed = time.perf_counter() - started
    print(f"[info] performance {args.mode} smoke elapsed {elapsed:.2f}s")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
