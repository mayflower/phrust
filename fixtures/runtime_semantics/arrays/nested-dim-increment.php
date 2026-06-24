<?php
// runtime-semantics: expect=pass
$a = ["x" => ["y" => 1]];
$a["x"]["y"]++;
echo $a["x"]["y"];
