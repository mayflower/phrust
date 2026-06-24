<?php
// runtime-semantics: expect=pass
$a = 1;
$b =& $a;
unset($a);
$b = 2;
echo isset($a) ? "bad" : "unset", "|", $b;
