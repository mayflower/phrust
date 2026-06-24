--TEST--
Phase 9 generated regression: Zend basic numeric literal separators
--DESCRIPTION--
original php-src path: Zend/tests/numeric_literal_separator/numeric_literal_separator_001.phpt
original source hash: e7854d36777eda6250e02933451e2ae35ebfbd4814267ddd833a843663eda9e1
generated timestamp: 20260624T000000Z
generator version: phase9-zend-basic-v1
reason: reduced scalar-literal regression generated from reference output
--FILE--
<?php
var_dump(299_792_458 === 299792458);
var_dump(96_485.332_12 === 96485.33212);
var_dump(6.626_070_15e-34 === 6.62607015e-34);
var_dump(0xCAFE_F00D === 0xCAFEF00D);
var_dump(0b0101_1111 === 0b01011111);
var_dump(0137_041 === 0137041);
var_dump(0_124 === 0124);
--EXPECT--
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
bool(true)
