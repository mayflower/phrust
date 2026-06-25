#!/usr/bin/env bash
set -euo pipefail

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  else
    php_src="third_party/php-src"
  fi
fi

manifest="${PHPT_MANIFEST:-tests/phpt/manifests/original-smoke.jsonl}"
work_dir="${PHPT_WORK_DIR:-target/phpt-work}/official-smoke"
rust_dir="$work_dir/rust"
official_dir="$work_dir/official"

reference_php="${REFERENCE_PHP:-}"
if [[ -z "$reference_php" ]]; then
  reference_php="$php_src/sapi/cli/php"
fi

if [[ ! -x "$reference_php" ]]; then
  printf '%s\n' 'Reference PHP CLI is not built; cannot run official PHPT smoke.' >&2
  printf '%s\n' 'Run: nix develop -c just build-ref-php' >&2
  exit 1
fi

if [[ ! -s "$manifest" ]]; then
  printf 'PHPT smoke manifest is missing or empty: %s\n' "$manifest" >&2
  exit 1
fi

if [[ ! -f "$php_src/run-tests.php" ]]; then
  printf 'php-src run-tests.php is missing: %s/run-tests.php\n' "$php_src" >&2
  exit 1
fi

php_src_abs="$(cd "$php_src" && pwd)"
reference_php_abs="$(cd "$(dirname "$reference_php")" && pwd)/$(basename "$reference_php")"

target_php="${TARGET_PHP:-$reference_php_abs}"
if [[ "$target_php" != /* ]]; then
  target_php="$(cd "$(dirname "$target_php")" && pwd)/$(basename "$target_php")"
fi

if [[ ! -x "$target_php" ]]; then
  printf 'Target PHP executable is not built: %s\n' "$target_php" >&2
  exit 1
fi

mkdir -p "$rust_dir" "$official_dir/temp-target"

test_list="$official_dir/test-list.txt"
: > "$test_list"
while IFS= read -r line; do
  [[ -z "$line" || "$line" == \#* ]] && continue
  path="$(printf '%s\n' "$line" | sed -n 's/^.*"path":"\([^"]*\)".*$/\1/p')"
  if [[ -z "$path" ]]; then
    path="$line"
  fi
  printf '%s/%s\n' "$php_src_abs" "$path" >> "$test_list"
done < "$manifest"

if [[ ! -s "$test_list" ]]; then
  printf 'Official PHPT smoke test list is empty: %s\n' "$test_list" >&2
  exit 1
fi

printf 'REFERENCE_PHP=%s\n' "$reference_php_abs"
printf 'TARGET_PHP=%s\n' "$target_php"
if [[ -z "${TARGET_PHP:-}" ]]; then
  printf '%s\n' '[info] TARGET_PHP is unset; official smoke uses REFERENCE_PHP as the -p executable.'
fi
printf 'PHP_SRC_DIR=%s\n' "$php_src_abs"
printf 'PHPT_MANIFEST=%s\n' "$manifest"

default_phpt_tool="${CARGO_TARGET_DIR:-target}/debug/php-phpt-tools"
phpt_tool="${PHPT_TOOLS_BIN:-$default_phpt_tool}"
if [[ -z "${PHPT_TOOLS_BIN:-}" && "$phpt_tool" == "$default_phpt_tool" ]]; then
  cargo build -q -p php_phpt_tools --bin php-phpt-tools
elif [[ ! -x "$phpt_tool" ]]; then
  printf 'PHPT tools executable is not built: %s\n' "$phpt_tool" >&2
  exit 1
fi

"$phpt_tool" run \
  --target "$target_php" \
  --manifest "$manifest" \
  --out "$rust_dir/results.jsonl" \
  --summary "$rust_dir/summary.md" \
  --php-src "$php_src_abs"

set +e
NO_INTERACTION=1 TEST_PHP_EXECUTABLE="$target_php" "$reference_php_abs" "$php_src_abs/run-tests.php" \
  -q \
  -p "$target_php" \
  -r "$test_list" \
  -w "$official_dir/failed.txt" \
  -W "$official_dir/all.txt" \
  -s "$official_dir/run-tests.log" \
  --temp-source "$php_src_abs" \
  --temp-target "$official_dir/temp-target" \
  > "$official_dir/stdout.log" 2> "$official_dir/stderr.log"
official_status=$?
set -e

summary="$official_dir/summary.md"
{
  printf '%s\n' '# PHPT Official PHPT Smoke'
  printf '\n'
  printf 'Reference PHP: `%s`\n\n' "$reference_php_abs"
  printf 'Target PHP: `%s`\n\n' "$target_php"
  printf 'Manifest: `%s`\n\n' "$manifest"
  printf 'Test list: `%s`\n\n' "$test_list"
  printf 'Official run-tests.php exit status: `%s`\n\n' "$official_status"
  printf 'Artifacts:\n\n'
  printf -- '- Rust runner JSONL: `%s`\n' "$rust_dir/results.jsonl"
  printf -- '- Rust runner summary: `%s`\n' "$rust_dir/summary.md"
  printf -- '- Official all-status list: `%s`\n' "$official_dir/all.txt"
  printf -- '- Official failed list: `%s`\n' "$official_dir/failed.txt"
  printf -- '- Official output log: `%s`\n' "$official_dir/run-tests.log"
  printf -- '- Official stdout: `%s`\n' "$official_dir/stdout.log"
  printf -- '- Official stderr: `%s`\n' "$official_dir/stderr.log"
} > "$summary"

if [[ "$official_status" -ne 0 ]]; then
  printf 'official run-tests.php smoke failed with status %s; see %s\n' "$official_status" "$official_dir/stdout.log" >&2
  exit "$official_status"
fi

if [[ -s "$official_dir/failed.txt" ]]; then
  printf 'official run-tests.php smoke recorded failures in %s\n' "$official_dir/failed.txt" >&2
  cat "$official_dir/failed.txt" >&2
  exit 1
fi

if grep -q '"outcome":"FAIL"\|"outcome":"BORK"\|"outcome":"TIMEOUT"\|"outcome":"CRASH"' "$rust_dir/results.jsonl"; then
  printf 'Rust PHPT runner smoke recorded non-green outcomes in %s\n' "$rust_dir/results.jsonl" >&2
  exit 1
fi

if [[ -n "$(git -C "$php_src_abs" status --short)" ]]; then
  printf '%s\n' 'official run-tests.php smoke mutated the php-src checkout:' >&2
  git -C "$php_src_abs" status --short >&2
  exit 1
fi

printf '[ok] official run-tests.php smoke agrees with Rust runner for %s\n' "$manifest"
printf '[ok] official smoke artifacts: %s\n' "$official_dir"
