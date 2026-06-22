<?php
$x = 1;
$f = function () use (&$x) {
    return $x;
};

$x = 3;
echo $f();
