<?php
// expect=skip
$x = 1;
$f = function () use (&$x) {
    $x = 4;
};

$f();
echo $x;
