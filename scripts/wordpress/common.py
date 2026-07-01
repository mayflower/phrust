"""Shared helpers for real WordPress smoke tooling."""

from __future__ import annotations

import json
import os
import re
import shutil
import socket
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlparse


REPO_ROOT = Path(__file__).resolve().parents[2]
WORDPRESS_REQUIRED_PATHS = (
    "wp-load.php",
    "wp-settings.php",
    "wp-includes",
    "wp-admin",
    "index.php",
)
DB_PHASES = {"db-install", "admin-login-page", "post-install-frontpage"}


def repo_path(path: str | Path | None) -> Path | None:
    if path is None:
        return None
    text = str(path).strip()
    if not text:
        return None
    candidate = Path(text).expanduser()
    if candidate.is_absolute():
        return candidate
    return REPO_ROOT / candidate


def canonical_path(path: Path) -> Path | None:
    try:
        return path.resolve(strict=True)
    except OSError:
        return None


def json_dump(data: dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def now_run_id(prefix: str) -> str:
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    commit = short_git_commit()
    if commit:
        return f"{prefix}-{timestamp}-{commit}"
    return f"{prefix}-{timestamp}"


def short_git_commit() -> str | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--short=12", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return result.stdout.strip() or None


def executable(path: Path | None) -> bool:
    return path is not None and path.is_file() and os.access(path, os.X_OK)


def binary_is_stale(binary: Path, source_roots: tuple[str, ...] = ("crates", "Cargo.lock", "Cargo.toml")) -> bool:
    if not binary.is_file():
        return True
    binary_mtime = binary.stat().st_mtime
    newest_source = 0.0
    for item in source_roots:
        path = REPO_ROOT / item
        if path.is_file():
            newest_source = max(newest_source, path.stat().st_mtime)
        elif path.is_dir():
            for source in path.rglob("*.rs"):
                try:
                    newest_source = max(newest_source, source.stat().st_mtime)
                except OSError:
                    continue
    return newest_source > binary_mtime


def wordpress_shape_blockers(wordpress_dir: Path | None) -> list[str]:
    if wordpress_dir is None:
        return ["missing_wordpress_checkout"]
    canonical = canonical_path(wordpress_dir)
    if canonical is None or not canonical.is_dir():
        return ["missing_wordpress_checkout"]
    missing = [name for name in WORDPRESS_REQUIRED_PATHS if not (canonical / name).exists()]
    return ["missing_wordpress_checkout"] if missing else []


def is_port_available(host: str, port: int) -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind((host, port))
        except OSError:
            return False
    return True


def docker_available() -> bool:
    if shutil.which("docker") is None:
        return False
    try:
        subprocess.run(
            ["docker", "info", "--format", "{{.ServerVersion}}"],
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=5,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return False
    return True


def parse_mysql_dsn(dsn: str) -> dict[str, Any]:
    parsed = urlparse(dsn)
    return {
        "scheme": parsed.scheme,
        "host": parsed.hostname or "127.0.0.1",
        "port": parsed.port or 3306,
        "user": parsed.username or "",
        "password": parsed.password or "",
        "database": parsed.path.lstrip("/"),
    }


def tcp_reachable(host: str, port: int, timeout_seconds: float = 3.0) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout_seconds):
            return True
    except OSError:
        return False


def mysql_credentials_valid(dsn: str, timeout_seconds: int = 5) -> bool | None:
    mysql = shutil.which("mysql")
    if mysql is None:
        return None
    parsed = parse_mysql_dsn(dsn)
    command = [
        mysql,
        "--batch",
        "--skip-column-names",
        f"--host={parsed['host']}",
        f"--port={parsed['port']}",
        f"--user={parsed['user']}",
        f"--database={parsed['database']}",
        "--execute=SELECT 1",
    ]
    if parsed["password"]:
        command.insert(-1, f"--password={parsed['password']}")
    try:
        subprocess.run(
            command,
            text=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=timeout_seconds,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired):
        return False
    return True


def excerpt(text: str, limit: int = 2000) -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + "\n[truncated]"


def parse_json_lines(text: str) -> list[dict[str, Any]]:
    items: list[dict[str, Any]] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        candidates = []
        if stripped.startswith("{"):
            candidates.append(stripped)
        match = re.search(r"diagnostics=(\{.*\})", stripped)
        if match:
            candidates.append(match.group(1))
        for candidate in candidates:
            try:
                value = json.loads(candidate)
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                items.append(value)
    return items


def diagnostic_id(value: dict[str, Any]) -> str | None:
    candidate = value.get("id")
    if isinstance(candidate, str) and candidate.startswith(("E_", "W_", "D_")):
        return candidate
    diagnostic = value.get("diagnostic")
    if isinstance(diagnostic, dict):
        nested = diagnostic.get("id")
        if isinstance(nested, str) and nested.startswith(("E_", "W_", "D_")):
            return nested
    return None


def extract_diagnostics(*texts: str) -> list[dict[str, Any]]:
    diagnostics: list[dict[str, Any]] = []
    seen: set[tuple[str | None, str | None]] = set()
    for text in texts:
        for item in parse_json_lines(text):
            diag = item.get("diagnostic") if isinstance(item.get("diagnostic"), dict) else item
            if not isinstance(diag, dict):
                continue
            ident = diagnostic_id(diag)
            if ident is None:
                continue
            message = diag.get("message")
            key = (ident, message if isinstance(message, str) else None)
            if key in seen:
                continue
            seen.add(key)
            diagnostics.append(diag)
    return diagnostics


def span_source_path(diagnostic: dict[str, Any]) -> str | None:
    span = diagnostic.get("span")
    if isinstance(span, dict):
        file_name = span.get("file")
        if isinstance(file_name, str) and file_name:
            return file_name
    return None


def span_start(diagnostic: dict[str, Any]) -> int | None:
    span = diagnostic.get("span")
    if isinstance(span, dict) and isinstance(span.get("start"), int):
        return span["start"]
    return None


def line_for_byte_offset(path: str | None, offset: int | None) -> int | None:
    if path is None or offset is None:
        return None
    try:
        data = Path(path).read_bytes()
    except OSError:
        return None
    offset = max(0, min(offset, len(data)))
    return data[:offset].count(b"\n") + 1


def runtime_stack(diagnostic: dict[str, Any]) -> list[dict[str, Any]]:
    stack = diagnostic.get("stack")
    if isinstance(stack, list):
        return [item for item in stack if isinstance(item, dict)]
    return []


def classify_failure(diagnostics: list[dict[str, Any]], text: str = "", timed_out: bool = False) -> tuple[str, str]:
    if timed_out:
        return ("timeout", "php_executor")
    joined = " ".join(
        str(part)
        for diagnostic in diagnostics
        for part in (diagnostic.get("id"), diagnostic.get("message"))
        if part is not None
    )
    joined = f"{joined} {text}".lower()
    if "mysql" in joined or "mysqli" in joined or "database" in joined or "dsn" in joined:
        return ("database", "php_runtime")
    if "undefined function" in joined or "unknown function" in joined or "stdlib" in joined:
        return ("stdlib", "php_std")
    if "server" in joined or "http" in joined or "request" in joined or "_server" in joined:
        return ("web", "php_server")
    if "diagnostic" in joined or "error handler" in joined:
        return ("diagnostics", "php_runtime")
    if "compile" in joined or "frontend" in joined or "lower" in joined or "ir_" in joined:
        return ("runtime", "php_vm")
    return ("runtime", "php_vm")


def environment_failure(blockers: list[str], inputs: dict[str, Any]) -> dict[str, Any]:
    return {
        "request": None,
        "exit_code": None,
        "http_status": None,
        "diagnostic_ids": blockers,
        "source_path": None,
        "line": None,
        "include_stack": [],
        "autoload_stack": [],
        "runtime_stack": [],
        "stdout_excerpt": "",
        "stderr_excerpt": "environment blockers: " + ", ".join(blockers),
        "candidate_owner_layer": "scripts/wordpress",
        "environment_blockers": blockers,
        "inputs": inputs,
    }


def owner_suggestion(first_failure_class: str, diagnostic_id_value: str | None) -> str:
    if first_failure_class == "database":
        return "tests/phpt/generated/wp.db-network or mysqli/curl/openssl modules"
    if first_failure_class == "stdlib":
        return "tests/phpt/generated/wp.core-builtins or owning stdlib module"
    if first_failure_class == "web":
        return "tests/phpt/generated/wp.web-runtime or php_server tests"
    if first_failure_class == "diagnostics":
        return "scripts/wordpress or diagnostics docs/tests"
    if diagnostic_id_value and ("AUTOLOAD" in diagnostic_id_value or "CALL" in diagnostic_id_value):
        return "fixtures/runtime_semantics/wp_language_vm or include_eval_autoload"
    if first_failure_class == "runtime":
        return "fixtures/runtime_semantics/wp_language_vm or include_eval_autoload"
    return "scripts/wordpress or diagnostics docs/tests"
