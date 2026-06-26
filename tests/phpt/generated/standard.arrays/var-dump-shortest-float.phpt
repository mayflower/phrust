--TEST--
Generated standard.arrays: var_dump uses shortest float by default
--DESCRIPTION--
module: standard.arrays
generated timestamp: 20260626T000000Z
generator version: phpt-standard-arrays-v1
reason: with the default serialize_precision=-1 var_dump prints the shortest round-trippable float, not a precision-limited form (tests/standard/array/range/range_inputs_float_basic.phpt with serialize_precision=14 shows the opposite case)
--FILE--
<?php
var_dump(1.6 + 0.1);
var_dump(-1.8446744073709552E+19);
?>
--EXPECT--
float(1.7000000000000002)
float(-1.8446744073709552E+19)
