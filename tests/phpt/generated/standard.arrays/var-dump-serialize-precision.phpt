--TEST--
Generated standard.arrays: var_dump honors serialize_precision
--INI--
serialize_precision=14
--FILE--
<?php
var_dump(1.6 + 0.1);
var_dump(0.1 + 0.2);
var_dump(1.0E20);
?>
--EXPECT--
float(1.7)
float(0.3)
float(1.0E+20)
