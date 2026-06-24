--TEST--
PHPT runner EXPECTF smoke
--FILE--
<?php
echo "value=42\n";
--EXPECTF--
value=%d
