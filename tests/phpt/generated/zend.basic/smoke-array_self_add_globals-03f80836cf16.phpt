--TEST--
Phase 9 generated smoke: Add $GLOBALS to itself
--DESCRIPTION--
original php-src path: Zend/tests/array_self_add_globals.phpt
original source hash: 03f80836cf1639f10d5644aa8f5ccbcc1d99c9c04dbfa46b7ac9069686f7fd17
generated timestamp: 20260624T084753Z
generator version: phase9-generate-v1
reason: smallest reference-passing example
--FILE--
<?php
$x = $GLOBALS + $GLOBALS;
?>
===DONE===
--EXPECT--
===DONE===
