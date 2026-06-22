<?php
$a = 1;
$b =& $a;
$c = 1;
echo ($a == $b) ? "1" : "0";
echo "|";
echo ($a === $b) ? "1" : "0";
echo "|";
echo ($b === $c) ? "1" : "0";
echo "\n";
