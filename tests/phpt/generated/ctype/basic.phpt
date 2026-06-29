--TEST--
ctype ASCII C-locale basics
--SKIPIF--
<?php if (!extension_loaded("ctype")) die("skip ctype extension not loaded"); ?>
--FILE--
<?php
echo ctype_alnum("abc123") ? "A" : "a";
echo ctype_alpha("abcXYZ") ? "B" : "b";
echo ctype_digit("0123") ? "C" : "c";
echo ctype_xdigit("09afAF") ? "D" : "d";
echo ctype_space(" \t\n") ? "E" : "e";
echo ctype_digit("") ? "bad" : "F";
echo "\n";
?>
--EXPECT--
ABCDEF
