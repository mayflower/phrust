<?php
$a = [];
$a["a"] = 1;
$a["b"] = 2;
$a["a"] = 3;
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
