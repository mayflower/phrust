--TEST--
Generated smoke: Zend basic scalar var_dump output
--DESCRIPTION--
original php-src path: Zend/tests/numeric_literal_separator/numeric_literal_separator_001.phpt
original source hash: e7854d36777eda6250e02933451e2ae35ebfbd4814267ddd833a843663eda9e1
generated timestamp: 20260624T000000Z
generator version: phpt-zend-basic-v1
reason: reduced scalar var_dump coverage for zend.basic
--FILE--
<?php
var_dump(null);
var_dump(true);
var_dump(42);
var_dump("ok");
?>
--EXPECT--
NULL
bool(true)
int(42)
string(2) "ok"
