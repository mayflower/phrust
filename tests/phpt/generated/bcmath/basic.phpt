--TEST--
bcmath bounded decimal basics
--SKIPIF--
<?php if (!extension_loaded("bcmath")) die("skip bcmath extension not loaded"); ?>
--FILE--
<?php
echo bcadd("1.20", "3.45", 2), "\n";
echo bcsub("5", "2.5", 1), "\n";
echo bcmul("1.5", "2", 2), "\n";
echo bcdiv("7", "2", 3), "\n";
echo bcpow("2", "10"), "\n";
echo bccomp("1.23", "1.22", 2), "\n";
?>
--EXPECT--
4.65
2.5
3.00
3.500
1024
1
