--TEST--
ctype legacy non-string fallback behavior
--SKIPIF--
<?php if (!extension_loaded("ctype")) die("skip ctype extension not loaded"); ?>
--FILE--
<?php
error_reporting(E_ALL & ~E_DEPRECATED);
echo ctype_digit(48) ? "A" : "a";
echo ctype_digit(65) ? "bad" : "B";
echo ctype_upper(-65) ? "bad" : "C";
echo ctype_digit(256) ? "D" : "d";
echo ctype_alpha(256) ? "bad" : "E";
echo ctype_graph(-129) ? "F" : "f";
echo ctype_digit(-129) ? "bad" : "G";
echo ctype_digit(true) ? "bad" : "H";
echo ctype_digit(null) ? "bad" : "I";
echo ctype_digit([]) ? "bad" : "J";
echo "\n";
?>
--EXPECT--
ABCDEFGHIJ
