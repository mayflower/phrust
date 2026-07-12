#!/usr/bin/env python3
"""Compare clean Phrust and PHP-FPM/OPcache WordPress HTTP requests."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

WORDPRESS_SCRIPT_DIR = Path(__file__).resolve().parents[1] / "wordpress"
sys.path.insert(0, str(WORDPRESS_SCRIPT_DIR))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from common import REPO_ROOT, now_run_id, repo_path, wordpress_shape_blockers  # noqa: E402
from http_engine_benchmark import (  # noqa: E402
    HttpTarget,
    bootstrap_percentile_ci,
    bootstrap_percentile_ratio_ci,
    http_get,
    normalize_headers,
    percentile,
    sample_curve,
)

DEFAULT_OUT_DIR = REPO_ROOT / "target/performance/wordpress-root"
DEFAULT_PHP_FPM_IMAGE = "phrust-php-fpm:8.5.7"
DEFAULT_NGINX_IMAGE = "nginx:1.28.0-alpine"
DEFAULT_OBSERVABLES = (("root", "/"),)
TARGET_PHP_VERSION = "8.5.7"
CLEAN_TIMING_FORBIDDEN_ENV = (
    "PHRUST_PERF_TRACE",
    "PHRUST_SERVER_PERF_TRACE_VM_COUNTERS",
    "PHRUST_REQUEST_PROFILE",
    "PHRUST_REQUEST_PROFILE_VM_COUNTERS",
    "PHRUST_REQUEST_PROFILE_SOURCE_ATTRIBUTION",
    "PHRUST_PERF_ABLATION",
)
# Clean runs override and report these values instead of inheriting ambient
# state. Ablations and instrumentation remain hard failures above.
MANAGED_CLEAN_ENV = {
    "PHRUST_JIT_COPY_PATCH": "1",
    "PHRUST_INCLUDE_REVALIDATE_MS": "2000",
    "PHRUST_WORKER_SYMBOL_EPOCH": "1",
    "PHRUST_PERSISTENT_FEEDBACK": "1",
}


@dataclass
class ManagedTarget:
    target: HttpTarget
    command: list[str]
    identity: dict[str, Any]
    cleanup_commands: list[list[str]] = field(default_factory=list)
    process: subprocess.Popen[str] | None = None
    log: Any = None
    artifacts: dict[str, str] = field(default_factory=dict)

    def stop(self) -> None:
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if self.log is not None:
            self.log.close()
        for command in self.cleanup_commands:
            subprocess.run(command, cwd=REPO_ROOT, check=False, capture_output=True, text=True)


def main() -> int:
    args = parse_args()
    if args.self_test:
        return self_test()
    out_dir = output_dir(args)
    out_dir.mkdir(parents=True, exist_ok=True)
    if args.feedback_ab:
        report = run_feedback_ab(args, out_dir)
        write_json(report, out_dir / "summary.json")
        write_feedback_ab_markdown(report, out_dir / "summary.md")
        print(f"[{report['status']}] feedback A/B wrote {rel(out_dir / 'summary.md')}")
        if report["status"] == "fail":
            return 1
        if report["status"] == "skip" and args.strict:
            return 2
        return 0
    if args.cranelift_ab:
        report = run_cranelift_ab(args, out_dir)
        write_json(report, out_dir / "summary.json")
        write_cranelift_ab_markdown(report, out_dir / "summary.md")
        print(f"[{report['status']}] Cranelift A/B wrote {rel(out_dir / 'summary.md')}")
        if report["status"] == "fail":
            return 1
        if report["status"] == "skip" and args.strict:
            return 2
        return 0
    report = run(args, out_dir)
    write_json(report, out_dir / "summary.json")
    write_markdown(report, out_dir / "summary.md")
    print(f"[{report['status']}] wordpress performance gate wrote {rel(out_dir / 'summary.md')}")
    if report["status"] == "pass" and args.record_baseline:
        baseline_path = repo_path(args.record_baseline) or Path(args.record_baseline).expanduser()
        baseline_path.parent.mkdir(parents=True, exist_ok=True)
        write_json(report, baseline_path)
        print(f"[ok] recorded baseline at {rel(baseline_path)}")
    if report["status"] == "fail":
        return 1
    if report["status"] == "skip" and args.strict:
        return 2
    return 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("clean", "diagnostic"), default="clean")
    parser.add_argument(
        "--phrust-url",
        "--url",
        dest="phrust_url",
        default=os.environ.get("PHRUST_WORDPRESS_PHRUST_URL", os.environ.get("PHRUST_WORDPRESS_URL", "")),
    )
    parser.add_argument("--php-url", default=os.environ.get("PHRUST_WORDPRESS_PHP_URL", ""))
    parser.add_argument("--host-header", default=os.environ.get("PHRUST_WORDPRESS_HOST_HEADER", ""))
    parser.add_argument("--wordpress-dir", default=os.environ.get("PHRUST_WORDPRESS_DIR", ""))
    parser.add_argument("--docroot", default=os.environ.get("PHRUST_WORDPRESS_DOCROOT", ""))
    parser.add_argument(
        "--server",
        default=os.environ.get("PHRUST_SERVER", "target/release/phrust-server"),
    )
    parser.add_argument("--php-version", default=os.environ.get("PHRUST_WORDPRESS_PHP_VERSION", ""))
    parser.add_argument("--php-fpm-image", default=os.environ.get("PHRUST_PHP_FPM_IMAGE", DEFAULT_PHP_FPM_IMAGE))
    parser.add_argument("--nginx-image", default=os.environ.get("PHRUST_NGINX_IMAGE", DEFAULT_NGINX_IMAGE))
    parser.add_argument("--docker-network", default=os.environ.get("PHRUST_WORDPRESS_DOCKER_NETWORK", ""))
    parser.add_argument("--out-dir", default="")
    parser.add_argument(
        "--samples",
        type=int,
        default=int(
            os.environ.get(
                "PHRUST_WORDPRESS_ROOT_SAMPLES", str(max(30, available_cpus() * 2))
            )
        ),
    )
    parser.add_argument("--warmups", type=int, default=int(os.environ.get("PHRUST_WORDPRESS_ROOT_WARMUPS", "5")))
    parser.add_argument(
        "--concurrency",
        default=os.environ.get("PHRUST_WORDPRESS_CONCURRENCY", ""),
        help="comma-separated levels; default is 1, CPU count, and twice CPU count",
    )
    parser.add_argument("--path", default="/")
    parser.add_argument(
        "--observable",
        action="append",
        default=[],
        metavar="NAME=PATH",
        help="HTTP observable compared after timing; repeat as needed",
    )
    parser.add_argument("--database-identity", default=os.environ.get("PHRUST_WORDPRESS_DB_IDENTITY", ""))
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--metrics-token", default=os.environ.get("PHRUST_METRICS_TOKEN", ""))
    parser.add_argument(
        "--engine-preset",
        choices=("default", "experimental-jit"),
        default="default",
        help="Phrust execution preset; experimental-jit requires a jit-cranelift build",
    )
    parser.add_argument(
        "--persistent-feedback",
        choices=("on", "off"),
        default="on",
        help="A/B switch for request-persistent quickening and callsite feedback",
    )
    parser.add_argument(
        "--feedback-ab",
        action="store_true",
        help="run isolated persistent-feedback off/on arms and write one comparison report",
    )
    parser.add_argument(
        "--copy-patch",
        choices=("on", "off"),
        default="on",
        help="A/B switch for the ARM copy-and-patch native tier",
    )
    parser.add_argument(
        "--cranelift-ab",
        action="store_true",
        help="run ARM experimental-jit across feedback and copy-patch off/on arms",
    )
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--baseline", default="")
    parser.add_argument("--compare", default="", help="legacy baseline comparison; implies --strict")
    parser.add_argument("--record-baseline", default="")
    parser.add_argument("--max-latency-regression-pct", type=float, default=20.0)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.compare:
        args.baseline = args.compare
        args.strict = True
    return args


def output_dir(args: argparse.Namespace) -> Path:
    if args.out_dir:
        return repo_path(args.out_dir) or Path(args.out_dir).expanduser()
    return DEFAULT_OUT_DIR / now_run_id(args.mode)


def run(args: argparse.Namespace, out_dir: Path) -> dict[str, Any]:
    errors = validate_configuration(args)
    if errors:
        return failure_report(errors, args, out_dir)
    docroot = repo_path(args.docroot) or repo_path(args.wordpress_dir)
    if not args.phrust_url:
        blockers = wordpress_shape_blockers(docroot)
        if blockers:
            return unavailable_report(blockers, args, out_dir)
    targets: list[ManagedTarget] = []
    try:
        phrust = resolve_phrust(args, out_dir, docroot)
        targets.append(phrust)
        if args.mode == "diagnostic":
            return collect_diagnostics(args, out_dir, phrust, docroot)
        php = resolve_php_fpm(args, out_dir, docroot)
        targets.append(php)
        return collect_clean(args, out_dir, phrust, php, docroot)
    except EnvironmentError as error:
        return unavailable_report([str(error)], args, out_dir)
    except Exception as error:
        return failure_report([str(error)], args, out_dir)
    finally:
        for target in reversed(targets):
            target.stop()


def run_feedback_ab(args: argparse.Namespace, out_dir: Path) -> dict[str, Any]:
    """Run feedback-off and feedback-on arms with an explicit joint report."""
    errors = validate_configuration(args)
    if errors:
        return {
            "schema_version": 1,
            "status": "fail",
            "mode": "feedback-ab",
            "timing_eligible": False,
            "comparison": [],
            "arms": {},
            "failures": errors,
        }
    arms: dict[str, dict[str, Any]] = {}
    for arm in ("off", "on"):
        arm_args = copy.copy(args)
        arm_args.feedback_ab = False
        arm_args.persistent_feedback = arm
        arm_args.out_dir = str(out_dir / arm)
        arm_args.baseline = ""
        arm_args.compare = ""
        arm_args.record_baseline = ""
        arm_dir = out_dir / arm
        arm_dir.mkdir(parents=True, exist_ok=True)
        arm_report = run(arm_args, arm_dir)
        write_json(arm_report, arm_dir / "summary.json")
        write_markdown(arm_report, arm_dir / "summary.md")
        arms[arm] = arm_report

    statuses = {report.get("status") for report in arms.values()}
    status = "fail" if "fail" in statuses else "skip" if "skip" in statuses else "pass"
    return {
        "schema_version": 1,
        "status": status,
        "mode": "feedback-ab",
        "timing_eligible": all(report.get("timing_eligible") is True for report in arms.values()),
        "comparison": build_feedback_ab_ratios(arms["off"], arms["on"]),
        "arms": {
            arm: {
                "status": report.get("status"),
                "summary_json": rel(out_dir / arm / "summary.json"),
                "summary_markdown": rel(out_dir / arm / "summary.md"),
                "phrust_identity": ((report.get("engines") or {}).get("phrust") or {}).get("identity"),
                "correctness_failures": (report.get("correctness") or {}).get("failures", []),
            }
            for arm, report in arms.items()
        },
    }


def run_cranelift_ab(args: argparse.Namespace, out_dir: Path) -> dict[str, Any]:
    """Run clean and diagnostic evidence for the ARM native-tier matrix."""
    errors = validate_configuration(args)
    if errors:
        return {
            "schema_version": 1,
            "status": "fail",
            "mode": "cranelift-ab",
            "timing_eligible": False,
            "arms": {},
            "failures": errors,
        }
    arms: dict[str, dict[str, Any]] = {}
    full_reports: dict[str, dict[str, Any]] = {}
    for feedback in ("off", "on"):
        for copy_patch in ("off", "on"):
            arm = f"feedback-{feedback}-copy-patch-{copy_patch}"
            arm_dir = out_dir / arm
            arm_dir.mkdir(parents=True, exist_ok=True)
            clean_args = copy.copy(args)
            clean_args.cranelift_ab = False
            clean_args.engine_preset = "experimental-jit"
            clean_args.persistent_feedback = feedback
            clean_args.copy_patch = copy_patch
            clean_args.mode = "clean"
            clean_args.baseline = ""
            clean_args.compare = ""
            clean_args.record_baseline = ""
            clean_dir = arm_dir / "clean"
            clean_dir.mkdir(parents=True, exist_ok=True)
            clean_report = run(clean_args, clean_dir)
            write_json(clean_report, clean_dir / "summary.json")
            write_markdown(clean_report, clean_dir / "summary.md")

            diagnostic_args = copy.copy(clean_args)
            diagnostic_args.mode = "diagnostic"
            diagnostic_dir = arm_dir / "diagnostic"
            diagnostic_dir.mkdir(parents=True, exist_ok=True)
            diagnostic_report = run(diagnostic_args, diagnostic_dir)
            write_json(diagnostic_report, diagnostic_dir / "summary.json")
            write_markdown(diagnostic_report, diagnostic_dir / "summary.md")
            full_reports[arm] = clean_report
            native = (
                ((diagnostic_report.get("profile") or {}).get("attribution") or {}).get("native")
                or {}
            )
            arms[arm] = {
                "status": combined_status(clean_report, diagnostic_report),
                "persistent_feedback": feedback,
                "copy_patch": copy_patch,
                "clean_summary": rel(clean_dir / "summary.json"),
                "diagnostic_summary": rel(diagnostic_dir / "summary.json"),
                "phrust_identity": ((clean_report.get("engines") or {}).get("phrust") or {}).get(
                    "identity"
                ),
                "compile": {
                    "jit_compile_attempts": native.get("jit_compile_attempts"),
                    "jit_compile_time_nanos": native.get("jit_compile_time_nanos"),
                    "jit_compiled": native.get("jit_compiled"),
                },
                "request_phases_nanos": (diagnostic_report.get("profile") or {}).get(
                    "phases_nanos", {}
                ),
            }
    statuses = {arm["status"] for arm in arms.values()}
    status = "fail" if "fail" in statuses else "skip" if "skip" in statuses else "pass"
    return {
        "schema_version": 1,
        "status": status,
        "mode": "cranelift-ab",
        "timing_eligible": all(
            report.get("timing_eligible") is True for report in full_reports.values()
        ),
        "engine_preset": "experimental-jit",
        "comparisons": {
            f"feedback-{feedback}-copy-patch-off-to-on": build_feedback_ab_ratios(
                full_reports[f"feedback-{feedback}-copy-patch-off"],
                full_reports[f"feedback-{feedback}-copy-patch-on"],
            )
            for feedback in ("off", "on")
        },
        "arms": arms,
        "failures": [],
    }


def combined_status(*reports: dict[str, Any]) -> str:
    statuses = {report.get("status") for report in reports}
    return "fail" if "fail" in statuses else "skip" if "skip" in statuses else "pass"


def build_feedback_ab_ratios(
    off_report: dict[str, Any], on_report: dict[str, Any]
) -> list[dict[str, Any]]:
    off_curves = (((off_report.get("engines") or {}).get("phrust") or {}).get("curves") or [])
    on_curves = (((on_report.get("engines") or {}).get("phrust") or {}).get("curves") or [])
    on_by_concurrency = {curve.get("concurrency"): curve for curve in on_curves}
    comparisons: list[dict[str, Any]] = []
    for off in off_curves:
        concurrency = off.get("concurrency")
        on = on_by_concurrency.get(concurrency)
        if on is None:
            continue
        off_walls = [float(sample["wall_ms"]) for sample in off.get("samples", [])]
        on_walls = [float(sample["wall_ms"]) for sample in on.get("samples", [])]
        comparisons.append(
            {
                "concurrency": concurrency,
                "off_to_on_p50_latency": safe_ratio(
                    (off.get("latency_ms") or {}).get("p50"),
                    (on.get("latency_ms") or {}).get("p50"),
                ),
                "off_to_on_p95_latency": safe_ratio(
                    (off.get("latency_ms") or {}).get("p95"),
                    (on.get("latency_ms") or {}).get("p95"),
                ),
                "off_to_on_p95_latency_ci95": bootstrap_percentile_ratio_ci(
                    off_walls,
                    on_walls,
                    95,
                    seed=int(concurrency or 0) * 131 + len(off_walls),
                ) if off_walls and on_walls else None,
                "on_to_off_requests_per_second": safe_ratio(
                    on.get("requests_per_second"), off.get("requests_per_second")
                ),
            }
        )
    return comparisons


def validate_configuration(args: argparse.Namespace) -> list[str]:
    errors: list[str] = []
    if args.feedback_ab and args.mode != "clean":
        errors.append("--feedback-ab requires --mode clean")
    if args.feedback_ab and (args.baseline or args.compare or args.record_baseline):
        errors.append("--feedback-ab cannot record or compare a regression baseline")
    if args.cranelift_ab and args.mode != "clean":
        errors.append("--cranelift-ab requires --mode clean")
    if args.cranelift_ab and args.feedback_ab:
        errors.append("--cranelift-ab and --feedback-ab are mutually exclusive")
    if args.cranelift_ab and (args.baseline or args.compare or args.record_baseline):
        errors.append("--cranelift-ab cannot record or compare a regression baseline")
    if args.cranelift_ab and platform.machine().lower() not in {"arm64", "aarch64"}:
        errors.append("--cranelift-ab is supported only on ARM/aarch64")
    levels = concurrency_levels(args.concurrency)
    if args.samples < 1:
        errors.append("samples must be positive")
    if args.strict and args.mode == "clean" and args.samples < 30:
        errors.append("strict clean mode requires at least 30 measured requests per concurrency")
    if args.strict and args.mode == "clean" and not args.database_identity:
        errors.append(
            "strict clean mode requires --database-identity for the restored database snapshot"
        )
    if levels and args.samples < max(levels):
        errors.append("samples must be at least the largest concurrency level")
    if args.mode == "clean" and args.strict and args.phrust_url and not args.php_url:
        errors.append("strict URL mode requires --php-url; missing reference PHP-FPM")
    if args.mode == "clean" and args.php_url and args.strict and args.php_version != TARGET_PHP_VERSION:
        errors.append(
            f"strict remote PHP-FPM requires --php-version {TARGET_PHP_VERSION}"
        )
    if args.mode == "clean":
        active_instrumentation = [
            name for name in CLEAN_TIMING_FORBIDDEN_ENV if os.environ.get(name)
        ]
        if active_instrumentation:
            errors.append(
                "clean timing rejects Phrust instrumentation environment: "
                + ", ".join(active_instrumentation)
            )
    if args.compare:
        baseline_path = repo_path(args.compare)
        if baseline_path is None or not baseline_path.is_file():
            errors.append(f"strict regression baseline is missing: {args.compare}")
    try:
        parse_observables(args.observable)
    except ValueError as error:
        errors.append(str(error))
    return errors


def concurrency_levels(value: str) -> list[int]:
    cpus = available_cpus()
    raw = [part.strip() for part in value.split(",") if part.strip()] if value else ["1", str(cpus), str(cpus * 2)]
    try:
        levels = [int(part) for part in raw]
    except ValueError as error:
        raise ValueError("concurrency levels must be positive integers") from error
    if any(level < 1 for level in levels):
        raise ValueError("concurrency levels must be positive integers")
    return list(dict.fromkeys(levels))


def available_cpus() -> int:
    if hasattr(os, "sched_getaffinity"):
        return max(1, len(os.sched_getaffinity(0)))
    return max(1, os.cpu_count() or 1)


def parse_observables(values: list[str]) -> list[tuple[str, str]]:
    if not values:
        return list(DEFAULT_OBSERVABLES)
    parsed = []
    for value in values:
        name, separator, path = value.partition("=")
        if not separator or not name.strip() or not path.startswith("/"):
            raise ValueError(f"invalid observable {value!r}; expected NAME=/path")
        parsed.append((name.strip(), path))
    return parsed


def resolve_phrust(args: argparse.Namespace, out_dir: Path, docroot: Path | None) -> ManagedTarget:
    if args.phrust_url:
        return ManagedTarget(
            HttpTarget("phrust", args.phrust_url.rstrip("/"), args.host_header),
            [],
            {"kind": "remote", "version": "operator supplied"},
        )
    if docroot is None:
        raise EnvironmentError("missing WordPress docroot")
    server = repo_path(args.server)
    if server is None or not server.is_file():
        raise EnvironmentError(f"missing Phrust server binary: {args.server}")
    if args.strict and not is_release_binary(server):
        raise RuntimeError(f"strict clean timing rejects non-release binary: {server}")
    log_path = out_dir / "phrust-server.log"
    log = log_path.open("w+", encoding="utf-8")
    command = [
        str(server),
        "--listen",
        "127.0.0.1:0",
        "--docroot",
        str(docroot),
        "--front-controller",
        "index.php",
        "--deployment-mode",
        "immutable",
        "--engine-preset",
        args.engine_preset,
    ]
    artifacts = {"server_log": rel(log_path)}
    if args.mode == "diagnostic":
        profile_dir = out_dir / "request-profiles"
        trace_path = out_dir / "perf-trace.jsonl"
        profile_dir.mkdir(parents=True, exist_ok=True)
        command.extend(
            [
                "--perf-trace",
                str(trace_path),
                "--perf-trace-vm-counters",
                "--request-profile",
                str(profile_dir),
                "--request-profile-vm-counters",
                "--request-profile-source-attribution",
            ]
        )
        artifacts.update({"request_profiles": rel(profile_dir), "trace": rel(trace_path)})
    startup_started_ns = time.perf_counter_ns()
    process_env = os.environ.copy()
    performance_environment = dict(MANAGED_CLEAN_ENV)
    performance_environment["PHRUST_PERSISTENT_FEEDBACK"] = (
        "1" if args.persistent_feedback == "on" else "0"
    )
    performance_environment["PHRUST_JIT_COPY_PATCH"] = "1" if args.copy_patch == "on" else "0"
    if args.mode == "clean":
        process_env.update(performance_environment)
        process_env.pop("PHRUST_PERF_ABLATION", None)
    process = subprocess.Popen(
        command,
        cwd=REPO_ROOT,
        env=process_env,
        text=True,
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    base_url = wait_for_server(process, log)
    identity = binary_identity(server)
    identity["deployment_mode"] = "immutable"
    identity["engine_preset"] = args.engine_preset
    identity["persistent_feedback"] = args.persistent_feedback
    identity["copy_patch"] = args.copy_patch
    identity["performance_environment"] = (
        performance_environment if args.mode == "clean" else "diagnostic"
    )
    identity["startup_ms"] = (time.perf_counter_ns() - startup_started_ns) / 1_000_000.0
    return ManagedTarget(
        HttpTarget("phrust", base_url, args.host_header, (process.pid,)),
        command,
        identity,
        process=process,
        log=log,
        artifacts=artifacts,
    )


def wait_for_server(process: subprocess.Popen[str], log: Any) -> str:
    for _ in range(300):
        if process.poll() is not None:
            log.flush()
            log.seek(0)
            raise RuntimeError(f"Phrust server exited early:\n{log.read()}")
        log.flush()
        log.seek(0)
        matches = re.findall(r"^listening http://(.+)$", log.read(), flags=re.MULTILINE)
        if matches:
            return f"http://{matches[-1].strip()}"
        time.sleep(0.05)
    raise RuntimeError("Phrust server did not print its listening address")


def resolve_php_fpm(args: argparse.Namespace, out_dir: Path, docroot: Path | None) -> ManagedTarget:
    if args.php_url:
        return ManagedTarget(
            HttpTarget("php-fpm", args.php_url.rstrip("/"), args.host_header),
            [],
            {"kind": "remote", "php_version": args.php_version, "opcache": "operator asserted"},
        )
    if docroot is None:
        raise EnvironmentError("missing WordPress docroot for PHP-FPM")
    if shutil.which("docker") is None:
        raise EnvironmentError("Docker is unavailable and --php-url was not supplied")
    if not docker_image_exists(args.php_fpm_image):
        raise EnvironmentError(
            f"missing reference image {args.php_fpm_image}; run just wordpress-reference-image"
        )
    if not docker_image_exists(args.nginx_image):
        raise EnvironmentError(f"missing nginx image {args.nginx_image}; pull it before the benchmark")
    suffix = f"{os.getpid()}-{int(time.time())}"
    use_host_network = sys.platform.startswith("linux") and not args.docker_network
    network = "host" if use_host_network else (args.docker_network or f"phrust-perf-{suffix}")
    created_network = not use_host_network and not args.docker_network
    fpm_name = f"phrust-fpm-{suffix}"
    nginx_name = f"phrust-nginx-{suffix}"
    cleanup: list[list[str]] = []
    if created_network:
        checked(["docker", "network", "create", network])
        cleanup.append(["docker", "network", "rm", network])
    fpm_port = available_port() if use_host_network else 9000
    nginx_port = available_port() if use_host_network else 80
    fpm_config = out_dir / "reference-php-fpm.conf"
    fpm_listen_host = "127.0.0.1" if use_host_network else "0.0.0.0"
    fpm_config.write_text(
        render_fpm_config(
            fpm_listen_host,
            fpm_port,
            max(concurrency_levels(args.concurrency)),
        ),
        encoding="utf-8",
    )
    nginx_config = out_dir / "reference-nginx.conf"
    upstream = f"127.0.0.1:{fpm_port}" if use_host_network else f"{fpm_name}:9000"
    nginx_config.write_text(render_nginx_config(docroot, upstream, nginx_port), encoding="utf-8")
    mount = f"{docroot}:{docroot}"
    fpm_command = [
        "docker", "run", "--detach", "--rm", "--name", fpm_name,
        "--network", network, "--volume", mount,
        "--volume", f"{fpm_config}:/tmp/phrust-reference-fpm.conf:ro",
        args.php_fpm_image,
        "php-fpm", "-F", "-y", "/tmp/phrust-reference-fpm.conf",
        "-d", "opcache.enable=1", "-d", "opcache.validate_timestamps=1",
        "-d", "opcache.revalidate_freq=0", "-d", "opcache.jit=0",
    ]
    checked(fpm_command)
    cleanup.insert(0, ["docker", "rm", "--force", fpm_name])
    nginx_command = [
        "docker", "run", "--detach", "--rm", "--name", nginx_name,
        "--network", network,
        "--volume", mount,
        "--volume", f"{nginx_config}:/etc/nginx/conf.d/default.conf:ro",
        args.nginx_image,
    ]
    if not use_host_network:
        nginx_command[8:8] = ["--publish", "127.0.0.1::80"]
    checked(nginx_command)
    cleanup.insert(0, ["docker", "rm", "--force", nginx_name])
    if use_host_network:
        base_url = f"http://127.0.0.1:{nginx_port}"
    else:
        port_text = checked(["docker", "port", nginx_name, "80/tcp"]).strip()
        base_url = f"http://127.0.0.1:{port_text.rsplit(':', 1)[-1]}"
    wait_for_http(HttpTarget("php-fpm", base_url, args.host_header), args.timeout_seconds)
    fpm_pid = int(checked(["docker", "inspect", "--format", "{{.State.Pid}}", fpm_name]).strip())
    nginx_pid = int(checked(["docker", "inspect", "--format", "{{.State.Pid}}", nginx_name]).strip())
    identity_text = checked(
        [
            "docker", "exec", fpm_name, "php",
            "-d", "opcache.enable=1",
            "-d", "opcache.validate_timestamps=1",
            "-d", "opcache.revalidate_freq=0",
            "-d", "opcache.jit=0",
            "-r",
            "echo json_encode(['php_version'=>PHP_VERSION,'opcache_version'=>phpversion('Zend OPcache')?:'missing','opcache_enable'=>ini_get('opcache.enable'),'opcache_validate_timestamps'=>ini_get('opcache.validate_timestamps'),'opcache_revalidate_freq'=>ini_get('opcache.revalidate_freq'),'opcache_jit'=>ini_get('opcache.jit')]);",
        ]
    )
    php_configuration = json.loads(identity_text)
    identity = {
        "kind": "docker_php_fpm_nginx",
        "php_version": php_configuration["php_version"],
        "opcache": php_configuration,
        "php_fpm_image": args.php_fpm_image,
        "php_fpm_image_id": checked(["docker", "image", "inspect", "--format", "{{.Id}}", args.php_fpm_image]).strip(),
        "nginx_image": args.nginx_image,
        "network": network,
        "fpm_children": max(concurrency_levels(args.concurrency)),
    }
    if args.strict and identity["php_version"] != TARGET_PHP_VERSION:
        for command in cleanup:
            subprocess.run(command, check=False, capture_output=True, text=True)
        raise RuntimeError(
            f"reference PHP version is {identity['php_version']}; expected {TARGET_PHP_VERSION}"
        )
    return ManagedTarget(
        HttpTarget("php-fpm", base_url, args.host_header, (fpm_pid, nginx_pid)),
        fpm_command + ["&&"] + nginx_command,
        identity,
        cleanup_commands=cleanup,
        artifacts={"nginx_config": rel(nginx_config), "fpm_config": rel(fpm_config)},
    )


def render_nginx_config(docroot: Path, upstream: str, listen_port: int) -> str:
    return f"""server {{
    listen {listen_port};
    server_name _;
    root {docroot};
    index index.php index.html;
    location / {{ try_files $uri $uri/ /index.php?$args; }}
    location ~ \\.php$ {{
        try_files $uri =404;
        include fastcgi_params;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        fastcgi_param DOCUMENT_ROOT $document_root;
        fastcgi_param SERVER_NAME $host;
        fastcgi_pass {upstream};
    }}
}}
"""


def render_fpm_config(listen_host: str, listen_port: int, children: int) -> str:
    return f"""[global]
