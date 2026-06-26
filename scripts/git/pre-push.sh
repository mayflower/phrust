#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ "${PHRUST_SKIP_GIT_HOOKS:-0}" == "1" ]]; then
  printf '%s\n' '[pre-push] skipped via PHRUST_SKIP_GIT_HOOKS=1' >&2
  exit 0
fi

if ! command -v nix >/dev/null 2>&1; then
  printf '%s\n' '[pre-push] nix is required; install Nix or push from a configured development host' >&2
  exit 1
fi

printf '%s\n' '[pre-push] running local CI parity gate'
nix develop -c just ci-local
printf '%s\n' '[pre-push] ok'
