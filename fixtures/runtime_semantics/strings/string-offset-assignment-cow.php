<?php
// runtime-semantics: expect=pass
$a = "ab";
$b = $a;
$b[1] = "Z";
echo $a, "|", $b;
