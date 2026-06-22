<?php
// expect=skip
$a = function () {
    static $x = 0;
    $x++;
    return $x;
};
$b = function () {
    static $x = 10;
    $x++;
    return $x;
};

echo $a(), '|', $a(), '|', $b(), '|', $b();
