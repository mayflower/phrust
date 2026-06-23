<?php
// phase6-diff: id=PHASE6_STDLIB_ARRAY_CALLBACKS area=stdlib expect=pass
function phase6_plus_one($value) { return $value + 1; }
class Phase6Scale { static function double($value) { return $value * 2; } }

$input = ["a" => 1, "b" => 2, "c" => 3];
echo var_export(array_map("phase6_plus_one", $input), true), "\n";
echo var_export(array_map(["Phase6Scale", "double"], [1, 2]), true), "\n";
echo var_export(array_map(fn($left, $right) => [$left, $right], ["x" => 1, "y" => 2], [3]), true), "\n";
echo var_export(array_filter($input, fn($value, $key) => $value > 1 && $key !== "c", 1), true), "\n";
echo array_reduce([1, 2, 3], fn($carry, $value) => $carry + $value, 0), "\n";
$walk = ["x" => 1, "y" => 2];
array_walk($walk, function($value, $key, $suffix) { echo $key, ":", $value, $suffix; }, ";");
echo "\n";
echo array_any($input, fn($value, $key) => $key === "b") ? "T" : "F";
echo array_all($input, fn($value, $key) => $value > 0) ? "T" : "F";
echo "|", array_find($input, fn($value, $key) => $value === 2);
echo "|", array_find_key($input, fn($value, $key) => $value === 3), "\n";
