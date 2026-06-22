<?php
// phase5-runtime: expect=pass
$a = 1;
$b =& $a;
$c =& $b;
$c = 3;
echo $a, "|", $b, "|", $c;
