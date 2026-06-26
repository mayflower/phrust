--TEST--
Generated standard.strings: uniqid length/uniqueness and usleep
--DESCRIPTION--
module: standard.strings
generated timestamp: 20260626T000000Z
generator version: phpt-standard-strings-v1
reason: uniqid(prefix) is prefix+13 chars, uniqid(prefix,true) is prefix+23, more_entropy ids are unique, and usleep is callable (tests/strings/001.phpt)
--FILE--
<?php
$a = uniqid("p", true);
$b = uniqid("p", true);
var_dump(strlen($a) === 24, strlen($a) === strlen($b), $a !== $b);
$c = uniqid("p");
usleep(1);
$d = uniqid("p");
var_dump(strlen($c) === 14, strlen($d) === 14);
?>
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
