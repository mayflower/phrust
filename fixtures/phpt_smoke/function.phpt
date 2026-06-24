--TEST--
runtime PHPT function with EXPECTF
--FILE--
<?php
function add($a, $b)
{
    return $a + $b;
}

echo "sum=", add(20, 22), "\n";
--EXPECTF--
sum=%d
