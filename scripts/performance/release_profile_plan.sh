#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

printf '%s\n' '# performance Experimental Release Build Profile'
printf '%s\n' ''
printf '%s\n' 'Use the concrete smoke recipes below for local before/after measurements.'
printf '%s\n' 'They write reports under target/performance/release/ and treat timings as advisory.'
printf '%s\n' ''
printf '%s\n' 'Required release smoke:'
printf '%s\n' '  nix develop -c just release-benchmark-smoke'
printf '%s\n' ''
printf '%s\n' 'Optional PGO smoke:'
printf '%s\n' '  nix develop -c just pgo-benchmark-smoke'
printf '%s\n' '  nix develop -c env PHRUST_RUN_PGO=1 just pgo-benchmark-smoke'
printf '%s\n' ''
printf '%s\n' 'Optional BOLT smoke:'
printf '%s\n' '  nix develop -c just bolt-benchmark-smoke'
printf '%s\n' '  nix develop -c env PHRUST_RUN_BOLT=1 PHRUST_BOLT_PERF_DATA=target/performance/release/perf.data just bolt-benchmark-smoke'
printf '%s\n' ''
printf '%s\n' 'Do not commit target/performance/release/, *.profraw, *.profdata, perf.data, or optimized binaries.'
