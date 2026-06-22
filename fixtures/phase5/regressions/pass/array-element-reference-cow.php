<?php
// phase5-runtime: expect=pass regression_category=refs-cow reference_behavior=stdout:1|9 fix_prompt=48
$a = ["x" => 1];
$b = $a;
$r = 9;
$b["x"] =& $r;
echo $a["x"], "|", $b["x"];
