<?php
// stdlib-diff: id=STDLIB_ARRAY_SLICE_MERGE area=stdlib expect=pass
$mixed = [2 => "two", "a" => 1, 4 => "four"];
echo var_export(array_slice($mixed, 1, 2), true), "\n";
echo var_export(array_slice($mixed, 1, 2, true), true), "\n";
echo var_export(array_reverse($mixed), true), "\n";
echo var_export(array_reverse($mixed, true), true), "\n";
echo var_export(array_pad([1], 3, 0), true), "\n";
echo var_export(array_pad([1], -3, 0), true), "\n";
echo var_export(array_chunk([1, 2, 3], 2), true), "\n";
echo var_export(array_flip(["a" => 1, "b" => "x"]), true), "\n";
$left = [0 => "x", "k" => 1];
$right = [7 => "y", "k" => 2];
echo var_export(array_merge($left, $right), true), "\n";
echo var_export(array_replace($left, $right), true), "\n";
echo var_export(array_merge_recursive(["k" => 1], ["k" => 2]), true), "\n";
