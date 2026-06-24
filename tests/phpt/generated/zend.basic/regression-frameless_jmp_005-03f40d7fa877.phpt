--TEST--
Phase 9 generated regression: Frameless jmp
--DESCRIPTION--
original php-src path: Zend/tests/frameless_jmp_005.phpt
original source hash: 03f40d7fa8776c860107e3a992ead4507c103033c7603dcb1b254791b34cb50a
generated timestamp: 20260624T084753Z
generator version: phase9-generate-v1
reason: known target failure minimized against reference output
--FILE--
<?php
var_dump(preg_replace("/foo/", '', '', 1));
--EXPECT--
string(0) ""
