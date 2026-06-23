#!/usr/bin/env python3
"""Normalize process output embedded in Phase 7 benchmark reports."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def normalize(text: str) -> str:
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = text.replace(str(ROOT), "$ROOT")
    text = re.sub(r"/private/var/folders/[^\s:'\"]+", "$TMP", text)
    text = re.sub(r"/var/folders/[^\s:'\"]+", "$TMP", text)
    text = re.sub(r"/tmp/[^\s:'\"]+", "$TMP", text)
    text = re.sub(r"target/phase7/tmp/[^\s:'\"]+", "target/phase7/tmp/$RUN", text)
    text = re.sub(r"0x[0-9a-fA-F]+", "0x$ADDR", text)
    return text


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", nargs="?")
    args = parser.parse_args()

    if args.path:
        text = Path(args.path).read_text(encoding="utf-8", errors="replace")
    else:
        text = sys.stdin.read()
    sys.stdout.write(normalize(text))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
