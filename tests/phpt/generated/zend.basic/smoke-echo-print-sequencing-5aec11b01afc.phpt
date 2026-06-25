--TEST--
Generated smoke: echo, print, and statement sequencing
--DESCRIPTION--
original php-src path: Zend/tests/foreach/bug41351.phpt
original source hash: 5aec11b01afcb600677ca69121aa5399cf244c3b0ac67431838a0605b23c4c87
generated timestamp: 20260625T000000Z
generator version: phpt-zend-basic-v1
reason: reduced top-level output sequencing coverage from reference output
--FILE--
<?php
echo "alpha";
print " beta\n";
echo 1, 2, 3, "\n";
?>
--EXPECT--
alpha beta
123
