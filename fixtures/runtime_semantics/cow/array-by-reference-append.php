<?php
// runtime-semantics: expect=pass
$a = [1];
$b =& $a;
$b[] = 2;
echo $a[0], "|", $a[1], "|", $b[1];
