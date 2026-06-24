#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "PHASE9.md"
  "docs/phase9/README.md"
  "docs/phase9/source-integrity.md"
  "docs/phase9/source-lookup.md"
  "docs/phase9/binary-discovery.md"
  "docs/phase9/official-runner.md"
  "docs/phase9/full-phpt-gate.md"
)

required_dirs=(
  "docs/phase9/modules"
  "docs/phase9/php-src-behavior"
  "docs/phase9/reports"
  "tests/phpt/generated"
  "tests/phpt/manifests"
)

for path in "${required_files[@]}"; do
  if [[ ! -s "$path" ]]; then
    printf 'Phase 9 foundation missing required file: %s\n' "$path" >&2
    exit 1
  fi
done

for path in "${required_dirs[@]}"; do
  if [[ ! -d "$path" ]]; then
    printf 'Phase 9 foundation missing required directory: %s\n' "$path" >&2
    exit 1
  fi
done

grep -q 'Module green' docs/phase9/README.md
grep -q 'Full-run no-regression' docs/phase9/README.md
grep -q 'Final strict green' docs/phase9/README.md
grep -q 'read-only input' docs/phase9/source-integrity.md
grep -q 'navigation aid' docs/phase9/source-lookup.md
grep -q 'PHASE9_TARGET_CLI_COMPAT' docs/phase9/binary-discovery.md
grep -q 'official `run-tests.php` wrapper' docs/phase9/official-runner.md
grep -q 'complete discovered PHPT corpus' docs/phase9/full-phpt-gate.md

if [[ -f tests/phpt/manifests/php-src-hashes.jsonl && ! -s tests/phpt/manifests/php-src-hashes.jsonl ]]; then
  printf '%s\n' 'Phase 9 source hash manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f tests/phpt/manifests/php-src-symbols.jsonl && ! -s tests/phpt/manifests/php-src-symbols.jsonl ]]; then
  printf '%s\n' 'Phase 9 source symbol manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f tests/phpt/manifests/phpt-corpus.jsonl && ! -s tests/phpt/manifests/phpt-corpus.jsonl ]]; then
  printf '%s\n' 'Phase 9 PHPT corpus manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f docs/phase9/reports/phpt-corpus-summary.md && ! -s docs/phase9/reports/phpt-corpus-summary.md ]]; then
  printf '%s\n' 'Phase 9 PHPT corpus summary exists but is empty.' >&2
  exit 1
fi

printf '%s\n' '[ok] Phase 9 foundation docs and directories are present.'
