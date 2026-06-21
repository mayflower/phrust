<?php
// phase4-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_GLOBALS_ALIAS_MATRIX
$x = 1;
$GLOBALS['x'] =& $x;
$x = 2;
echo $GLOBALS['x'];
echo "\n";
