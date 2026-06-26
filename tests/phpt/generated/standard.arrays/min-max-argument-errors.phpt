--TEST--
Generated standard.arrays: min/max argument errors match PHP wording
--DESCRIPTION--
module: standard.arrays
generated timestamp: 20260626T000000Z
generator version: phpt-standard-arrays-v1
reason: single-argument min/max require a non-empty array and throw PHP-worded TypeError/ValueError with get_debug_type names (tests/standard/array/min.phpt, max.phpt)
--FILE--
<?php
try { min(1); } catch (\TypeError $e) { echo $e->getMessage(), "\n"; }
try { min([]); } catch (\ValueError $e) { echo $e->getMessage(), "\n"; }
try { max(new stdClass()); } catch (\TypeError $e) { echo $e->getMessage(), "\n"; }
?>
--EXPECT--
min(): Argument #1 ($value) must be of type array, int given
min(): Argument #1 ($value) must contain at least one element
max(): Argument #1 ($value) must be of type array, stdClass given
