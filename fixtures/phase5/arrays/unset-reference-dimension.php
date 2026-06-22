<?php
// phase5-runtime: expect=pass
$a = ["x" => 1, "y" => 2];
$r =& $a["x"];
unset($a["x"]);
$r = 7;
echo isset($a["x"]) ? "bad" : "unset", "|", $a["y"], "|", $r;
