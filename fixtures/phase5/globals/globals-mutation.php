<?php
// phase5-runtime: category=globals expect=pass
$x = 1;
$GLOBALS["x"] = 7;
echo $x, "\n";
$y = 3;
echo $GLOBALS["y"], "\n";
