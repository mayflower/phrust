<?php
// runtime-semantics: category=arrays expect=pass php_ref_required=1
// Packed values-only storage surface: list creation, foreach keys,
// unset holes and key preservation, append after unset, mixed
// conversion, COW isolation, references, nesting, json, slice/count.
$list = [10, 20, 30, 40];
foreach ($list as $k => $v) {
    echo "$k=$v,";
}
echo "\n";

unset($list[1]);
foreach ($list as $k => $v) {
    echo "$k=$v,";
}
echo count($list), "\n";

$list[] = 99;
var_dump(array_keys($list));

$tail = [1, 2, 3];
unset($tail[2]);
$tail[] = 7;
var_dump(array_keys($tail));

$copy = $tail;
$copy[] = 8;
echo count($tail), "|", count($copy), "\n";

$nested = [[1, 2], [3, 4]];
$nested[1][] = 5;
echo json_encode($nested), "\n";
echo json_encode(["packed" => [7, 8, 9]]), "\n";

$ref = [1, 2, 3];
$alias =& $ref[0];
$alias = 100;
echo $ref[0], "|", implode(",", array_slice($ref, 1)), "\n";

$mix = [0 => "a", 1 => "b"];
$mix["name"] = "c";
$mix[5] = "d";
foreach ($mix as $k => $v) {
    echo "$k:$v,";
}
echo "\n";
echo array_sum([1, 2, 3, 4, 5]), "|", max([3, 9, 2]), "\n";
