<?php
// phase5-runtime: expect=pass
$a = 1;
$b = 2;
$c =& $a;
$c =& $b;
$c = 4;
echo $a, "|", $b, "|", $c;
