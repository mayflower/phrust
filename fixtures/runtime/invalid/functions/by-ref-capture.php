<?php
$x = 1;
$f = function () use (&$x) {
    return $x;
};

$f();
