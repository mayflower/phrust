<?php
$x = 4;
$f = fn ($y) => $x + $y;
$x = 100;

echo $f(3), "\n";
