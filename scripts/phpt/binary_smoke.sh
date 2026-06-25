#!/usr/bin/env bash
set -euo pipefail

mode="${1:?usage: binary_smoke.sh reference|target}"

php_src="${PHP_SRC_DIR:-}"
if [[ -z "$php_src" ]]; then
  if [[ -d third_party/php-src-8.5.7 ]]; then
    php_src="third_party/php-src-8.5.7"
  else
    php_src="third_party/php-src"
  fi
fi

work_dir="${PHPT_WORK_DIR:-target/phpt-work}"
mkdir -p "$work_dir/reference" "$work_dir/target" "$work_dir/full-runs" "$work_dir/module-runs"

reference_php="${REFERENCE_PHP:-}"
if [[ -z "$reference_php" ]]; then
  if [[ -x "$php_src/sapi/cli/php" ]]; then
    reference_php="$php_src/sapi/cli/php"
  else
    printf '%s\n' 'Reference PHP CLI is not built.' >&2
    printf '%s\n' 'Run: nix develop -c just build-ref-php' >&2
    exit 1
  fi
fi

case "$mode" in
  reference)
    printf 'REFERENCE_PHP=%s\n' "$reference_php"
    printf 'PHP_SRC_DIR=%s\n' "$php_src"
    printf 'PHPT_WORK_DIR=%s\n' "$work_dir"
    "$reference_php" -v >/dev/null
    [[ "$("$reference_php" -r 'echo "ok\n";')" == "ok" ]]
    printf '%s\n' '[ok] Reference PHP supports -v and -r.'
    ;;
  target)
    target_php="${TARGET_PHP:-}"
    if [[ -z "$target_php" ]]; then
      target_php="target/debug/phrust-php"
    fi
    printf 'TARGET_PHP=%s\n' "$target_php"
    printf 'PHPT_WORK_DIR=%s\n' "$work_dir"
    if [[ -z "${TARGET_PHP:-}" && "$target_php" == "target/debug/phrust-php" ]]; then
      cargo build -p php_vm_cli --bin phrust-php
    elif [[ ! -x "$target_php" ]]; then
      printf 'Target PHP executable is not built: %s\n' "$target_php" >&2
      exit 1
    fi
    version_out="$("$target_php" -v 2>&1)"
    run_out="$("$target_php" -r 'echo "ok\n";' 2>&1)"
    if [[ "$version_out" != *"PHP"* || "$run_out" != "ok" ]]; then
      printf '%s\n' '[fail] Target PHP lacks required PHP CLI flags.' >&2
      printf '%s\n' '[fail] Required smoke: TARGET_PHP -v and TARGET_PHP -r '\''echo "ok\n";'\''' >&2
      printf 'TARGET_PHP -v output:\n%s\n' "$version_out"
      printf 'TARGET_PHP -r output:\n%s\n' "$run_out"
      exit 1
    fi
    printf '%s\n' '[ok] Target PHP supports -v and -r.'
    ;;
  *)
    printf 'unknown PHPT binary smoke mode: %s\n' "$mode" >&2
    exit 2
    ;;
esac
