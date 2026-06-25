--TEST--
PHPT runner XFAIL smoke
--XFAIL--
deliberate mismatch proves expected-failure handling
--FILE--
<?php
echo "actual\n";
--EXPECT--
expected
