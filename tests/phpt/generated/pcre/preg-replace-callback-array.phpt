--TEST--
pcre: preg_replace_callback_array dispatches userland callbacks
--DESCRIPTION--
Generated focused coverage for sequential callback-array replacement, total
count by reference, array subject key preservation, and empty pattern maps.
--FILE--
<?php
function pcre_wrap_a($m) {
    return '[' . $m[0] . ']';
}

$count = null;
var_dump(preg_replace_callback_array([
    '/a+/' => 'pcre_wrap_a',
    '/b+/' => function ($m) {
        return strtoupper($m[0]);
    },
], 'aa bb ab', -1, $count));
var_dump($count);

$count = null;
var_dump(preg_replace_callback_array([
    '/a/' => function ($m) {
        return 'A';
    },
], ['x' => 'a', 'y' => 'b'], -1, $count));
var_dump($count);

var_dump(preg_replace_callback_array([], 'abc'));
var_dump(preg_last_error_msg());
?>
--EXPECT--
string(12) "[aa] BB [a]B"
int(4)
array(2) {
  ["x"]=>
  string(1) "A"
  ["y"]=>
  string(1) "b"
}
int(1)
string(3) "abc"
string(8) "No error"
