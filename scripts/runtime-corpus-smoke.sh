#!/usr/bin/env bash
set -euo pipefail

out_dir="${PHRUST_RUNTIME_CORPUS_OUT_DIR:-target/phase4/runtime-corpus-smoke}"
vm="${PHP_VM_CLI:-target/debug/php-vm}"
extra_limit="${PHRUST_RUNTIME_CORPUS_EXTRA_LIMIT:-25}"
mkdir -p "$out_dir"

report="$out_dir/report.txt"
: > "$report"

pass_count=0
known_gap_count=0
fail_count=0
skip_count=0
actual_failures=0
valid_total=0
valid_pass=0
valid_fail=0
extra_total=0
extra_pass=0

record() {
  printf '%s\n' "$*" | tee -a "$report"
}

safe_name() {
  printf '%s' "$1" | sed 's#[^A-Za-z0-9_.-]#_#g'
}

ensure_vm() {
  if [[ ! -x "$vm" ]]; then
    cargo build -p php_vm_cli
    vm="target/debug/php-vm"
  fi

  if [[ ! -x "$vm" ]]; then
    record "[error] Rust VM binary is not executable: $vm"
    exit 1
  fi
}

run_pass_fixture() {
  local name="$1"
  local file="$2"
  local expected="$3"
  local base
  base="$(safe_name "$name")"

  if "$vm" run "$file" > "$out_dir/$base.out" 2> "$out_dir/$base.err"; then
    printf '%s' "$expected" > "$out_dir/$base.expected"
    if cmp "$out_dir/$base.expected" "$out_dir/$base.out" >/dev/null; then
      pass_count=$((pass_count + 1))
      record "[pass] corpus $name"
    else
      actual_failures=$((actual_failures + 1))
      record "[error] corpus $name stdout mismatch"
    fi
  else
    actual_failures=$((actual_failures + 1))
    record "[error] corpus $name exited non-zero"
  fi
}

run_fail_fixture() {
  local name="$1"
  local file="$2"
  local expected_code="$3"
  local diagnostic="$4"
  local base
  local code
  base="$(safe_name "$name")"

  set +e
  "$vm" run "$file" > "$out_dir/$base.out" 2> "$out_dir/$base.err"
  code=$?
  set -e

  fail_count=$((fail_count + 1))
  if [[ "$code" -eq "$expected_code" ]] && grep -q "$diagnostic" "$out_dir/$base.err"; then
    record "[fail] corpus $name expected_exit=$expected_code diagnostic=$diagnostic"
  else
    actual_failures=$((actual_failures + 1))
    record "[error] corpus $name expected exit $expected_code with $diagnostic, got exit $code"
  fi
}

valid_args_for() {
  case "$1" in
    fixtures/runtime/valid/superglobals/argv.php)
      printf '%s\n' "alpha" "beta"
      ;;
    fixtures/runtime/valid/superglobals/server-argv.php)
      printf '%s\n' "red"
      ;;
  esac
}

run_valid_smoke() {
  local file
  local base
  local code
  local args=()

  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    valid_total=$((valid_total + 1))
    base="valid-$(safe_name "${file#fixtures/runtime/valid/}")"
    args=()
    while IFS= read -r arg; do
      [[ -n "$arg" ]] || continue
      args+=("$arg")
    done < <(valid_args_for "$file")

    set +e
    if [[ "${#args[@]}" -gt 0 ]]; then
      "$vm" run "$file" -- "${args[@]}" > "$out_dir/$base.out" 2> "$out_dir/$base.err"
    else
      "$vm" run "$file" > "$out_dir/$base.out" 2> "$out_dir/$base.err"
    fi
    code=$?
    set -e

    if [[ "$code" -eq 0 ]]; then
      valid_pass=$((valid_pass + 1))
      pass_count=$((pass_count + 1))
    else
      valid_fail=$((valid_fail + 1))
      actual_failures=$((actual_failures + 1))
      record "[error] valid smoke $file exited $code"
    fi
  done < <(find fixtures/runtime/valid -type f -name '*.php' | LC_ALL=C sort)

  record "[pass] valid runtime compile+execute smoke total=$valid_total pass=$valid_pass fail=$valid_fail"
}

run_optional_extra_paths() {
  local extra_paths="${PHRUST_RUNTIME_CORPUS_EXTRA_PATHS:-}"
  local roots=()
  local root
  local file
  local base
  local code
  local seen=0

  if [[ -z "$extra_paths" ]]; then
    skip_count=$((skip_count + 1))
    record "[skip] optional local corpus paths not set; set PHRUST_RUNTIME_CORPUS_EXTRA_PATHS for local Composer project compile smoke"
    return
  fi

  IFS=':' read -r -a roots <<< "$extra_paths"
  for root in "${roots[@]}"; do
    if [[ ! -d "$root" ]]; then
      skip_count=$((skip_count + 1))
      record "[skip] optional local corpus path is not a directory: $root"
      continue
    fi

    while IFS= read -r file; do
      [[ -n "$file" ]] || continue
      seen=$((seen + 1))
      extra_total=$((extra_total + 1))
      base="extra-$(safe_name "$seen-${file#$root/}")"

      set +e
      "$vm" compile "$file" --json > "$out_dir/$base.json" 2> "$out_dir/$base.err"
      code=$?
      set -e

      if [[ "$code" -eq 0 ]] && grep -q '"ok":true' "$out_dir/$base.json"; then
        extra_pass=$((extra_pass + 1))
        pass_count=$((pass_count + 1))
      else
        actual_failures=$((actual_failures + 1))
        record "[error] optional local corpus compile failed: $file"
      fi

      if [[ "$seen" -ge "$extra_limit" ]]; then
        break
      fi
    done < <(find "$root" -path '*/vendor' -prune -o -type f -name '*.php' -print | LC_ALL=C sort)
  done

  record "[pass] optional local corpus compile smoke total=$extra_total pass=$extra_pass limit=$extra_limit"
}

ensure_vm

run_pass_fixture "config-array" "fixtures/runtime/corpus_smoke/config-array.php" $'prod|db|1\n'
run_pass_fixture "router-dispatch" "fixtures/runtime/corpus_smoke/router-dispatch.php" $'users:index\n'
run_pass_fixture "class-methods" "fixtures/runtime/corpus_smoke/class-methods.php" $'1|3|jobs\n'
run_pass_fixture "include-graph" "fixtures/runtime/corpus_smoke/include-graph.php" $'root|config|health\n'
run_fail_fixture "error-case" "fixtures/runtime/corpus_smoke/error-case.php" 3 "E_PHP_RUNTIME_UNDEFINED_FUNCTION"
run_valid_smoke
run_optional_extra_paths

record "[summary] runtime corpus smoke pass=$pass_count known_gap=$known_gap_count fail=$fail_count skip=$skip_count actual_failures=$actual_failures report=$report"

if [[ "$actual_failures" -ne 0 ]]; then
  exit 1
fi

record "[ok] Phase 4 runtime corpus smoke passed."
