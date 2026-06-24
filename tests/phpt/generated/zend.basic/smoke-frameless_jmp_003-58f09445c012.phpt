--TEST--
Phase 9 generated smoke: Frameless jmp
--DESCRIPTION--
original php-src path: Zend/tests/frameless_jmp_003.phpt
original source hash: 58f09445c012ab9756b4c6b46bfc61d8dc5c334b3eb36a392e60bc429f0f7c32
generated timestamp: 20260624T084753Z
generator version: phase9-generate-v1
reason: smallest reference-passing example
--FILE--
<?php
namespace Foo;
preg_replace('/foo/', '', '');
?>
--EXPECT--
