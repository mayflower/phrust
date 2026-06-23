# Phase 6 Array Stack, Slice, and Merge Helpers

Reference target: PHP 8.5.7 (`php-8.5.7`).

Prompt 06.17 implements mutation-safe array helpers:

- Mutating stack helpers: `array_push`, `array_pop`, `array_shift`,
  `array_unshift`, and `array_splice`.
- Copying helpers: `array_slice`, `array_merge`,
  `array_merge_recursive`, `array_replace`, `array_reverse`,
  `array_pad`, `array_chunk`, and `array_flip`. Unsupported `array_flip`
  values are skipped with PHP-style warnings.

The VM binds the first argument of the mutating helpers by reference through
the existing `ReferenceCell` mechanism. Copying helpers build new arrays and
leave their input arrays untouched.

Known gaps for this slice:

- `PHASE6-GAP-ARRAY-MERGE-RECURSIVE-REFERENCES`
