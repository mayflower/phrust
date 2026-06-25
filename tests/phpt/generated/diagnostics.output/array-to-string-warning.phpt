--TEST--
Generated smoke: array to string warning emits Array and continues
--DESCRIPTION--
reference behavior: PHP 8.5.7 CLI array to string diagnostic
generated timestamp: 20260625T000000Z
generator version: phpt-diagnostics-output-v1
reason: central diagnostic output warning formatting and continuation
--FILE--
<?php
echo [1, 2], "\n";
echo "done\n";
--EXPECTF--
Warning: Array to string conversion in %s on line %d
Array
done
