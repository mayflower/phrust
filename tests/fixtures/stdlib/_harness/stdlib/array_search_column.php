<?php
// stdlib-diff: id=STDLIB_ARRAY_SEARCH_COLUMN area=stdlib expect=pass
$haystack = [0, "7", 7];
echo in_array(7, $haystack) ? "1" : "0";
echo in_array(7, $haystack, true) ? "1" : "0";
echo in_array("7", $haystack, true) ? "1" : "0";
echo "\n";
echo var_export(array_search("7", $haystack), true), "|", var_export(array_search("7", $haystack, true), true), "|", var_export(array_search("missing", $haystack), true), "\n";
$row1 = ["id" => 2, "name" => "Ada"];
$row2 = ["id" => 3, "name" => "Grace"];
$rows = [$row1, $row2];
echo var_export(array_column($rows, "name", "id"), true), "\n";
echo var_export(array_keys($haystack, 7), true), "\n";
echo var_export(array_keys($haystack, 7, true), true), "\n";
