#!/usr/bin/env bash
set -euo pipefail

command_name="${1:-phase9-command}"
shift || true

printf '[todo] %s is reserved for a later Phase 9 prompt.\n' "$command_name" >&2
if (($# > 0)); then
  printf '[todo] received arguments: %s\n' "$*" >&2
fi
printf '%s\n' '[todo] Implement the prompt-specific tool before using this gate as evidence.' >&2
exit 1
