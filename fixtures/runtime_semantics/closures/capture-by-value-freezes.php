<?php
// expect=skip
$x = 1;
$f = function () use ($x) {
    return $x;
};

$x = 9;
echo $f();
