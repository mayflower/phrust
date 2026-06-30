<?php
// runtime-semantics: category=functions expect=pass
function binder_matrix($first, $second = "B", &$ref = null, ...$rest) {
    $ref = $first . $second;
    echo $first, "|", $second, "|", $ref, "|", $rest["tail"], "|", count(func_get_args()), "\n";
}

$value = "x";
binder_matrix(second: "S", ref: $value, first: "F", tail: "T");
echo $value, "\n";

function binder_extra($first) {
    echo implode(",", func_get_args()), "\n";
}

binder_extra("A", "B", "C");
