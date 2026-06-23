#!/usr/bin/env bash
set -euo pipefail

scripts/phase6_preflight.py --out target/phase6/preflight.json >/dev/null

for target in verify-phase0 verify-phase1 verify-phase2 verify-phase3 verify-phase4 verify-phase5; do
  grep -q "\"${target}\": true" target/phase6/preflight.json
done

for script in \
  scripts/verify-phase0.sh \
  scripts/verify-phase1.sh \
  scripts/verify-phase2.sh \
  scripts/verify-phase3.sh \
  scripts/verify-phase4.sh \
  scripts/verify-phase5.sh
do
  test -x "$script"
done

test -f docs/phase-0/final-audit.md
test -f docs/phase-1/final-audit.md
test -f docs/phase-2/final-audit.md
test -f docs/phase-3/phase-3-final-audit.md
test -f docs/phase4-known-gaps.md
test -f docs/phase5-final-audit.md
test -f docs/phase6-handoff.md

printf '%s\n' '[pass] phase6 historical regression smoke complete'
