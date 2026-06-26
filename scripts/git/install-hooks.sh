#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

git config core.hooksPath .githooks
printf '%s\n' '[hooks] installed versioned git hooks from .githooks'
printf '%s\n' '[hooks] pre-commit: fmt, lint, PHPT gate'
printf '%s\n' '[hooks] pre-push: local CI parity gate'
