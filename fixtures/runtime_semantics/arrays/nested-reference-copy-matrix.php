<?php
// runtime-semantics: expect=pass
$a = ["x" => ["y" => 1]];
$b = $a;
$b["x"]["y"] = 2;
echo $a["x"]["y"], "|", $b["x"]["y"], "\n";

$ref = 10;
$a = ["x" => ["y" => 1]];
$a["x"]["y"] =& $ref;
$b = $a;
$b["x"]["z"] = 3;
$ref = 11;
echo $a["x"]["y"], "|", $b["x"]["y"], "|";
echo isset($a["x"]["z"]) ? "bad" : "no-z";
echo "|", $b["x"]["z"], "\n";
