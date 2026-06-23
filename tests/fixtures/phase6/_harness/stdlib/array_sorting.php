<?php
// phase6-diff: id=PHASE6_STDLIB_ARRAY_SORTING area=stdlib expect=pass
$a = [2 => "b", 0 => "a", 1 => "c"];
sort($a);
echo var_export($a, true), "\n";
$a = [2 => "b", 0 => "a", 1 => "c"];
rsort($a);
echo var_export($a, true), "\n";

$b = ["z" => 2, "a" => 1, "m" => 3];
asort($b);
echo var_export($b, true), "\n";
arsort($b);
echo var_export($b, true), "\n";
ksort($b);
echo var_export($b, true), "\n";
krsort($b);
echo var_export($b, true), "\n";

$c = [3, 1, 2];
usort($c, fn($left, $right) => $right <=> $left);
echo var_export($c, true), "\n";
$d = ["z" => 2, "a" => 1, "m" => 3];
uasort($d, fn($left, $right) => $right <=> $left);
echo var_export($d, true), "\n";
uksort($d, fn($left, $right) => strcmp($right, $left));
echo var_export($d, true), "\n";

$e = ["img10", "img2", "img1"];
natsort($e);
echo var_export($e, true), "\n";
$f = ["B2", "a10", "A1"];
natcasesort($f);
echo var_export($f, true), "\n";
