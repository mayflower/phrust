# Performance Output Buffer Fast Paths

Work item keeps PHP-visible output semantics unchanged while reducing
avoidable work in string-heavy output paths.

## Implemented Paths

- `OutputBuffer` tracks request-local append, batch-write, and flush statistics.
- Empty writes are ignored before touching the active buffer.
- Internal callers can append several byte slices through one active-buffer
  reservation with `write_slices`.
- The VM preallocates the root output buffer from statically known literal
  `echo` string operands. This is only a reserve hint and does not write bytes.
- `echo` writes existing `Value::String` bytes directly and handles `true`,
  `false`, and `null` without allocating an intermediate `PhpString`.
- Object, reference, and fallback conversions still flow through
  `value_to_string`, preserving `__toString` side effects and conversion
  errors.

## Semantics Boundaries

Output buffering levels are not bypassed. Writes still target the active buffer
when `ob_start` is active, and `ob_end_flush`/shutdown flushing still controls
when nested bytes become visible in root output.

Output buffering callbacks remain the Standard library unsupported gap. Fast paths must
not attempt to invoke or skip callbacks.

## Validation

The focused VM tests cover multi-argument `echo`, `print` return value output,
buffer flushing, object `__toString` fallback, throwing `__toString`, and the
callback unsupported diagnostic. The Performance smoke corpus includes
`tests/fixtures/performance/perf_smoke/output_writes.php` so `benchmark-smoke`
reports output append/flush counters in the hotpath inventory.
