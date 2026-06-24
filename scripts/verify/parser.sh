#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

just verify-foundation
just verify-lexer
just fmt
just lint
just test
just parser-diff
just cst-roundtrip

printf '%s\n' '[pass] parser verification complete'
