--TEST--
Phase 4 PHPT SKIPIF classification
--SKIPIF--
<?php die("skip explicit smoke skip\n"); ?>
--FILE--
<?php
echo "should not run\n";
--EXPECT--
should not run
