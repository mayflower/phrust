<?php
// phase5-runtime: category=globals expect=pass
$x = 10;
$fn = function () {
    global $x;
    $x = $x + 1;
};
$fn();
echo $x, "\n";
