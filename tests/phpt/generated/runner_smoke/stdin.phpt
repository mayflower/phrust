--TEST--
PHPT runner STDIN smoke
--STDIN--
hello stdin
--FILE--
<?php
echo stream_get_contents(STDIN), "\n";
--EXPECT--
hello stdin
