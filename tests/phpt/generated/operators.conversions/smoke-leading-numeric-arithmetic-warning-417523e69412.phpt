--TEST--
Generated smoke: leading numeric strings warn and convert in arithmetic
--DESCRIPTION--
original php-src path: Zend/tests/add_006.phpt
original source hash: 417523e69412980afcc56742d0ccc8cfa38a979ed1ac50c98e62170f74db7ee4
generated timestamp: 20260624T000000Z
generator version: phpt-operators-conversions-v1
reason: reduced leading-numeric arithmetic warning regression generated from reference output
--FILE--
<?php
var_dump(1 + "2x");
var_dump("4.5tail" * 2);
--EXPECTF--
Warning: A non-numeric value encountered in %s on line %d
int(3)

Warning: A non-numeric value encountered in %s on line %d
float(9)
