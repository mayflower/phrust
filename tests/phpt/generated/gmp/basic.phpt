--TEST--
gmp BigInt facade basics
--SKIPIF--
<?php if (!extension_loaded("gmp")) die("skip gmp extension not loaded"); ?>
--FILE--
<?php
echo gmp_strval(gmp_add(gmp_init("0xff", 0), "1")), "\n";
echo gmp_strval(gmp_mul("12", "12")), "\n";
echo gmp_strval(gmp_div_q("7", "2")), "\n";
echo gmp_cmp("10", "9"), "\n";
echo gmp_strval(gmp_pow("2", 8)), "\n";
?>
--EXPECT--
256
144
3
1
256
