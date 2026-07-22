#!/usr/bin/env python3
"""Pinned-PHP HTTP differential smoke for Welle-3 request input."""

from __future__ import annotations

import http.client
import os
import socket
import subprocess
import tempfile
import time
from contextlib import contextmanager
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
REFERENCE = Path(os.environ.get("REFERENCE_PHP", ROOT / "third_party/php-src/sapi/cli/php")).resolve()
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug/phrust-server"


def fail(message: str) -> "NoReturn":
    raise SystemExit(f"[fail] {message}")


def verify_reference() -> None:
    if not REFERENCE.is_file() or not os.access(REFERENCE, os.X_OK):
        fail(f"pinned reference PHP is not executable: {REFERENCE}")
    probe = subprocess.run(
        [
            str(REFERENCE),
            "-n",
            "-r",
            'echo PHP_VERSION, "\\n", function_exists("request_parse_body") ? "yes" : "no";',
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if probe != "8.5.7\nyes":
        fail(f"reference must be PHP 8.5.7 with request_parse_body(), got {probe!r}")


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


def request(port: int, method: str, path: str, body: bytes, content_type: str) -> tuple[int, bytes]:
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=8)
    connection.request(method, path, body=body, headers={"Content-Type": content_type})
    response = connection.getresponse()
    result = (response.status, response.read())
    connection.close()
    return result


def exercise(port: int) -> dict[str, tuple[int, bytes]]:
    multipart = (
        b"--BOUNDARY\r\n"
        b'Content-Disposition: form-data; name="title"\r\n\r\nHello\r\n'
        b"--BOUNDARY\r\n"
        b'Content-Disposition: form-data; name="nested[key]"\r\n\r\ninside\r\n'
        b"--BOUNDARY\r\n"
        b'Content-Disposition: form-data; name="avatar"; filename="dir/fallback.txt"; '
        b"filename*=UTF-8''caf%C3%A9.txt\r\n"
        b"Content-Type: application/octet-stream\r\n\r\nUPLOAD\x00DATA\r\n"
        b"--BOUNDARY--\r\n"
    )
    return {
        "raw-file": request(port, "POST", "/raw.php", b"x" * (300 * 1024), "application/octet-stream"),
        "urlencoded-binary": request(
            port,
            "POST",
            "/form.php",
            b"value=A%00B%ff&nested%5Bkey%5D=inside&list%5B%5D=one&list%5B%5D=two",
            "application/x-www-form-urlencoded",
        ),
        "automatic-multipart": request(
            port, "POST", "/multipart.php", multipart, "multipart/form-data; boundary=BOUNDARY"
        ),
        "parse-urlencoded": request(
            port,
            "PATCH",
            "/parse_form.php",
            b"value=A%00B%ff&nested%5Bkey%5D=inside",
            "application/x-www-form-urlencoded",
        ),
        "parse-multipart": request(
            port, "PUT", "/parse_multipart.php", multipart, "multipart/form-data; boundary=BOUNDARY"
        ),
        "parse-then-input": request(
            port, "DELETE", "/interaction.php", b"a=1&b=2", "application/x-www-form-urlencoded"
        ),
        "invalid-content-type": request(port, "PUT", "/invalid.php", b"abc", "text/plain"),
        "options-and-repeat": request(
            port, "PATCH", "/options.php", b"a=1", "application/x-www-form-urlencoded"
        ),
    }


def exercise_disabled(port: int) -> dict[str, tuple[int, bytes]]:
    body = b"a=1&b=2"
    content_type = "application/x-www-form-urlencoded"
    return {
        "disabled-raw": request(port, "POST", "/disabled_input.php", body, content_type),
        "disabled-explicit-parse": request(port, "POST", "/disabled_parse.php", body, content_type),
    }


def write_fixtures(docroot: Path) -> None:
    fixtures = {
        "raw.php": "<?php $a=file_get_contents('php://input'); $b=file_get_contents('php://input'); echo strlen($a),'|',strlen($b),'\\n';",
        "form.php": "<?php echo strlen($_POST['value']),'|',ord($_POST['value'][1]),'|',ord($_POST['value'][3]),'|',$_POST['nested']['key'],'|',implode(',',$_POST['list']),'\\n';",
        "multipart.php": "<?php echo $_POST['title'],'|',$_POST['nested']['key'],'|',$_FILES['avatar']['name'],'|',$_FILES['avatar']['full_path'],'|',$_FILES['avatar']['type'],'|',$_FILES['avatar']['error'],'|',$_FILES['avatar']['size'],'|',strlen(file_get_contents($_FILES['avatar']['tmp_name'])),'\\n';",
        "parse_form.php": "<?php [$p,$f]=request_parse_body(); echo strlen($p['value']),'|',ord($p['value'][1]),'|',ord($p['value'][3]),'|',$p['nested']['key'],'|',count($_POST),'\\n';",
        "parse_multipart.php": "<?php [$p,$f]=request_parse_body(); echo $p['title'],'|',$f['avatar']['name'],'|',$f['avatar']['full_path'],'|',$f['avatar']['error'],'|',$f['avatar']['size'],'|',strlen(file_get_contents($f['avatar']['tmp_name'])),'|',count($_FILES),'\\n';",
        "interaction.php": "<?php [$p]=request_parse_body(); echo $p['a'],'|',file_get_contents('php://input'),'\\n';",
        "invalid.php": "<?php try { request_parse_body(); } catch (RequestParseBodyException $e) { echo get_class($e),'\\n'; }",
        "options.php": "<?php try { request_parse_body(['unknown'=>1]); } catch (ValueError $e) { echo 'value-error|'; } [$p]=request_parse_body(['max_input_vars'=>2]); echo $p['a'],'|'; try { $again=request_parse_body(); echo 'returned-',gettype($again),'\\n'; } catch (Throwable $e) { echo 'threw-',get_class($e),'\\n'; }",
        "disabled_input.php": "<?php echo count($_POST),'|',file_get_contents('php://input'),'\\n';",
        "disabled_parse.php": "<?php [$p]=request_parse_body(); echo count($_POST),'|',$p['a'],'|',file_get_contents('php://input'),'\\n';",
    }
    for name, source in fixtures.items():
        (docroot / name).write_text(source, encoding="utf-8")


def main() -> None:
    verify_reference()
    if not TARGET.is_file() or not os.access(TARGET, os.X_OK):
        fail(f"phrust-server is not executable: {TARGET}")
    with tempfile.TemporaryDirectory(prefix="phrust-request-diff-") as temporary:
        docroot = Path(temporary) / "public"
        spool = Path(temporary) / "spool"
        uploads = Path(temporary) / "uploads"
        docroot.mkdir()
        spool.mkdir()
        uploads.mkdir()
        write_fixtures(docroot)

        reference_port = free_port()
        reference_command = [str(REFERENCE), "-n", "-S", f"127.0.0.1:{reference_port}", "-t", str(docroot)]
        with server(reference_command, reference_port):
            expected = exercise(reference_port)

        target_port = free_port()
        target_command = [
            str(TARGET),
            "--listen",
            f"127.0.0.1:{target_port}",
            "--docroot",
            str(docroot),
            "--request-body-memory-bytes",
            "128",
            "--request-body-temp-dir",
            str(spool),
            "--upload-temp-dir",
            str(uploads),
        ]
        with server(target_command, target_port):
            actual = exercise(target_port)

        for name, expected_result in expected.items():
            if actual[name] != expected_result:
                fail(f"{name}: reference={expected_result!r}, phrust={actual[name]!r}")

        reference_port = free_port()
        reference_command = [
            str(REFERENCE),
            "-n",
            "-d",
            "enable_post_data_reading=0",
            "-S",
            f"127.0.0.1:{reference_port}",
            "-t",
            str(docroot),
        ]
        with server(reference_command, reference_port):
            expected_disabled = exercise_disabled(reference_port)

        target_port = free_port()
        target_command = [
            str(TARGET),
            "--listen",
            f"127.0.0.1:{target_port}",
            "--docroot",
            str(docroot),
            "--disable-post-data-reading",
            "--request-body-temp-dir",
            str(spool),
            "--upload-temp-dir",
            str(uploads),
        ]
        with server(target_command, target_port):
            actual_disabled = exercise_disabled(target_port)
        for name, expected_result in expected_disabled.items():
            if actual_disabled[name] != expected_result:
                fail(f"{name}: reference={expected_result!r}, phrust={actual_disabled[name]!r}")
        if any(spool.iterdir()) or any(uploads.iterdir()):
            fail("request/upload temp directories were not empty after request completion")
    print(f"[ok] request-input differential passed against {REFERENCE} (PHP 8.5.7)")


if __name__ == "__main__":
    main()
