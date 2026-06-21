<?php
$a = ["x" => 0, "y" => 1];
echo isset($a["x"]), isset($a["z"]), "|";
echo empty($a["x"]), empty($a["z"]), empty($missing), "|";
unset($a["y"]);
echo isset($a["y"]), "\n";