daemonize = no
error_log = /proc/self/fd/2

[www]
listen = {listen_host}:{listen_port}
user = {os.getuid()}
group = {os.getgid()}
pm = static
pm.max_children = {children}
catch_workers_output = yes
clear_env = no
security.limit_extensions = .php
"""


def available_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def collect_clean(
    args: argparse.Namespace,
    out_dir: Path,
    phrust: ManagedTarget,
    php: ManagedTarget,
    docroot: Path | None,
) -> dict[str, Any]:
    observables = parse_observables(args.observable)
    failures: list[str] = []
    for target in (phrust.target, php.target):
        for _ in range(args.warmups):
            http_get(target, args.path, args.timeout_seconds)
    before = {
        target.name: collect_observables(target, observables, args.timeout_seconds)
        for target in (phrust.target, php.target)
    }
    benchmark_before = {
        target.name: http_get(target, args.path, args.timeout_seconds)
        for target in (phrust.target, php.target)
    }
    failures.extend(compare_observables(before["phrust"], before["php-fpm"]))
    failures.extend(
        compare_observables(
            {"benchmark_path": benchmark_before["phrust"]},
            {"benchmark_path": benchmark_before["php-fpm"]},
        )
    )
    failures.extend(validate_benchmark_path_status(benchmark_before, strict=args.strict))
    if failures:
        return {
            "schema_version": 2,
            "status": "fail",
            "mode": "clean",
            "timing_eligible": False,
            "measurement_model": {
                "warmups_per_engine": args.warmups,
                "samples_per_concurrency": args.samples,
                "concurrency": concurrency_levels(args.concurrency),
                "instrumentation": "external_process_sampling_only",
                "vm_instrumentation": "disabled",
                "latency_percentile": "nearest-rank",
                "process_sampling_hz": 20,
            },
            "environment": environment_identity(docroot, args.database_identity),
            "engines": {
                managed.target.name: {
                    "command": managed.command,
                    "identity": managed.identity,
                    "base_url": managed.target.base_url,
                    "curves": [],
                }
                for managed in (phrust, php)
            },
            "correctness": {
                "before": before,
                "benchmark_before": benchmark_before,
                "after": {},
                "failures": failures,
            },
            "ratios": [],
            "baseline": baseline_summary(load_json(args.baseline)),
            "artifacts": artifact_paths(out_dir, phrust, php),
        }
    engines: dict[str, Any] = {}
    for managed in (phrust, php):
        curves = [
            sample_curve(
                managed.target,
                args.path,
                concurrency,
                args.samples,
                args.timeout_seconds,
            )
            for concurrency in concurrency_levels(args.concurrency)
        ]
        engines[managed.target.name] = {
            "command": managed.command,
            "identity": managed.identity,
            "base_url": managed.target.base_url,
            "curves": curves,
        }
        failures.extend(
            validate_curves(
                managed.target.name,
                curves,
                args.samples,
                benchmark_before[managed.target.name],
            )
        )
    after = {
        target.name: collect_observables(target, observables, args.timeout_seconds)
        for target in (phrust.target, php.target)
    }
    engines["phrust"]["post_run_metrics"] = fetch_metrics(
        phrust.target, args.timeout_seconds, args.metrics_token
    )
    failures.extend(compare_observables(after["phrust"], after["php-fpm"]))
    ratios = build_ratios(engines["phrust"]["curves"], engines["php-fpm"]["curves"])
    baseline = load_json(args.baseline)
    measurement_model = {
        "warmups_per_engine": args.warmups,
        "samples_per_concurrency": args.samples,
        "concurrency": concurrency_levels(args.concurrency),
        "instrumentation": "external_process_sampling_only",
        "vm_instrumentation": "disabled",
        "latency_percentile": "nearest-rank",
        "process_sampling_hz": 20,
    }
    environment = environment_identity(docroot, args.database_identity)
    failures.extend(
        compare_baseline_identity(
            baseline,
            measurement_model,
            environment,
            engines["php-fpm"]["identity"],
        )
    )
    failures.extend(compare_baseline(engines["phrust"]["curves"], baseline, args.max_latency_regression_pct))
    return {
        "schema_version": 2,
        "status": "fail" if failures else "pass",
        "mode": "clean",
        "timing_eligible": True,
        "measurement_model": measurement_model,
        "environment": environment,
        "engines": engines,
        "correctness": {
            "before": before,
            "benchmark_before": benchmark_before,
            "after": after,
            "failures": failures,
        },
        "ratios": ratios,
        "baseline": baseline_summary(baseline),
        "artifacts": artifact_paths(out_dir, phrust, php),
    }


def collect_diagnostics(
    args: argparse.Namespace,
    out_dir: Path,
    phrust: ManagedTarget,
    docroot: Path | None,
) -> dict[str, Any]:
    for _ in range(args.warmups):
        http_get(phrust.target, args.path, args.timeout_seconds)
    sample = http_get(
        phrust.target,
        args.path,
        args.timeout_seconds,
        {"X-Phrust-Request-Profile": "1"},
    )
    metrics = fetch_metrics(phrust.target, args.timeout_seconds, args.metrics_token)
    profiles = sorted((out_dir / "request-profiles").glob("*.json"))
    profile = json.loads(profiles[-1].read_text(encoding="utf-8")) if profiles else {}
    failures = [] if sample["status"] < 500 else [f"diagnostic request returned HTTP {sample['status']}"]
    return {
        "schema_version": 2,
        "status": "fail" if failures else "pass",
        "mode": "diagnostic",
        "timing_eligible": False,
        "warning": "instrumented samples are excluded from clean latency comparisons",
        "environment": environment_identity(docroot, args.database_identity),
        "engine": {"command": phrust.command, "identity": phrust.identity},
        "sample": sample,
        "metrics": metrics,
        "profile": profile,
        "correctness": {"failures": failures},
        "artifacts": artifact_paths(out_dir, phrust),
    }


def collect_observables(
    target: HttpTarget,
    observables: list[tuple[str, str]],
    timeout_seconds: float,
) -> dict[str, Any]:
    return {name: http_get(target, path, timeout_seconds) for name, path in observables}


def compare_observables(left: dict[str, Any], right: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    for name in sorted(set(left) | set(right)):
        if name not in left or name not in right:
            failures.append(f"observable {name!r} is missing from one engine")
            continue
        for field in ("status", "headers", "body_sha256"):
            if left[name].get(field) != right[name].get(field):
                failures.append(f"observable {name!r} {field} differs between Phrust and PHP-FPM")
    return failures


def validate_benchmark_path_status(
    samples: dict[str, dict[str, Any]], *, strict: bool
) -> list[str]:
    if not strict:
        return []
    return [
        f"{name} benchmark path returned HTTP {sample.get('status')}; "
        "strict clean timing requires HTTP 200"
        for name, sample in samples.items()
        if sample.get("status") != 200
    ]


def validate_curves(
    name: str,
    curves: list[dict[str, Any]],
    requested: int,
    expected: dict[str, Any],
) -> list[str]:
    failures: list[str] = []
    for curve in curves:
        concurrency = curve["concurrency"]
        if curve["completed_samples"] != requested:
            failures.append(
                f"{name} concurrency {concurrency} completed {curve['completed_samples']} of {requested} samples"
            )
        failures.extend(f"{name} concurrency {concurrency}: {failure}" for failure in curve["failures"])
        for index, sample in enumerate(curve["samples"]):
            if sample["status"] >= 500:
                failures.append(f"{name} concurrency {concurrency} sample {index} returned HTTP {sample['status']}")
            for field in ("status", "headers", "body_sha256"):
                if sample.get(field) != expected.get(field):
                    failures.append(
                        f"{name} concurrency {concurrency} sample {index} {field} "
                        "differs from the warmed correctness sample"
                    )
    return failures


def build_ratios(phrust_curves: list[dict[str, Any]], php_curves: list[dict[str, Any]]) -> list[dict[str, Any]]:
    php_by_concurrency = {curve["concurrency"]: curve for curve in php_curves}
    ratios = []
    for phrust in phrust_curves:
        php = php_by_concurrency[phrust["concurrency"]]
        phrust_walls = [float(sample["wall_ms"]) for sample in phrust["samples"]]
        php_walls = [float(sample["wall_ms"]) for sample in php["samples"]]
        concurrency = int(phrust["concurrency"])
        ratios.append(
            {
                "concurrency": concurrency,
                "phrust_to_php_p50_latency": safe_ratio(phrust["latency_ms"]["p50"], php["latency_ms"]["p50"]),
                "phrust_to_php_p50_latency_ci95": bootstrap_percentile_ratio_ci(
                    phrust_walls, php_walls, 50, seed=concurrency * 107 + len(phrust_walls)
                ),
                "phrust_to_php_p95_latency": safe_ratio(phrust["latency_ms"]["p95"], php["latency_ms"]["p95"]),
                "phrust_to_php_p95_latency_ci95": bootstrap_percentile_ratio_ci(
                    phrust_walls, php_walls, 95, seed=concurrency * 109 + len(phrust_walls)
                ),
                "phrust_to_php_requests_per_second": safe_ratio(phrust["requests_per_second"], php["requests_per_second"]),
                "phrust_to_php_cpu_seconds": safe_ratio(phrust["process"]["cpu_seconds"], php["process"]["cpu_seconds"]),
                "phrust_to_php_peak_rss": safe_ratio(phrust["process"]["peak_rss_bytes"], php["process"]["peak_rss_bytes"]),
            }
        )
    return ratios


def safe_ratio(left: Any, right: Any) -> float | None:
    if not isinstance(left, (int, float)) or not isinstance(right, (int, float)) or right == 0:
        return None
    return left / right


def compare_baseline(curves: list[dict[str, Any]], baseline: dict[str, Any] | None, threshold: float) -> list[str]:
    if baseline is None:
        return []
    previous_curves = (((baseline.get("engines") or {}).get("phrust") or {}).get("curves") or [])
    previous = {curve.get("concurrency"): curve for curve in previous_curves}
    failures = []
    for curve in curves:
        old = previous.get(curve["concurrency"])
        if not old:
            continue
        current_p95 = curve["latency_ms"]["p95"]
        old_p95 = (old.get("latency_ms") or {}).get("p95")
        if isinstance(current_p95, (int, float)) and isinstance(old_p95, (int, float)) and old_p95 > 0:
            regression = (current_p95 - old_p95) / old_p95 * 100.0
            if regression > threshold:
                failures.append(
                    f"Phrust p95 at concurrency {curve['concurrency']} regressed by {regression:.1f}%"
                )
    return failures


def compare_baseline_identity(
    baseline: dict[str, Any] | None,
    measurement_model: dict[str, Any],
    environment: dict[str, Any],
    php_identity: dict[str, Any],
) -> list[str]:
    if baseline is None:
        return []
    failures: list[str] = []
    if baseline.get("schema_version") != 2 or baseline.get("mode") != "clean":
        failures.append("baseline is not a schema-version-2 clean benchmark")
    previous_model = baseline.get("measurement_model") or {}
    for field in (
        "warmups_per_engine",
        "samples_per_concurrency",
        "concurrency",
        "latency_percentile",
        "process_sampling_hz",
    ):
        if previous_model.get(field) != measurement_model.get(field):
            failures.append(f"baseline measurement model differs for {field}")
    previous_environment = baseline.get("environment") or {}
    for field in ("platform", "available_cpus", "database_identity"):
        if previous_environment.get(field) != environment.get(field):
            failures.append(f"baseline environment differs for {field}")
    previous_wordpress = previous_environment.get("wordpress") or {}
    current_wordpress = environment.get("wordpress") or {}
    for field in ("version", "git_commit", "tree_sha256", "file_count"):
        if previous_wordpress.get(field) != current_wordpress.get(field):
            failures.append(f"baseline WordPress identity differs for {field}")
    previous_php = (((baseline.get("engines") or {}).get("php-fpm") or {}).get("identity") or {})
    for field in ("php_version", "php_fpm_image_id", "opcache"):
        if previous_php.get(field) != php_identity.get(field):
            failures.append(f"baseline PHP-FPM identity differs for {field}")
    return failures


def fetch_metrics(target: HttpTarget, timeout: float, token: str) -> dict[str, float]:
    headers = {"Authorization": f"Bearer {token}"} if token else None
    sample = raw_http_text(target, "/__phrust/metrics", timeout, headers)
    values: dict[str, float] = {}
    if sample["status"] != 200:
        return values
    for line in sample["text"].splitlines():
        if not line or line.startswith("#"):
            continue
        name, _, value = line.partition(" ")
        try:
            values[name] = float(value)
        except ValueError:
            continue
    return values


def raw_http_text(target: HttpTarget, path: str, timeout: float, headers: dict[str, str] | None) -> dict[str, Any]:
    # Reuse the standard library client but retain the body for metrics parsing.
    import http.client
    from urllib.parse import urlparse

    parsed = urlparse(target.base_url)
    connection = http.client.HTTPConnection(parsed.hostname, parsed.port or 80, timeout=timeout)
    request_headers = {"Host": target.host_header or parsed.netloc}
    request_headers.update(headers or {})
    try:
        connection.request("GET", path, headers=request_headers)
        response = connection.getresponse()
        text = response.read().decode("utf-8", errors="replace")
        return {"status": response.status, "text": text}
    finally:
        connection.close()


def wait_for_http(target: HttpTarget, timeout: float) -> None:
    deadline = time.monotonic() + min(timeout, 30.0)
    last_error = "not ready"
    while time.monotonic() < deadline:
        try:
            http_get(target, "/", min(timeout, 2.0))
            return
        except Exception as error:
            last_error = str(error)
            time.sleep(0.1)
    raise RuntimeError(f"reference HTTP server did not become ready: {last_error}")


def is_release_binary(path: Path) -> bool:
    parts = {part.lower() for part in path.resolve().parts}
    return "release" in parts or "profiling" in parts


def binary_identity(path: Path) -> dict[str, Any]:
    return {
        "kind": "local_binary",
        "path": str(path.resolve()),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "bytes": path.stat().st_size,
        "release_path": is_release_binary(path),
    }


def environment_identity(docroot: Path | None, database_identity: str) -> dict[str, Any]:
    return {
        "platform": sys.platform,
        "logical_cpus": os.cpu_count(),
        "available_cpus": available_cpus(),
        "repository_commit": command_output(["git", "rev-parse", "HEAD"], REPO_ROOT),
        "repository_dirty": bool(command_output(["git", "status", "--porcelain"], REPO_ROOT)),
        "wordpress": wordpress_identity(docroot),
        "database_identity": database_identity or None,
    }


def wordpress_identity(docroot: Path | None) -> dict[str, Any]:
    if docroot is None:
        return {
            "docroot": None,
            "version": None,
            "git_commit": None,
            "tree_sha256": None,
            "file_count": None,
        }
    version_file = docroot / "wp-includes/version.php"
    version = None
    if version_file.is_file():
        match = re.search(r"\$wp_version\s*=\s*'([^']+)'", version_file.read_text(encoding="utf-8", errors="replace"))
        version = match.group(1) if match else None
    resolved_docroot = docroot.resolve()
    git_root = command_output(["git", "-C", str(docroot), "rev-parse", "--show-toplevel"])
    git_commit = None
    if git_root is not None and Path(git_root).resolve() == resolved_docroot:
        git_commit = command_output(["git", "-C", str(docroot), "rev-parse", "HEAD"])
    tree_sha256, file_count = directory_identity(resolved_docroot)
    return {
        "docroot": str(resolved_docroot),
        "version": version,
        "git_commit": git_commit,
        "tree_sha256": tree_sha256,
        "file_count": file_count,
    }


def directory_identity(root: Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    file_count = 0
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if ".git" in relative.parts or not path.is_file():
            continue
        relative_bytes = relative.as_posix().encode("utf-8", errors="surrogateescape")
        digest.update(len(relative_bytes).to_bytes(8, "big"))
        digest.update(relative_bytes)
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
        file_count += 1
    return digest.hexdigest(), file_count


def command_output(command: list[str], cwd: Path | None = None) -> str | None:
    completed = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    return completed.stdout.strip() if completed.returncode == 0 else None


def checked(command: list[str]) -> str:
    completed = subprocess.run(command, cwd=REPO_ROOT, text=True, capture_output=True, check=False)
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise RuntimeError(f"command failed ({' '.join(command)}): {detail}")
    return completed.stdout


def docker_image_exists(image: str) -> bool:
    return subprocess.run(
        ["docker", "image", "inspect", image], capture_output=True, text=True, check=False
    ).returncode == 0


def load_json(path_text: str) -> dict[str, Any] | None:
    path = repo_path(path_text) if path_text else None
    if path is None or not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def baseline_summary(baseline: dict[str, Any] | None) -> dict[str, Any]:
    return {"configured": baseline is not None}


def unavailable_report(blockers: list[str], args: argparse.Namespace, out_dir: Path) -> dict[str, Any]:
    return {
        "schema_version": 2,
        "status": "fail" if args.strict else "skip",
        "mode": args.mode,
        "timing_eligible": False,
        "reason": "environment",
        "failures": blockers,
        "artifacts": artifact_paths(out_dir),
    }


def failure_report(failures: list[str], args: argparse.Namespace, out_dir: Path) -> dict[str, Any]:
    return {
        "schema_version": 2,
        "status": "fail",
        "mode": args.mode,
        "timing_eligible": False,
        "failures": failures,
        "artifacts": artifact_paths(out_dir),
    }


def artifact_paths(out_dir: Path, *targets: ManagedTarget) -> dict[str, str]:
    artifacts = {
        "summary_json": rel(out_dir / "summary.json"),
        "summary_markdown": rel(out_dir / "summary.md"),
    }
    for target in targets:
        artifacts.update({f"{target.target.name}_{key}": value for key, value in target.artifacts.items()})
    return artifacts


def write_json(value: dict[str, Any], path: Path) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_markdown(report: dict[str, Any], path: Path) -> None:
    lines = ["# WordPress PHP-FPM Performance Gate", "", f"Status: `{report['status']}`", "", f"Mode: `{report['mode']}`", ""]
    if report.get("mode") == "diagnostic":
        lines.extend(["Instrumented diagnostic data; not eligible for latency comparison.", ""])
    for failure in report.get("failures", []) or (report.get("correctness") or {}).get("failures", []):
        lines.append(f"- {failure}")
    if "engines" in report:
        lines.extend(["## Clean timing", ""])
        for name, engine in report["engines"].items():
            lines.append(f"### {name}")
            lines.append("")
            for curve in engine["curves"]:
                latency = curve["latency_ms"]
                lines.append(
                    f"- concurrency {curve['concurrency']}: {curve['requests_per_second']:.2f} req/s, "
                    f"p50 {latency['p50']:.3f} ms, p95 {latency['p95']:.3f} ms"
                )
            lines.append("")
        lines.extend(["## Phrust / PHP ratios", ""])
        for ratio in report["ratios"]:
            lines.append(
                f"- concurrency {ratio['concurrency']}: p95 latency "
                f"{format_ratio(ratio['phrust_to_php_p95_latency'])} "
                f"(95% bootstrap {format_interval(ratio['phrust_to_php_p95_latency_ci95'])}), throughput "
                f"{format_ratio(ratio['phrust_to_php_requests_per_second'])}"
            )
        lines.append("")
    lines.extend(["## Artifacts", ""])
    for name, value in report.get("artifacts", {}).items():
        lines.append(f"- `{name}`: `{value}`")
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def write_feedback_ab_markdown(report: dict[str, Any], path: Path) -> None:
    lines = [
        "# WordPress persistent-feedback A/B",
        "",
        f"Status: `{report['status']}`",
        "",
        "Ratios above 1.0 favor persistent feedback: off/on latency and on/off throughput.",
        "",
    ]
    for failure in report.get("failures", []):
        lines.append(f"- {failure}")
    for comparison in report.get("comparison", []):
        lines.append(
            f"- concurrency {comparison['concurrency']}: p50 latency "
            f"{format_ratio(comparison['off_to_on_p50_latency'])}, p95 latency "
            f"{format_ratio(comparison['off_to_on_p95_latency'])} "
            f"(95% bootstrap {format_interval(comparison['off_to_on_p95_latency_ci95'])}), "
            f"throughput {format_ratio(comparison['on_to_off_requests_per_second'])}"
        )
    if not report.get("comparison") and not report.get("failures"):
        lines.append("- No comparable clean timing curves were produced.")
    lines.extend(["", "## Arms", ""])
    for arm, details in report.get("arms", {}).items():
        lines.append(
            f"- feedback {arm}: {details['status']} "
            f"(`{details['summary_markdown']}`)"
        )
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def write_cranelift_ab_markdown(report: dict[str, Any], path: Path) -> None:
    lines = [
        "# WordPress ARM Cranelift A/B",
        "",
        f"Status: `{report['status']}`",
        "",
        "Clean latency and instrumented compile evidence are collected in separate runs.",
        "",
    ]
    for failure in report.get("failures", []):
        lines.append(f"- {failure}")
    lines.extend(["", "## Arms", ""])
    for name, arm in report.get("arms", {}).items():
        compile_evidence = arm.get("compile", {})
        lines.append(
            f"- `{name}`: {arm['status']}; JIT compiles "
            f"{compile_evidence.get('jit_compile_attempts', 'unavailable')}, compile ns "
            f"{compile_evidence.get('jit_compile_time_nanos', 'unavailable')}; "
            f"clean `{arm['clean_summary']}`, diagnostic `{arm['diagnostic_summary']}`"
        )
    lines.extend(["", "## Copy-patch comparisons", ""])
    for name, comparisons in report.get("comparisons", {}).items():
        if not comparisons:
            lines.append(f"- `{name}`: no comparable clean timing curves")
            continue
        for comparison in comparisons:
            lines.append(
                f"- `{name}` concurrency {comparison['concurrency']}: p50 off/on "
                f"{format_ratio(comparison['off_to_on_p50_latency'])}, p95 off/on "
                f"{format_ratio(comparison['off_to_on_p95_latency'])}, throughput on/off "
                f"{format_ratio(comparison['on_to_off_requests_per_second'])}"
            )
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def format_ratio(value: float | None) -> str:
    return "unsupported" if value is None else f"{value:.3f}x"


def format_interval(value: list[float] | None) -> str:
    if value is None:
        return "unsupported"
    return f"{value[0]:.3f}x–{value[1]:.3f}x"


def rel(path: Path) -> str:
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def self_test() -> int:
    assert percentile([1.0, 2.0, 3.0, 4.0], 50) == 2.0
    assert percentile(list(map(float, range(1, 101))), 95) == 95.0
    interval = bootstrap_percentile_ci([1.0, 2.0, 3.0, 4.0], 50, seed=7, iterations=100)
    assert interval is not None and interval[0] <= 2.0 <= interval[1]
    assert "fastcgi_param SERVER_NAME $host;" in render_nginx_config(
        Path("/tmp/wordpress"), "127.0.0.1:9000", 8080
    )
    assert "listen = 0.0.0.0:9000" in render_fpm_config("0.0.0.0", 9000, 4)
    assert normalize_headers([("Date", "today"), ("Content-Type", " text/html ")]) == [["content-type", "text/html"]]
    (REPO_ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=REPO_ROOT / "target") as temporary:
        wordpress = Path(temporary)
        (wordpress / "wp-includes").mkdir()
        (wordpress / "wp-includes/version.php").write_text(
            "<?php $wp_version = '8.5-test';\n", encoding="utf-8"
        )
        first_identity = wordpress_identity(wordpress)
        assert first_identity["version"] == "8.5-test"
        assert first_identity["git_commit"] is None
        assert first_identity["file_count"] == 1
        (wordpress / "index.php").write_text("<?php echo 'changed';\n", encoding="utf-8")
        second_identity = wordpress_identity(wordpress)
        assert second_identity["tree_sha256"] != first_identity["tree_sha256"]
        assert second_identity["file_count"] == 2
    mismatch = compare_observables(
        {"root": {"status": 200, "headers": [], "body_sha256": "a"}},
        {"root": {"status": 200, "headers": [], "body_sha256": "b"}},
    )
    assert mismatch == ["observable 'root' body_sha256 differs between Phrust and PHP-FPM"]
    redirect_samples = {
        "phrust": {"status": 301},
        "php-fpm": {"status": 301},
    }
    assert not validate_benchmark_path_status(redirect_samples, strict=False)
    assert validate_benchmark_path_status(redirect_samples, strict=True) == [
        "phrust benchmark path returned HTTP 301; strict clean timing requires HTTP 200",
        "php-fpm benchmark path returned HTTP 301; strict clean timing requires HTTP 200",
    ]
    strict_missing = parse_args(["--strict", "--phrust-url", "http://127.0.0.1:1"])
    assert "missing reference PHP-FPM" in " ".join(validate_configuration(strict_missing))
    insufficient = parse_args(["--strict", "--samples", "29"])
    assert "at least 30" in " ".join(validate_configuration(insufficient))
    assert not is_release_binary(REPO_ROOT / "target/debug/phrust-server")
    assert is_release_binary(REPO_ROOT / "target/release/phrust-server")
    diagnostic = parse_args(["--mode", "diagnostic", "--samples", "1", "--concurrency", "1"])
    assert not validate_configuration(diagnostic)
    invalid_ab = parse_args(["--mode", "diagnostic", "--feedback-ab"])
    assert "requires --mode clean" in " ".join(validate_configuration(invalid_ab))
    cranelift_ab_errors = validate_configuration(parse_args(["--cranelift-ab"]))
    if platform.machine().lower() in {"arm64", "aarch64"}:
        assert "requires an ARM64 host" not in " ".join(cranelift_ab_errors)
    else:
        assert "requires an ARM64 host" in " ".join(cranelift_ab_errors)
    ab_off = {
        "engines": {
            "phrust": {
                "curves": [{
                    "concurrency": 1,
                    "latency_ms": {"p50": 10.0, "p95": 20.0},
                    "requests_per_second": 100.0,
                    "samples": [{"wall_ms": 10.0}, {"wall_ms": 20.0}],
                }]
            }
        }
    }
    ab_on = copy.deepcopy(ab_off)
    ab_on["engines"]["phrust"]["curves"][0].update(
        latency_ms={"p50": 5.0, "p95": 10.0}, requests_per_second=200.0
    )
    ab_on["engines"]["phrust"]["curves"][0]["samples"] = [
        {"wall_ms": 5.0},
        {"wall_ms": 10.0},
    ]
    ab_comparison = build_feedback_ab_ratios(ab_off, ab_on)
    assert ab_comparison[0]["off_to_on_p95_latency"] == 2.0
    assert ab_comparison[0]["on_to_off_requests_per_second"] == 2.0
    previous_trace = os.environ.get("PHRUST_PERF_TRACE")
    os.environ["PHRUST_PERF_TRACE"] = "target/forbidden-clean-trace.jsonl"
    try:
        instrumented_clean = validate_configuration(parse_args(["--samples", "30"]))
        assert "clean timing rejects Phrust instrumentation environment" in " ".join(
            instrumented_clean
        )
    finally:
        if previous_trace is None:
            os.environ.pop("PHRUST_PERF_TRACE", None)
        else:
            os.environ["PHRUST_PERF_TRACE"] = previous_trace
    skipped = unavailable_report(["missing WordPress"], parse_args([]), REPO_ROOT / "target")
    assert skipped["status"] == "skip"
    strict_skip = unavailable_report(["missing WordPress"], parse_args(["--strict"]), REPO_ROOT / "target")
    assert strict_skip["status"] == "fail"
    print("[pass] wordpress_root_benchmark self-test")
    return 0


if __name__ == "__main__":
    sys.exit(main())
