--TEST--
runtime PHPT array
--FILE--
<?php
$items = ["first" => 1, "second" => 2];
echo $items["first"], "|", $items["second"], "\n";
--EXPECT--
1|2
