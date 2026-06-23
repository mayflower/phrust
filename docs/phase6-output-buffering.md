# Phase 6 Output Buffering

Reference target: PHP 8.5.7 (`php-8.5.7`).

Prompt 06.24 implements a stack-aware output buffer in `php_runtime` and
VM-backed standard functions for Composer/framework-style output capture.

## Implemented

- `ob_start()` starts a nested output buffer and returns `true`.
- `ob_get_contents()` returns the active buffer contents or `false`.
- `ob_get_clean()` returns and discards the active buffer or `false`.
- `ob_get_length()` returns the active buffer byte length or `false`.
- `ob_get_level()` returns the current buffer stack depth.
- `ob_end_clean()` discards the active buffer and returns whether one existed.
- `ob_end_flush()` flushes the active buffer into its parent or root output and
  returns whether one existed.
- `flush()` is a deterministic no-op that returns `null`.
- Open buffers are flushed to root output during VM finalization.
- Caught exceptions do not corrupt the output buffer stack.

## Known Gap

- `PHASE6-GAP-OUTPUT-BUFFER-CALLBACKS`: `ob_start($callback)` transformation,
  chunk flags, and callback lifecycle edge cases are intentionally deferred.

## Validation

- VM unit tests cover nested buffers, clean/flush behavior, and caught
  exceptions while a buffer is active.
- Differential fixtures:
  - `PHASE6_STDLIB_OUTPUT_BUFFERING`
  - `PHASE6_STDLIB_OUTPUT_BUFFERING_EXCEPTION`
