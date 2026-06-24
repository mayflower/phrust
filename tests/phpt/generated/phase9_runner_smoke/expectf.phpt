--TEST--
Phase 9 runner EXPECTF smoke
--FILE--
<?php
echo "value=42\n";
--EXPECTF--
value=%d
