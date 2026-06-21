--TEST--
Phase 4 PHPT INI known gap classification
--INI--
precision=14
--FILE--
<?php
echo "should not run\n";
--EXPECT--
should not run
