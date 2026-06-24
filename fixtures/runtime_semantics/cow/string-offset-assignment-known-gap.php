<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_COW_STRING_OFFSET_WRITE
$a = "ab";
$b = $a;
$b[1] = "Z";
echo $a, "|", $b;
