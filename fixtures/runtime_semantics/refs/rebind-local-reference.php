<?php
// runtime-semantics: expect=pass
$a = 1;
$b = 2;
$c =& $a;
$c =& $b;
$c = 4;
echo $a, "|", $b, "|", $c;
