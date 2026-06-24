<?php
// runtime-fixture: kind=valid expected_stdout="3x\n"
$a = 1;
$a += 2;
$a .= "x";
echo $a, "\n";
