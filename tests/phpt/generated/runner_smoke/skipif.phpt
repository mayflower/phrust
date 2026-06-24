--TEST--
PHPT runner SKIPIF smoke
--SKIPIF--
<?php
echo "skip deliberate runner smoke\n";
--FILE--
<?php
echo "should not run\n";
--EXPECT--
should not run
