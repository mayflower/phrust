--TEST--
Generated standard.arrays: array_pop rewinds the negative auto-index
--DESCRIPTION--
module: standard.arrays
generated timestamp: 20260626T000000Z
generator version: phpt-standard-arrays-v1
reason: popping the trailing auto-index element decrements the next free index, so a following append reuses it; appends after a negative key continue from that key (tests/standard/array/negative_index.phpt)
--FILE--
<?php
$d[-2] = true;
$d[] = true;
$d[] = true;
var_dump(array_keys($d));
$e = [-2 => false];
array_pop($e);
$e[] = true;
$e[] = true;
var_dump(array_keys($e));
?>
--EXPECT--
array(3) {
  [0]=>
  int(-2)
  [1]=>
  int(-1)
  [2]=>
  int(0)
}
array(2) {
  [0]=>
  int(-2)
  [1]=>
  int(-1)
}
