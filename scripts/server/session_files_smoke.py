#!/usr/bin/env python3
"""Pinned-PHP and cross-process files-session smoke for Welle 3."""

from __future__ import annotations

import concurrent.futures
import http.client
import os
import socket
import subprocess
import tempfile
import time
from contextlib import contextmanager
from pathlib import Path
from typing import NoReturn


ROOT = Path(__file__).resolve().parents[2]
REFERENCE = Path(os.environ.get("REFERENCE_PHP", ROOT / "third_party/php-src/sapi/cli/php")).resolve()
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug/phrust-server"


def fail(message: str) -> NoReturn:
    raise SystemExit(f"[fail] {message}")


def verify_reference() -> None:
    if not REFERENCE.is_file() or not os.access(REFERENCE, os.X_OK):
        fail(f"pinned reference PHP is not executable: {REFERENCE}")
    probe = subprocess.run(
        [str(REFERENCE), "-n", "-r", 'echo PHP_VERSION, "\\n", extension_loaded("session") ? "yes" : "no";'],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if probe != "8.5.7\nyes":
        fail(f"reference must be PHP 8.5.7 with session, got {probe!r}")


def free_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def wait_ready(port: int, process: subprocess.Popen[bytes]) -> None:
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        if process.poll() is not None:
            fail(f"server exited during startup with status {process.returncode}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.1):
                return
        except OSError:
            time.sleep(0.02)
    fail(f"server did not listen on 127.0.0.1:{port}")


@contextmanager
def server(command: list[str], port: int):
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    try:
        wait_ready(port, process)
        yield
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)
        if process.returncode not in (0, -15):
            output = process.stdout.read().decode(errors="replace") if process.stdout else ""
            fail(f"server exited with status {process.returncode}:\n{output}")


def target_command(port: int, docroot: Path, sessions: Path) -> list[str]:
    return [
        str(TARGET),
        "--listen",
        f"127.0.0.1:{port}",
        "--docroot",
        str(docroot),
        "--session-save-path",
        str(sessions),
        "--cpu-execution-limit",
        "2",
    ]


def reference_command(port: int, docroot: Path, sessions: Path) -> list[str]:
    return [
        str(REFERENCE),
        "-n",
        "-d",
        f"session.save_path={sessions}",
        "-S",
        f"127.0.0.1:{port}",
        "-t",
        str(docroot),
    ]


def get(port: int, path: str, session_id: str) -> tuple[int, bytes, list[str]]:
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=8)
    connection.request("GET", path, headers={"Cookie": f"PHPSESSID={session_id}"})
    response = connection.getresponse()
    result = (response.status, response.read(), [value for name, value in response.getheaders() if name.lower() == "set-cookie"])
    connection.close()
    return result


def assert_ok(result: tuple[int, bytes, list[str]], expected: bytes, label: str) -> None:
    if result[0] != 200 or result[1] != expected:
        fail(f"{label}: got status/body {result[0]} {result[1]!r}, expected 200 {expected!r}")


def write_fixtures(docroot: Path) -> None:
    fixtures = {
        "write.php": "<?php ini_set('session.serialize_handler',$_GET['h']); session_start(); $_SESSION['n']=(int)$_GET['n']; session_write_close(); echo \"ok\\n\";",
        "read.php": "<?php ini_set('session.serialize_handler',$_GET['h']); session_start(['read_and_close'=>true]); echo $_SESSION['n'],\"\\n\";",
        "increment.php": "<?php session_start(); $n=$_SESSION['n']; usleep(150000); $_SESSION['n']=$n+1; session_write_close(); echo \"ok\\n\";",
        "sleep.php": "<?php session_start(); usleep(350000); session_write_close(); echo \"ok\\n\";",
        "read_close.php": "<?php session_start(['read_and_close'=>true]); $_SESSION['n']=99; echo \"ok\\n\";",
        "abort.php": "<?php session_start(); $_SESSION['n']=88; session_abort(); echo \"ok\\n\";",
        "destroy.php": "<?php session_start(); var_dump(session_destroy());",
    }
    for name, source in fixtures.items():
        (docroot / name).write_text(source, encoding="utf-8")


