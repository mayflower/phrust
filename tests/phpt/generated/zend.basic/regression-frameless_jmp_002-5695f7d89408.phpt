--TEST--
Phase 9 generated regression: Frameless jmp
--DESCRIPTION--
original php-src path: Zend/tests/frameless_jmp_002.phpt
original source hash: 5695f7d8940825ca79f6cbe593edce7112fdddaa77b6887234ea3aed49086f98
generated timestamp: 20260624T084753Z
generator version: phase9-generate-v1
reason: known target failure minimized against reference output
--FILE--
<?php
var_dump(class_exists('\foo'));
--EXPECT--
bool(false)
