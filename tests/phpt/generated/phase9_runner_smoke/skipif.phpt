--TEST--
Phase 9 runner SKIPIF smoke
--SKIPIF--
<?php
echo "skip deliberate phase9 smoke\n";
--FILE--
<?php
echo "should not run\n";
--EXPECT--
should not run
