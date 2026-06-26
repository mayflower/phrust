--TEST--
Generated standard.arrays: var_dump renders shortest scientific floats
--DESCRIPTION--
module: standard.arrays
generated timestamp: 20260626T000000Z
generator version: phpt-standard-arrays-v1
reason: var_dump uses the shortest round-trippable mantissa in PHP's E+dd notation (serialize_precision=-1), e.g. 1.8446744073709552E+19 not a truncated 16-digit form (tests/standard/array/min_int_float_optimisation.phpt)
--FILE--
<?php
var_dump(1e20, -1.8446744073709552E+19, 1.5e-10, 9.999999E+22);
?>
--EXPECT--
float(1.0E+20)
float(-1.8446744073709552E+19)
float(1.5E-10)
float(9.999999E+22)
