<?php

$x = 1;
$y = 2;
$closure = function (int $value) use ($x, &$y): int {
    return $value + $x + $y;
};
