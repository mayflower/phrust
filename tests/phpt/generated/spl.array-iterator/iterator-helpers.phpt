--TEST--
SPL generated iterator helper functions cover arrays and ArrayIterator
--FILE--
<?php
echo iterator_count([]), "\n";
echo iterator_count(['a' => 1, 'b' => 2]), "\n";

var_dump(iterator_to_array(['a' => 1, 'b' => 2, 5 => 3]));
var_dump(iterator_to_array(['a' => 1, 'b' => 2, 5 => 3], false));

$it = new ArrayIterator(['x' => 7, 'y' => 8]);
echo iterator_count($it), "\n";
var_dump(iterator_to_array($it));
?>
--EXPECT--
0
2
array(3) {
  ["a"]=>
  int(1)
  ["b"]=>
  int(2)
  [5]=>
  int(3)
}
array(3) {
  [0]=>
  int(1)
  [1]=>
  int(2)
  [2]=>
  int(3)
}
2
array(2) {
  ["x"]=>
  int(7)
  ["y"]=>
  int(8)
}
