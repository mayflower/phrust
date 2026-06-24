--TEST--
PHPT runner CLEAN smoke
--FILE--
<?php
file_put_contents("runner-clean.txt", "x");
echo file_exists("runner-clean.txt") ? "made\n" : "missing\n";
--CLEAN--
<?php
@unlink("runner-clean.txt");
echo "clean\n";
--EXPECT--
made
