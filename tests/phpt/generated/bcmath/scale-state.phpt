--TEST--
bcmath request-local scale state
--SKIPIF--
<?php if (!extension_loaded("bcmath")) die("skip bcmath extension not loaded"); ?>
--FILE--
<?php
var_dump(bcscale());
echo bcadd("1.2", "3.45"), "\n";
var_dump(bcscale(3));
var_dump(bcscale());
echo bcadd("1.2", "3.45"), "\n";
echo bcadd("1.2", "3.45", 1), "\n";
var_dump(bcscale(0));
var_dump(bcscale());
?>
--EXPECT--
int(0)
4
int(0)
int(3)
4.650
4.6
int(3)
int(0)
