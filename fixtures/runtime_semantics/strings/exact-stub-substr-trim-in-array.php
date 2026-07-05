<?php

$path = '/items/42/edit';
var_dump(substr($path, 0, 7));
var_dump(substr($path, 7));
var_dump(substr($path, -4));
var_dump(substr($path, 3, -5));
var_dump(substr($path, 99));
var_dump(substr($path, 0, null));
var_dump(substr($path, length: 6, offset: 1));

var_dump(trim("  \t\n padded \v\0 "));
var_dump(trim('untouched'));
var_dump(trim(''));
var_dump(trim('xxvaluexx', 'x'));
var_dump(trim(42));

$haystack = ['10', 20, 'thirty', '40'];
var_dump(in_array('10', $haystack, true));
var_dump(in_array(10, $haystack, true));
var_dump(in_array(20, $haystack, true));
var_dump(in_array('20', $haystack, true));
var_dump(in_array('40', $haystack, false));
var_dump(in_array(40, $haystack));
var_dump(in_array(2.5, [2.5, 3.5], true));

try {
    substr('x');
} catch (ArgumentCountError $error) {
    echo get_class($error), "\n";
}
try {
    in_array('only');
} catch (ArgumentCountError $error) {
    echo get_class($error), "\n";
}

$ref = '  spaced  ';
$alias = &$ref;
var_dump(trim($alias));
var_dump($ref);
