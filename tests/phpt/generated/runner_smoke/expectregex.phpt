--TEST--
PHPT runner EXPECTREGEX smoke
--FILE--
<?php
echo "token=abc123\n";
--EXPECTREGEX--
^token=[a-z]+[0-9]+$
