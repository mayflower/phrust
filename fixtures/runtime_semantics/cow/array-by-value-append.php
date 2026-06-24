<?php
// runtime-semantics: expect=pass
$a = [1];
$b = $a;
$b[] = 2;
echo $a[0], "|", isset($a[1]) ? "bad" : "ok", "|", $b[0], "|", $b[1];
