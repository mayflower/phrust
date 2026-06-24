<?php
// expect=skip
$x = 3;
$f = static function () use ($x) {
    return $x;
};

echo $f();
