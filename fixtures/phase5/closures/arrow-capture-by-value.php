<?php
// expect=skip
$x = 2;
$f = fn () => $x;
$x = 8;
echo $f();
