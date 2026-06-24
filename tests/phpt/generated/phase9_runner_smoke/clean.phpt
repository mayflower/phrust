--TEST--
Phase 9 runner CLEAN smoke
--FILE--
<?php
file_put_contents("phase9-clean.txt", "x");
echo file_exists("phase9-clean.txt") ? "made\n" : "missing\n";
--CLEAN--
<?php
@unlink("phase9-clean.txt");
echo "clean\n";
--EXPECT--
made
