#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT}"

printf '%s\n' '[info] standard library verification starts from the runtime semantics baseline.'

just stdlib-docs
scripts/performance_regression_smoke.sh
just stdlib-coverage
if test -d target/stdlib/diff-stdlib-test; then
  rm -rf target/stdlib/diff-stdlib
  cp -R target/stdlib/diff-stdlib-test target/stdlib/diff-stdlib
  printf '%s\n' '[info] reused stdlib differential report'
else
  just diff-stdlib
fi
just diff-streams
just diff-json-pcre-date
just diff-spl-reflection
just composer-smoke

printf '%s\n' '[pass] standard library verification complete'
