--TEST--
Phase 4 PHPT variables
--FILE--
<?php
$value = 7;
$value += 5;
echo $value, "\n";
--EXPECT--
12
