--TEST--
PHPT runner INI smoke
--INI--
precision=14
--FILE--
<?php
echo ini_get("precision"), "\n";
--EXPECT--
14
