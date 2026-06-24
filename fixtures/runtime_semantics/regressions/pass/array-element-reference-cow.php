<?php
// runtime-semantics: expect=pass regression_category=refs-cow reference_behavior=stdout:1|9 regression_case=48
$a = ["x" => 1];
$b = $a;
$r = 9;
$b["x"] =& $r;
echo $a["x"], "|", $b["x"];
