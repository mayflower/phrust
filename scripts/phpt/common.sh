#!/usr/bin/env bash

phpt_default_jobs() {
  if [[ -n "${PHPT_JOBS:-}" ]]; then
    printf '%s\n' "$PHPT_JOBS"
    return
  fi

  local detected=""
  if command -v nproc >/dev/null 2>&1; then
    detected="$(nproc 2>/dev/null || true)"
  elif command -v sysctl >/dev/null 2>&1; then
    detected="$(sysctl -n hw.ncpu 2>/dev/null || true)"
  fi

  if [[ ! "$detected" =~ ^[0-9]+$ ]] || [[ "$detected" -lt 1 ]]; then
    detected=1
  fi
  if [[ "$detected" -gt 8 ]]; then
    detected=8
  fi
  printf '%s\n' "$detected"
}

phpt_normalize_module() {
  case "$1" in
    closure.core-runtime)
      printf '%s\n' 'closure.core'
      ;;
    *)
      printf '%s\n' "$1"
      ;;
  esac
}
