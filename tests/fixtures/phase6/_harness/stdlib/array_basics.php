<?php
// phase6-diff: id=PHASE6_STDLIB_ARRAY_BASICS area=stdlib expect=pass
$mixed = ["1" => "one", "01" => "zero-one", 2 => "two", "name" => "n"];
echo count($mixed), "|", sizeof([]), "\n";
echo array_key_exists("1", $mixed) ? "1" : "0";
echo array_key_exists("01", $mixed) ? "1" : "0";
echo array_key_exists("missing", $mixed) ? "1" : "0";
echo "\n";
echo var_export(array_keys($mixed), true), "\n";
echo var_export(array_values($mixed), true), "\n";
echo array_is_list(["x", "y"]) ? "1" : "0";
echo array_is_list($mixed) ? "1" : "0";
echo "\n";
echo var_export(array_key_first($mixed), true), "|", var_export(array_key_last($mixed), true), "\n";
echo var_export(array_key_first([]), true), "|", var_export(array_key_last([]), true), "\n";
