#!/usr/bin/env bash
set -euo pipefail

required_files=(
  "docs/phpt/README.md"
  "docs/phpt/source-integrity.md"
  "docs/phpt/source-lookup.md"
  "docs/phpt/binary-discovery.md"
  "docs/phpt/official-runner.md"
  "docs/phpt/full-phpt-gate.md"
)

required_dirs=(
  "docs/phpt/modules"
  "docs/phpt/php-src-behavior"
  "docs/phpt/reports"
  "tests/phpt/generated"
  "tests/phpt/manifests"
)

for path in "${required_files[@]}"; do
  if [[ ! -s "$path" ]]; then
    printf 'PHPT foundation missing required file: %s\n' "$path" >&2
    exit 1
  fi
done

for path in "${required_dirs[@]}"; do
  if [[ ! -d "$path" ]]; then
    printf 'PHPT foundation missing required directory: %s\n' "$path" >&2
    exit 1
  fi
done

grep -q 'Module green' docs/phpt/README.md
grep -q 'Full-run no-regression' docs/phpt/README.md
grep -q 'Final strict green' docs/phpt/README.md
grep -q 'read-only input' docs/phpt/source-integrity.md
grep -q 'navigation aid' docs/phpt/source-lookup.md
grep -q 'PHPT_TARGET_CLI_COMPAT' docs/phpt/binary-discovery.md
grep -q 'official `run-tests.php` wrapper' docs/phpt/official-runner.md
grep -q 'complete discovered PHPT corpus' docs/phpt/full-phpt-gate.md

if [[ -f tests/phpt/manifests/php-src-hashes.jsonl && ! -s tests/phpt/manifests/php-src-hashes.jsonl ]]; then
  printf '%s\n' 'PHPT source hash manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f tests/phpt/manifests/php-src-symbols.jsonl && ! -s tests/phpt/manifests/php-src-symbols.jsonl ]]; then
  printf '%s\n' 'PHPT source symbol manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f tests/phpt/manifests/phpt-corpus.jsonl && ! -s tests/phpt/manifests/phpt-corpus.jsonl ]]; then
  printf '%s\n' 'PHPT corpus manifest exists but is empty.' >&2
  exit 1
fi

if [[ -f docs/phpt/reports/phpt-corpus-summary.md && ! -s docs/phpt/reports/phpt-corpus-summary.md ]]; then
  printf '%s\n' 'PHPT corpus summary exists but is empty.' >&2
  exit 1
fi

printf '%s\n' '[ok] PHPT foundation docs and directories are present.'
