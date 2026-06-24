<?php
// expect=skip
$f = function () {
    static $x = 0;
    $x++;
    return $x;
};

echo $f(), '|', $f();
