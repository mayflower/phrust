<?php
// phase5-runtime: expect=pass
$a = ["x" => 1];
$b = $a;
$r = 9;
$b["x"] =& $r;
echo $a["x"], "|", $b["x"];
