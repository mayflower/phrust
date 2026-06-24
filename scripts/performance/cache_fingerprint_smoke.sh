#!/usr/bin/env bash
set -euo pipefail

cargo test -p php_bytecode_cache fingerprint
printf '%s\n' '[pass] cache fingerprint smoke'
