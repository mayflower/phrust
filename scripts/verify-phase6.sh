#!/usr/bin/env bash
set -euo pipefail

printf '%s\n' '[info] Phase 6 verification starts from the Phase 5 baseline.'

just test-phase6
scripts/phase6_regression_smoke.sh
just coverage-phase6
if test -d target/phase6/diff-stdlib-test; then
  rm -rf target/phase6/diff-stdlib
  cp -R target/phase6/diff-stdlib-test target/phase6/diff-stdlib
  printf '%s\n' '[info] reused test-phase6 stdlib differential report'
else
  just diff-stdlib
fi
just diff-streams
just diff-json-pcre-date
just diff-spl-reflection
just composer-smoke

printf '%s\n' '[pass] phase6 verification complete'
