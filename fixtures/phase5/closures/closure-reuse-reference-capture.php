<?php
// expect=skip
$x = 1;
$f = function () use (&$x) {
    $x++;
    return $x;
};

echo $f(), '|', $f(), '|', $x;