def main() -> None:
    verify_reference()
    if not TARGET.is_file() or not os.access(TARGET, os.X_OK):
        fail(f"phrust-server is not executable: {TARGET}")
    with tempfile.TemporaryDirectory(prefix="phrust-session-diff-") as temporary:
        docroot = Path(temporary) / "public"
        sessions = Path(temporary) / "sessions"
        docroot.mkdir()
        sessions.mkdir(mode=0o700)
        write_fixtures(docroot)

        # Phrust writes each codec; the pinned reference reads it.
        for index, handler in enumerate(("php", "php_binary", "php_serialize")):
            session_id = f"phrust-to-ref-{index}"
            port = free_port()
            with server(target_command(port, docroot, sessions), port):
                assert_ok(get(port, f"/write.php?h={handler}&n=7", session_id), b"ok\n", f"Phrust write {handler}")
            port = free_port()
            with server(reference_command(port, docroot, sessions), port):
                assert_ok(get(port, f"/read.php?h={handler}", session_id), b"7\n", f"reference read {handler}")

        # The pinned reference writes each codec; Phrust reads it.
        for index, handler in enumerate(("php", "php_binary", "php_serialize")):
            session_id = f"ref-to-phrust-{index}"
            port = free_port()
            with server(reference_command(port, docroot, sessions), port):
                assert_ok(get(port, f"/write.php?h={handler}&n=9", session_id), b"ok\n", f"reference write {handler}")
            port = free_port()
            with server(target_command(port, docroot, sessions), port):
                assert_ok(get(port, f"/read.php?h={handler}", session_id), b"9\n", f"Phrust read {handler}")

        # Two independent Phrust processes must share the OS file lock.
        first_port, second_port = free_port(), free_port()
        with server(target_command(first_port, docroot, sessions), first_port), server(
            target_command(second_port, docroot, sessions), second_port
        ):
            shared_id = "two-phrust-processes"
            assert_ok(get(first_port, "/write.php?h=php&n=0", shared_id), b"ok\n", "seed shared session")
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                futures = [
                    executor.submit(get, first_port, "/increment.php", shared_id),
                    executor.submit(get, second_port, "/increment.php", shared_id),
                ]
                for future in futures:
                    assert_ok(future.result(), b"ok\n", "cross-process increment")
            assert_ok(get(first_port, "/read.php?h=php", shared_id), b"2\n", "cross-process no-lost-update")

            # read_and_close and abort both release without writing.
            assert_ok(get(first_port, "/read_close.php", shared_id), b"ok\n", "read_and_close")
            assert_ok(get(second_port, "/read.php?h=php", shared_id), b"2\n", "read_and_close persisted state")
            assert_ok(get(first_port, "/abort.php", shared_id), b"ok\n", "abort")
            assert_ok(get(second_port, "/read.php?h=php", shared_id), b"2\n", "abort persisted state")

            # Different IDs must not share a global lock.
            started = time.monotonic()
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                results = list(executor.map(lambda item: get(item[0], "/sleep.php", item[1]), [
                    (first_port, "parallel-a"),
                    (second_port, "parallel-b"),
                ]))
            for result in results:
                assert_ok(result, b"ok\n", "different-id parallel request")
            if time.monotonic() - started >= 0.65:
                fail("different session IDs were serialized across Phrust processes")

            assert_ok(get(first_port, "/destroy.php", shared_id), b"bool(true)\n", "destroy")
            if (sessions / f"sess_{shared_id}").exists():
                fail("session_destroy() left the session file behind")

        for path in sessions.glob("sess_*"):
            if path.stat().st_mode & 0o777 != 0o600:
                fail(f"session file is not mode 0600: {path}")
    print(f"[ok] files-session differential/cross-process smoke passed against {REFERENCE} (PHP 8.5.7)")


if __name__ == "__main__":
    main()
