<?php
$a = 1;
$b =& $a;
$b = 2;
echo $a;
$a = 3;
echo $b;
echo "\n";
