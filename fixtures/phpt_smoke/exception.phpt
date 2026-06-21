--TEST--
Phase 4 PHPT caught exception
--FILE--
<?php
try {
    throw new Exception("boom");
} catch (Exception $e) {
    echo "caught\n";
}
--EXPECT--
caught
