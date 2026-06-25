--TEST--
Generated smoke: top-level exit stops execution
--DESCRIPTION--
original php-src path: Zend/tests/exit/exit_values.phpt
original source hash: ff648fb6d646696cf9371cbcf9fe18a2cb015e652b4e286733df6e7a349c73af
generated timestamp: 20260625T000000Z
generator version: phpt-zend-basic-v1
reason: reduced top-level exit coverage from reference output
--FILE--
<?php
echo "before\n";
exit;
echo "after\n";
?>
--EXPECT--
before
