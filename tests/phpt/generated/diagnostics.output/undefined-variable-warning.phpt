--TEST--
Generated smoke: undefined variable warning continues execution
--DESCRIPTION--
reference behavior: PHP 8.5.7 CLI undefined variable diagnostic
generated timestamp: 20260625T000000Z
generator version: phpt-diagnostics-output-v1
reason: central diagnostic output warning formatting and continuation
--FILE--
<?php
echo $missing;
echo "after\n";
--EXPECTF--
Warning: Undefined variable $missing in %s on line %d
after
