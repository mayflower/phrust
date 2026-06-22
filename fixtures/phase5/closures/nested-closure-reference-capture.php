<?php
// expect=skip
$x = 1;
$outer = function () use (&$x) {
    return function () use (&$x) {
        return $x;
    };
};

$inner = $outer();
$x = 6;
echo $inner();
