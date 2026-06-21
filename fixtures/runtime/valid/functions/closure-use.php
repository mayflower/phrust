<?php
$x = 2;
$f = function ($y) use ($x) {
    return $x + $y;
};
$x = 100;

echo $f(3), "\n";
