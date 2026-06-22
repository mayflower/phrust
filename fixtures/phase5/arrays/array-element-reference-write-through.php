<?php
// phase5-runtime: expect=pass
$a = [];
$b = 1;
$a["x"] =& $b;
$a["x"] = 4;
echo $b, "|", $a["x"];
