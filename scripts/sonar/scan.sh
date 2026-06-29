#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

host="${SONAR_HOST_URL:-${SONARQUBE_URL:-}}"
token="${SONAR_TOKEN:-${SONARQUBE_TOKEN:-}}"

if [[ -z "$host" ]]; then
  printf '%s\n' '[fail] SONAR_HOST_URL or SONARQUBE_URL must be set.' >&2
  exit 1
fi

if [[ -z "$token" ]]; then
  printf '%s\n' '[fail] SONAR_TOKEN or SONARQUBE_TOKEN must be set.' >&2
  exit 1
fi

if ! command -v sonar-scanner >/dev/null 2>&1; then
  printf '%s\n' '[fail] sonar-scanner is unavailable; run through nix develop.' >&2
  exit 1
fi

scanner_args=("-Dsonar.host.url=$host")

if [[ "${PHRUST_SONAR_COVERAGE:-1}" == "1" ]]; then
  if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov --version >/dev/null 2>&1; then
    printf '%s\n' '[fail] cargo-llvm-cov is unavailable; run through nix develop or set PHRUST_SONAR_COVERAGE=0.' >&2
    exit 1
  fi

  mkdir -p target/sonar
  cargo llvm-cov --workspace --lcov --output-path target/sonar/lcov.info
else
  printf '%s\n' '[skip] PHRUST_SONAR_COVERAGE=0; skipping LCOV generation.'
  scanner_args+=("-Dsonar.rust.lcov.reportPaths=")
fi

export SONAR_TOKEN="$token"
sonar-scanner "${scanner_args[@]}" "$@"
