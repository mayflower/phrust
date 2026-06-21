<?php

namespace Exprs;

$array = ["a" => 1, "b" => [2, 3]];
[$first, $second] = $array["b"];
list("a" => $named) = $array;
$nested = [[$first], ["second" => $second]];
