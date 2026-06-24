--TEST--
Phase 9 generated regression: scalar operators and conversions
--DESCRIPTION--
original php-src path: Zend/tests/concat/concat_002.phpt
original source hash: 8930bdcfc75261fd13aff475dc89100ab5b2b1dd8bf397cc0ddf529eb138d0a8
related originals:
- Zend/tests/in-de-crement/increment_001_64bit.phpt (433a185b405a9a3a499497b74e7ead060de3e02fb1a20e94746e8044329d9ed6)
generated timestamp: 20260624T000000Z
generator version: phase9-operators-conversions-v1
reason: reduced arithmetic, bitwise, comparison, concat, truthiness, and overflow regression generated from reference output
--FILE--
<?php
var_dump(1 + 2 * 3);
var_dump(8 / 4);
var_dump(7 % 4);
var_dump(2 ** 3);
var_dump(6 & 3, 6 | 3, 6 ^ 3, 8 << 1, 8 >> 1);
$x = 6;
$x &= 3;
var_dump($x);
$y = "a";
$y .= "b";
var_dump($y);
var_dump(42 == "000042");
var_dump(42 == "42abc");
var_dump(2 <=> 3);
var_dump((bool)"0", (bool)"00");
$i = PHP_INT_MAX;
$i++;
var_dump($i);
--EXPECT--
int(7)
int(2)
int(3)
int(8)
int(2)
int(7)
int(5)
int(16)
int(4)
int(2)
string(2) "ab"
bool(true)
bool(false)
int(-1)
bool(false)
bool(true)
float(9.223372036854776E+18)
