--TEST--
Generated smoke: top-level return stops execution
--DESCRIPTION--
original php-src path: Zend/tests/bug44913.phpt
original source hash: f9f31fdf1b6ef1036782a07d8c670e253a92189e24301e3656ffd958202a0da1
generated timestamp: 20260625T000000Z
generator version: phpt-zend-basic-v1
reason: reduced top-level return coverage from reference output
--FILE--
<?php
echo "before\n";
return;
echo "after\n";
?>
--EXPECT--
before
