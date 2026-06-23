#!/usr/bin/env python3
"""Emit machine-readable Cranelift host support status."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

SUPPORTED_HOST_TRIPLES = {
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
}


def rust_host_triple() -> str:
    completed = subprocess.run(
        ["rustc", "-vV"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    for line in completed.stdout.splitlines():
        if line.startswith("host:"):
            return line.split(":", 1)[1].strip()
    raise RuntimeError("rustc -vV did not report a host triple")


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out",
        default="target/phase7/cranelift/platform.json",
        help="machine-readable platform status path",
    )
    args = parser.parse_args()

    out = Path(args.out)
    try:
        host = rust_host_triple()
    except Exception as error:  # noqa: BLE001 - this is a probe with JSON output.
        payload = {
            "schema_version": 1,
            "status": "skip",
            "reason": f"unable to probe rust host triple: {error}",
            "host_triple": None,
            "supported_host_triples": sorted(SUPPORTED_HOST_TRIPLES),
        }
        write_json(out, payload)
        print(f"[skip] Cranelift platform probe unavailable; wrote {out}")
        return 77

    if host not in SUPPORTED_HOST_TRIPLES:
        payload = {
            "schema_version": 1,
            "status": "skip",
            "reason": "host triple is not in the Phase 7 Cranelift support allow-list",
            "host_triple": host,
            "supported_host_triples": sorted(SUPPORTED_HOST_TRIPLES),
        }
        write_json(out, payload)
        print(f"[skip] Cranelift unsupported on {host}; wrote {out}")
        return 77

    payload = {
        "schema_version": 1,
        "status": "pass",
        "reason": "host triple is supported for Phase 7 Cranelift smoke gates",
        "host_triple": host,
        "supported_host_triples": sorted(SUPPORTED_HOST_TRIPLES),
    }
    write_json(out, payload)
    print(f"[pass] Cranelift supported on {host}; wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
