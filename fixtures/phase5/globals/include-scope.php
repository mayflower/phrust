<?php
// phase5-runtime: category=globals expect=pass
$x = 1;
include (__DIR__ . "/_data/include-scope-helper.php");
echo $x, ":", $GLOBALS["from_include"], "\n";
